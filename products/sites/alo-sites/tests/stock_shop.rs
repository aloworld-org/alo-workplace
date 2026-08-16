//! In-process integration tests of the public stock shop (`/shop`,
//! `/shop/{item}`, `/shop/order/{order}`, and the shared `POST /_alo/pay`):
//! real fixtures through the real store into the compose Postgres — a
//! stocked product with real ledger goods — real requests through the real
//! router, the deterministic fixture payment provider standing in for the
//! hosted page.
//!
//! The mandatory isolation case is a foreign Host resolving another site's
//! item and order ids to one clean 404 — offer, checkout and return page
//! alike. The rest pin the wire contract: the listing priced by the Billing
//! seam and counted by the Inventory ledger at every read (never cached),
//! the whole arc from the buy form through the fixture provider to the paid
//! confirmation — with the shelf actually dropping — the honeypot's silent
//! no-op, verbatim refusals, the rate limit, and the unconfigured
//! deployment telling the truth instead of rendering a checkout.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::inv_locations::{Location, LocationKind, LocationSeed};
use alo_store::inv_moves::{MoveReason, NewMove};
use alo_store::{
    AccountStore, BillingProductId, BlobStore, FixtureSitePayments, InvLocationId, NewProduct,
    SiteId, SitePaymentProvider, SitePaymentStatus, SitePublicStore, SiteShopItemId, Store,
};
use serde_json::json;
use time::OffsetDateTime;

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

/// The flat delivery price every test shop charges: € 5.95.
const SHIPPING: i64 = 595;

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5433/alo".to_owned())
}

fn seed_names() -> LocationSeed {
    LocationSeed {
        stock: "Hoofdmagazijn".to_owned(),
        supplier: "Leveranciers".to_owned(),
        customer: "Klanten".to_owned(),
        adjustment: "Correcties".to_owned(),
        production: "Productie".to_owned(),
    }
}

/// One tenant's live site selling a stocked book, and the app serving it
/// through the fixture payment provider.
struct Shop {
    account: AccountStore,
    state: Arc<AppState>,
    provider: Arc<FixtureSitePayments>,
    product: BillingProductId,
    item: SiteShopItemId,
    main: InvLocationId,
    host: String,
    pool: sqlx::PgPool,
}

async fn shop(tag: &str, shelf: i64) -> Shop {
    shop_with(tag, shelf, true).await
}

