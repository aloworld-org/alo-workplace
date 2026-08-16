//! In-process integration tests of the public ticket shop (`/tix`,
//! `/tix/{event}`, `/tix/order/{order}`, `POST /_alo/pay`): real fixtures
//! through the real store into the compose Postgres, real requests through
//! the real router, the deterministic fixture payment provider standing in
//! for the hosted page.
//!
//! The mandatory isolation case is a foreign Host resolving another site's
//! event and order ids to one clean 404 — offer, checkout and return page
//! alike — plus the webhook door answering an unknown payment id exactly like
//! success. The rest pin the wire contract: the listing priced from the seam
//! and never cached, the whole arc from the buy form through the fixture
//! provider to the ticket link, the honeypot's silent no-op, the verbatim
//! refusals, the rate limit, and the unconfigured deployment telling the
//! truth instead of rendering a checkout.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{
    AccountStore, BillingProductId, BlobStore, FixtureSitePayments, SiteId, SitePaymentProvider,
    SitePaymentStatus, SitePublicStore, SiteTicketEventId, Store,
};
use serde_json::json;
use time::{Duration, OffsetDateTime};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5433/alo".to_owned())
}

/// One tenant's live site selling seats to one event, and the app serving it
/// through the fixture payment provider.
struct Venue {
    account: AccountStore,
    store: Store,
    state: Arc<AppState>,
    provider: Arc<FixtureSitePayments>,
    site: SiteId,
    product: BillingProductId,
    event: SiteTicketEventId,
    host: String,
}

async fn venue(tag: &str, capacity: i32) -> Venue {
    venue_with(tag, capacity, true).await
}

async fn venue_with(tag: &str, capacity: i32, configured: bool) -> Venue {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");

    let tenant = store.create_tenant(&format!("tix-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("owner@{tag}.tickets.test"))
        .await
        .unwrap();
    let account = store.for_account(tenant, user);

    let subdomain = unique(tag);
    let site = account.create_site("Venue", &subdomain).await.unwrap();
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
                    "type": "tickets",
                    "heading": "Evenings at the press",
                }]
            }),
        )
        .await
        .unwrap();
    account.publish_site(&site).await.unwrap();

    let product = account
        .create_billing_product(&alo_store::NewProduct {
            name: "Letterpress workshop".to_owned(),
            unit: "seat".to_owned(),
            unit_price_cents: 8_500,
            vat_rate_bp: 2100,
            ..Default::default()
        })
        .await
        .unwrap();
    let event = account
        .create_site_ticket_event(
            &site,
            &product,
            OffsetDateTime::now_utc() + Duration::days(7),
            capacity,
        )
        .await
        .unwrap();

    let provider = Arc::new(FixtureSitePayments::new());
    let payments: Option<Arc<dyn SitePaymentProvider>> =
        configured.then(|| Arc::clone(&provider) as Arc<dyn SitePaymentProvider>);
    let state = AppState::with_payments(
        SitePublicStore::new(pool, blobs),
        APEX.to_owned(),
        b"ticket-shop-tests-analytics-secret!!",
        payments,
    );
    Venue {
        account,
        store,
        state,
        provider,
        site,
        product,
        event,
        host: format!("{subdomain}.{APEX}"),
    }
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

async fn get(venue: &Venue, host: &str, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(&venue.state))
        .oneshot(request)
        .await
        .unwrap()
}

async fn post(venue: &Venue, host: &str, path: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, host)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(&venue.state))
        .oneshot(request)
        .await
        .unwrap()
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