async fn shop_with(tag: &str, shelf: i64, configured: bool) -> Shop {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");

    let tenant = store.create_tenant(&format!("shop-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("owner@{tag}.shop.test"))
        .await
        .unwrap();
    let account = store.for_account(tenant, user);

    let seeded = account
        .inv_locations_or_seed(&seed_names(), false)
        .await
        .unwrap();
    let of = |kind: LocationKind| -> InvLocationId {
        seeded
            .iter()
            .find(|l: &&Location| l.kind == kind)
            .unwrap_or_else(|| panic!("the seed must write a {kind:?} location"))
            .id
            .clone()
    };
    let main = of(LocationKind::Stock);
    let supplier = of(LocationKind::Supplier);
    let product = account
        .create_billing_product(&NewProduct {
            name: "Field guide".to_owned(),
            unit: "piece".to_owned(),
            unit_price_cents: 2_400,
            vat_rate_bp: 600,
            stocked: true,
            purchase_price_cents: 900,
            ..Default::default()
        })
        .await
        .unwrap();
    if shelf > 0 {
        account
            .record_move(&NewMove {
                product_id: product.clone(),
                from_location_id: supplier,
                to_location_id: main.clone(),
                qty_milli: shelf * 1_000,
                reason: MoveReason::Purchase,
                reason_code: None,
                note: String::new(),
                reference: None,
                occurred_at: None,
            })
            .await
            .unwrap();
    }

    let subdomain = unique(tag);
    let site = account.create_site("Shop", &subdomain).await.unwrap();
    let home = account
        .create_site_page(&site, "Home", "", true)
        .await
        .unwrap();
    account
        .set_page_sections(
            &site,
            &home,
            json!({
                "schema_version": 1,
                "sections": [{
                    "type": "shop",
                    "heading": "The roastery shop",
                }]
            }),
        )
        .await
        .unwrap();
    account.publish_site(&site).await.unwrap();
    let item = account
        .add_site_shop_item(&site, &product, OffsetDateTime::now_utc())
        .await
        .unwrap();
    account
        .set_site_shop_shipping_cents(&site, SHIPPING)
        .await
        .unwrap();

    let provider = Arc::new(FixtureSitePayments::new());
    let payments: Option<Arc<dyn SitePaymentProvider>> =
        configured.then(|| Arc::clone(&provider) as Arc<dyn SitePaymentProvider>);
    let state = AppState::with_payments(
        SitePublicStore::new(pool.clone(), blobs),
        APEX.to_owned(),
        b"stock-shop-tests-analytics-secret!!!",
        payments,
    );
    Shop {
        account,
        state,
        provider,
        product,
        item,
        main,
        host: format!("{subdomain}.{APEX}"),
        pool,
    }
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

/// A complete, valid buy form for `units` of the fixture product.
fn buy_body(units: i64) -> String {
    format!(
        "quantity={units}&name=Maud+Adams&email=maud%40example.org\
         &address=Keizersgracht+1&city=Amsterdam&postcode=1015+CS&country=nl&website="
    )
}

async fn get(shop: &Shop, host: &str, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(&shop.state)).oneshot(request).await.unwrap()
}

async fn post(shop: &Shop, host: &str, path: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, host)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(&shop.state)).oneshot(request).await.unwrap()
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn header_value(response: &Response, name: header::HeaderName) -> String {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap().to_owned())
        .unwrap_or_default()
}

impl Shop {
    /// Milli-units of the product on the main shelf, straight from the
    /// ledger.
    async fn on_shelf(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(qty_milli), 0)::bigint FROM inv_stock \
              WHERE tenant_id = $1 AND product_id = $2 AND location_id = $3",
        )
        .bind(self.account.tenant().as_str())
        .bind(self.product.as_str())
        .bind(self.main.as_str())
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// Stock-sale holds this tenant has, of any state.
    async fn holds(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM inv_stock_sale_holds WHERE tenant_id = $1")
            .bind(self.account.tenant().as_str())
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn the_shop_sells_a_book_from_listing_to_shipped_goods() {
    let s = shop("arc", 10).await;

    // The published page renders the door to the live shop.
    let home = body_string(get(&s, &s.host, "/").await).await;
    assert!(home.contains("href=\"/shop\""), "{home}");
    assert!(home.contains("The roastery shop"), "{home}");

    // The listing: live, priced by the seam, counted by the ledger, never
    // cached, delivery stated.
    let listing = get(&s, &s.host, "/shop").await;
    assert_eq!(listing.status(), StatusCode::OK);
    assert_eq!(
        header_value(&listing, header::CACHE_CONTROL),
        "no-store",
        "a price and a shelf count are live state"
    );
    let page = body_string(listing).await;
    // The design note's page budget holds for the shop's pages too (S3.06d1).
    assert!(
        page.len() < 100 * 1024,
        "listing is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(page.contains("Field guide"), "{page}");
    assert!(
        page.contains("€\u{a0}24.00") || page.contains("€\u{a0}24,00"),
        "{page}"
    );
    assert!(
        page.contains("€\u{a0}5.95") || page.contains("€\u{a0}5,95"),
        "the delivery price is stated: {page}"
    );
    assert!(
        page.contains(&format!("/shop/{}", s.item.as_str())),
        "the listing links the offer: {page}"
    );

    // The offer page carries the buy form, capped by what is on the shelf.
    let offer = get(&s, &s.host, &format!("/shop/{}", s.item.as_str())).await;
    assert_eq!(offer.status(), StatusCode::OK);
    let page = body_string(offer).await;
    assert!(
        page.len() < 100 * 1024,
        "offer page is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(page.contains("name=\"quantity\""), "{page}");
    assert!(page.contains("max=\"10\""), "{page}");
    assert!(page.contains("name=\"address\""), "{page}");
    assert!(page.contains("name=\"country\""), "{page}");
    assert!(
        page.contains("name=\"website\""),
        "the honeypot rides: {page}"
    );

    // The purchase: 303 to the provider's hosted page.
    let bought = post(
        &s,
        &s.host,
        &format!("/shop/{}", s.item.as_str()),
        "1.1.1.1",
        &buy_body(2),
    )
    .await;
    assert_eq!(bought.status(), StatusCode::SEE_OTHER);
    let checkout_url = header_value(&bought, header::LOCATION);
    let payment_id = checkout_url
        .rsplit('/')
        .next()
        .expect("the fixture URL names its payment")
        .to_owned();
    let order_id = payment_id
        .strip_prefix("fixpay-")
        .expect("the fixture id is fixpay-<order>")
        .to_owned();

    // The return page before paying: the order waits, the link stands, and
    // the shelf has not moved (a hold is not a movement).
    let waiting = get(&s, &s.host, &format!("/shop/order/{order_id}")).await;
    assert_eq!(waiting.status(), StatusCode::OK);
    assert_eq!(header_value(&waiting, header::CACHE_CONTROL), "no-store");
    let page = body_string(waiting).await;
    assert!(
        page.len() < 100 * 1024,
        "return page is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(
        page.contains("Your payment has not finished yet."),
        "{page}"
    );
    assert!(page.contains(&checkout_url), "{page}");
    assert!(
        !page.contains("Maud Adams") && !page.contains("Keizersgracht"),
        "a return URL proves less than being the buyer: {page}"
    );
    assert_eq!(s.on_shelf().await, 10_000);

    // The buyer pays on the hosted page; the return page FETCHES the truth
    // (no webhook needed), settles, and the goods leave the shelf.
    s.provider
        .mark(&payment_id, SitePaymentStatus::Paid)
        .unwrap();
    let paid = get(&s, &s.host, &format!("/shop/order/{order_id}")).await;
    let page = body_string(paid).await;
    assert!(
        page.contains("your order is confirmed"),
        "the paid page says so: {page}"
    );
    assert_eq!(s.on_shelf().await, 8_000, "paid means the goods moved");

    // The webhook replayed after the fact is one sale, quietly.
    let rung = post(
        &s,
        "webhooks.have.no.host",
        "/_alo/pay",
        "2.2.2.2",
        &format!("id={payment_id}"),
    )
    .await;
    assert_eq!(rung.status(), StatusCode::OK);
    assert_eq!(s.on_shelf().await, 8_000, "a retried webhook moves nothing");
}

#[tokio::test]
async fn the_walls_hold_on_every_host() {
    let a = shop("wall-a", 10).await;
    let b = shop("wall-b", 10).await;

    // An order of A's, to test the return-page wall with.
    let bought = post(
        &a,
        &a.host,
        &format!("/shop/{}", a.item.as_str()),
        "3.3.3.3",
        &buy_body(1),
    )
    .await;
    assert_eq!(bought.status(), StatusCode::SEE_OTHER);
    let order_id = header_value(&bought, header::LOCATION)
        .rsplit("fixpay-")
        .next()
        .unwrap()
        .to_owned();

    // B's host — a different tenant; B's own app instance knows both sites
    // through the shared database — resolves A's ids to one 404.
    let foreign_offer = get(&b, &b.host, &format!("/shop/{}", a.item.as_str())).await;
    assert_eq!(foreign_offer.status(), StatusCode::NOT_FOUND);
    let foreign_buy = post(
        &b,
        &b.host,
        &format!("/shop/{}", a.item.as_str()),
        "3.3.3.4",
        &buy_body(1),
    )
    .await;
    assert_eq!(foreign_buy.status(), StatusCode::NOT_FOUND);
    let foreign_order = get(&b, &b.host, &format!("/shop/order/{order_id}")).await;
    assert_eq!(foreign_order.status(), StatusCode::NOT_FOUND);

    // An unknown host is the same nothing.
    let nowhere = get(&a, &format!("ghost.{APEX}"), "/shop").await;
    assert_eq!(nowhere.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_honeypot_buys_nothing_and_refusals_speak_verbatim() {
    let s = shop("gates", 2).await;
    let path = format!("/shop/{}", s.item.as_str());

    // The field no human sees is filled: answer like a hop back to the
    // offer, reserve nothing.
    let bot = post(
        &s,
        &s.host,
        &path,
        "4.4.4.1",
        "quantity=2&name=Bot&email=bot%40example.test&address=x&city=y&postcode=z\
         &country=nl&website=https%3A%2F%2Fspam",
    )
    .await;
    assert_eq!(bot.status(), StatusCode::SEE_OTHER);
    assert_eq!(header_value(&bot, header::LOCATION), path);
    assert_eq!(s.holds().await, 0, "a bot costs no hold");

    // A typo is refused before any goods are touched, in the store's words.
    let typo = post(
        &s,
        &s.host,
        &path,
        "4.4.4.2",
        "quantity=2&name=Maud+Adams&email=maud%40example.org\
         &address=Keizersgracht+1&city=Amsterdam&postcode=1015+CS&country=Netherlands&website=",
    )
    .await;
    assert_eq!(typo.status(), StatusCode::BAD_REQUEST);
    assert!(
        body_string(typo)
            .await
            .contains("country must be a two-letter code")
    );
    assert_eq!(s.holds().await, 0, "a typo costs no hold");

    // Both units are still there: the real buyer gets them...
    let real = post(&s, &s.host, &path, "4.4.4.3", &buy_body(2)).await;
    assert_eq!(real.status(), StatusCode::SEE_OTHER);

    // ...and the next visitor is told so in the goods' own words.
    let late = post(&s, &s.host, &path, "4.4.4.4", &buy_body(1)).await;
    assert_eq!(late.status(), StatusCode::CONFLICT);
    assert!(body_string(late).await.contains("sold out"));

    // The sold-out offer page says so and offers no form.
    let page = body_string(get(&s, &s.host, &path).await).await;
    assert!(page.contains("Sold out"), "{page}");
    assert!(!page.contains("<form"), "{page}");
}

#[tokio::test]
async fn the_checkout_door_is_rate_limited() {
    let s = shop("rate", 10).await;
    let path = format!("/shop/{}", s.item.as_str());
    // Honeypot posts spend the budget without spending goods.
    let body = "quantity=1&name=Bot&email=bot%40example.test&address=x&city=y&postcode=z\
                &country=nl&website=x";
    for _ in 0..alo_sites::serve::rate::MAX_PER_WINDOW {
        let allowed = post(&s, &s.host, &path, "5.5.5.5", body).await;
        assert_eq!(allowed.status(), StatusCode::SEE_OTHER);
    }
    let limited = post(&s, &s.host, &path, "5.5.5.5", body).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(!header_value(&limited, header::RETRY_AFTER).is_empty());
    // Another client is not the one being limited.
    let other = post(&s, &s.host, &path, "5.5.5.6", body).await;
    assert_eq!(other.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn an_unconfigured_shop_tells_the_truth() {
    let s = shop_with("bare", 5, false).await;
    let path = format!("/shop/{}", s.item.as_str());

    // The listing and the offer still show what is on: honesty, not a hole.
    let page = body_string(get(&s, &s.host, "/shop").await).await;
    assert!(page.contains("Field guide"), "{page}");
    let page = body_string(get(&s, &s.host, &path).await).await;
    assert!(
        page.contains("Online sales are not set up on this site yet."),
        "{page}"
    );
    assert!(
        !page.contains("<form"),
        "no checkout that can only fail: {page}"
    );

    // A POST anyway (an old tab, a script) is the same honest sentence, and
    // no goods are held for it.
    let posted = post(&s, &s.host, &path, "6.6.6.6", &buy_body(1)).await;
    assert_eq!(posted.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(s.holds().await, 0);
}