#[tokio::test]
async fn the_shop_sells_a_seat_from_listing_to_ticket() {
    let v = venue("arc", 10).await;

    // The listing: live, priced from the seam, never cached.
    let listing = get(&v, &v.host, "/tix").await;
    assert_eq!(listing.status(), StatusCode::OK);
    assert_eq!(
        header_value(&listing, header::CACHE_CONTROL),
        "no-store",
        "a price is live state"
    );
    let page = body_string(listing).await;
    // The design note's page budget holds for the shop's pages too (S3.06d1).
    assert!(
        page.len() < 100 * 1024,
        "listing is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(page.contains("Letterpress workshop"), "{page}");
    assert!(
        page.contains("€\u{a0}85.00") || page.contains("€\u{a0}85,00"),
        "{page}"
    );
    assert!(
        page.contains(&format!("/tix/{}", v.event.as_str())),
        "the listing links the offer: {page}"
    );

    // The offer page carries the buy form, capped by what is left.
    let offer = get(&v, &v.host, &format!("/tix/{}", v.event.as_str())).await;
    assert_eq!(offer.status(), StatusCode::OK);
    let page = body_string(offer).await;
    assert!(
        page.len() < 100 * 1024,
        "offer page is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(page.contains("name=\"quantity\""), "{page}");
    assert!(page.contains("max=\"10\""), "{page}");
    assert!(
        page.contains("name=\"website\""),
        "the honeypot rides: {page}"
    );

    // The purchase: 303 to the provider's hosted page.
    let bought = post(
        &v,
        &v.host,
        &format!("/tix/{}", v.event.as_str()),
        "1.1.1.1",
        "quantity=2&name=Maud+Adams&email=maud%40example.org&website=",
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

    // The return page before paying: the order waits, the link stands.
    let waiting = get(&v, &v.host, &format!("/tix/order/{order_id}")).await;
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

    // The buyer pays on the hosted page; the return page FETCHES the truth
    // (no webhook needed) and settles.
    v.provider
        .mark(&payment_id, SitePaymentStatus::Paid)
        .unwrap();
    let paid = get(&v, &v.host, &format!("/tix/order/{order_id}")).await;
    let page = body_string(paid).await;
    assert!(
        page.contains("your ticket is on its way"),
        "paid, ticket not yet minted: {page}"
    );

    // The webhook replayed after the fact is one sale, quietly.
    let rung = post(
        &v,
        "webhooks.have.no.host",
        "/_alo/pay",
        "2.2.2.2",
        &format!("id={payment_id}"),
    )
    .await;
    assert_eq!(rung.status(), StatusCode::OK);

    // Fulfilment mints the ticket; the return page then links it.
    v.store.claim_ticket_fulfilments(500).await.unwrap();
    let done = get(&v, &v.host, &format!("/tix/order/{order_id}")).await;
    let page = body_string(done).await;
    let ticket_path = page
        .split("href=\"/t/")
        .nth(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .expect("the return page links the ticket");
    let ticket = get(&v, &v.host, &format!("/t/{ticket_path}")).await;
    assert_eq!(ticket.status(), StatusCode::OK);
    let page = body_string(ticket).await;
    assert!(
        page.len() < 100 * 1024,
        "ticket page is {} bytes, budget is 100KB",
        page.len()
    );
    assert!(page.contains("Maud Adams"), "{page}");
}

#[tokio::test]
async fn the_walls_hold_on_every_host() {
    let a = venue("wall-a", 10).await;
    let b = venue("wall-b", 10).await;

    // An order of A's, to test the return-page wall with.
    let bought = post(
        &a,
        &a.host,
        &format!("/tix/{}", a.event.as_str()),
        "3.3.3.3",
        "quantity=1&name=Maud+Adams&email=maud%40example.org&website=",
    )
    .await;
    assert_eq!(bought.status(), StatusCode::SEE_OTHER);
    let order_id = header_value(&bought, header::LOCATION)
        .rsplit("fixpay-")
        .next()
        .unwrap()
        .to_owned();

    // B's host — a different tenant, note: B's own app instance knows both
    // sites through the shared database — resolves A's ids to one 404.
    let foreign_offer = get(&b, &b.host, &format!("/tix/{}", a.event.as_str())).await;
    assert_eq!(foreign_offer.status(), StatusCode::NOT_FOUND);
    let foreign_buy = post(
        &b,
        &b.host,
        &format!("/tix/{}", a.event.as_str()),
        "3.3.3.4",
        "quantity=1&name=Ada+Lovelace&email=ada%40example.test&website=",
    )
    .await;
    assert_eq!(foreign_buy.status(), StatusCode::NOT_FOUND);
    let foreign_order = get(&b, &b.host, &format!("/tix/order/{order_id}")).await;
    assert_eq!(foreign_order.status(), StatusCode::NOT_FOUND);

    // An unknown host is the same nothing.
    let nowhere = get(&a, &format!("ghost.{APEX}"), "/tix").await;
    assert_eq!(nowhere.status(), StatusCode::NOT_FOUND);

    // The webhook door answers an id nobody holds exactly like success.
    let probe = post(
        &a,
        "any.host.at.all",
        "/_alo/pay",
        "3.3.3.5",
        "id=fixpay-never-was",
    )
    .await;
    assert_eq!(probe.status(), StatusCode::OK);
    assert!(body_string(probe).await.is_empty());
    let malformed = post(&a, "any.host.at.all", "/_alo/pay", "3.3.3.6", "ring=ring").await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_honeypot_buys_nothing_and_refusals_speak_verbatim() {
    let v = venue("gates", 2).await;
    let path = format!("/tix/{}", v.event.as_str());

    // The field no human sees is filled: answer like a hop back to the offer,
    // reserve nothing.
    let bot = post(
        &v,
        &v.host,
        &path,
        "4.4.4.1",
        "quantity=2&name=Bot&email=bot%40example.test&website=https%3A%2F%2Fspam",
    )
    .await;
    assert_eq!(bot.status(), StatusCode::SEE_OTHER);
    assert_eq!(header_value(&bot, header::LOCATION), path);

    // A typo is refused before any seat is touched.
    let typo = post(
        &v,
        &v.host,
        &path,
        "4.4.4.2",
        "quantity=2&name=Maud+Adams&email=not-an-address&website=",
    )
    .await;
    assert_eq!(typo.status(), StatusCode::BAD_REQUEST);

    // Both seats are still there: the real buyer gets them...
    let real = post(
        &v,
        &v.host,
        &path,
        "4.4.4.3",
        "quantity=2&name=Maud+Adams&email=maud%40example.org&website=",
    )
    .await;
    assert_eq!(real.status(), StatusCode::SEE_OTHER);

    // ...and the next visitor is told so in the seats' own words.
    let late = post(
        &v,
        &v.host,
        &path,
        "4.4.4.4",
        "quantity=1&name=Ada+Lovelace&email=ada%40example.test&website=",
    )
    .await;
    assert_eq!(late.status(), StatusCode::CONFLICT);
    assert!(body_string(late).await.contains("sold out"));

    // The sold-out offer page says so and offers no form.
    let page = body_string(get(&v, &v.host, &path).await).await;
    assert!(page.contains("Sold out"), "{page}");
    assert!(!page.contains("<form"), "{page}");
}

#[tokio::test]
async fn the_checkout_door_is_rate_limited() {
    let v = venue("rate", 10).await;
    let path = format!("/tix/{}", v.event.as_str());
    // Honeypot posts spend the budget without spending seats.
    let body = "quantity=1&name=Bot&email=bot%40example.test&website=x";
    for _ in 0..alo_sites::serve::rate::MAX_PER_WINDOW {
        let allowed = post(&v, &v.host, &path, "5.5.5.5", body).await;
        assert_eq!(allowed.status(), StatusCode::SEE_OTHER);
    }
    let limited = post(&v, &v.host, &path, "5.5.5.5", body).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(!header_value(&limited, header::RETRY_AFTER).is_empty());
    // Another client is not the one being limited.
    let other = post(&v, &v.host, &path, "5.5.5.6", body).await;
    assert_eq!(other.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn an_unconfigured_shop_tells_the_truth() {
    let v = venue_with("bare", 5, false).await;
    let path = format!("/tix/{}", v.event.as_str());

    // The listing and the offer still show what is on: honesty, not a hole.
    let page = body_string(get(&v, &v.host, "/tix").await).await;
    assert!(page.contains("Letterpress workshop"), "{page}");
    let page = body_string(get(&v, &v.host, &path).await).await;
    assert!(
        page.contains("Online ticket sales are not set up on this site yet."),
        "{page}"
    );
    assert!(
        !page.contains("<form"),
        "no checkout that can only fail: {page}"
    );

    // A POST anyway (an old tab, a script) is the same honest sentence, and
    // no seat is held for it.
    let posted = post(
        &v,
        &v.host,
        &path,
        "6.6.6.6",
        "quantity=1&name=Maud+Adams&email=maud%40example.org&website=",
    )
    .await;
    assert_eq!(posted.status(), StatusCode::SERVICE_UNAVAILABLE);
    let held = v
        .account
        .ticket_availability(&v.site, &v.event, OffsetDateTime::now_utc())
        .await
        .unwrap();
    assert_eq!(held.held, 0);

    // The webhook rings into an unconfigured installation and learns nothing.
    let rung = post(&v, "any.host", "/_alo/pay", "6.6.6.7", "id=fixpay-x").await;
    assert_eq!(rung.status(), StatusCode::OK);

    // The published page still renders the section with its link.
    let home = body_string(get(&v, &v.host, "/").await).await;
    assert!(home.contains("href=\"/tix\""), "{home}");
    assert!(home.contains("Evenings at the press"), "{home}");
    let _ = v.product;
}
