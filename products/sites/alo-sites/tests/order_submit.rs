//! In-process integration tests of `POST /o/:catalog_id`: real fixtures
//! through the real store into the compose Postgres, real requests through the
//! real router.
//!
//! The mandatory isolation case is the foreign/unknown/unpublished catalog id
//! yielding one clean 404, plus proof that an accepted order lands only in the
//! owning tenant. The rest pin the wire contract: the honeypot's silent drop,
//! 400 on a malformed body or a refused order, 413 on an oversized body, 429
//! with `Retry-After` under the per-client rate limit — and that prices come
//! from the publish rather than from anything the client sent.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{
    AccountStore, BlobStore, SiteCatalogAvailability, SiteCatalogId, SiteCatalogInput,
    SiteCatalogItemInput, SiteId, SitePublicStore, Store,
};
use serde_json::json;

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5433/alo".to_owned())
}

/// A migrated store plus the service state sharing the same Postgres.
async fn harness() -> (Store, Arc<AppState>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");
    let state = AppState::new(
        SitePublicStore::new(pool, blobs),
        APEX.to_owned(),
        b"order-submit-tests-analytics-secret",
    );
    (store, state)
}

async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@orders.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

fn item<'a>(
    name: &'a str,
    slug: &'a str,
    price: Option<i64>,
    availability: SiteCatalogAvailability,
) -> SiteCatalogItemInput<'a> {
    SiteCatalogItemInput {
        category: None,
        name,
        slug,
        description: None,
        price_cents: price,
        price_note: None,
        image: None,
        availability,
        position: 0,
    }
}

/// A live site with one catalog on its home page. `orders` decides whether the
/// publish offers ordering at all.
async fn live_site_with_catalog(
    acc: &AccountStore,
    tag: &str,
    orders: bool,
) -> (SiteId, SiteCatalogId) {
    let site = acc.create_site("Bakery", &unique(tag)).await.unwrap();
    let home = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    let catalog = acc
        .create_site_catalog(
            &site,
            &SiteCatalogInput {
                name: "Saturday bake",
                currency: "EUR",
                orders_enabled: orders,
            },
        )
        .await
        .unwrap();
    acc.create_site_catalog_item(
        &site,
        &catalog,
        &item(
            "Sourdough",
            "sourdough",
            Some(450),
            SiteCatalogAvailability::Available,
        ),
    )
    .await
    .unwrap();
    acc.create_site_catalog_item(
        &site,
        &catalog,
        &item(
            "Focaccia",
            "focaccia",
            Some(600),
            SiteCatalogAvailability::SoldOut,
        ),
    )
    .await
    .unwrap();
    acc.set_page_sections(
        &site,
        &home,
        json!({
            "schema_version": 1,
            "sections": [{"type": "catalog", "catalog_id": catalog.as_str()}]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, catalog)
}

/// One urlencoded order POST through the real router. `client` becomes the
/// `X-Forwarded-For` value — how tests model distinct senders.
async fn post_order(state: &Arc<AppState>, catalog: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/o/{catalog}"))
        .header(header::HOST, format!("whatever.{APEX}"))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn accepts_an_order_into_the_owning_tenant_only_and_prices_it_from_the_publish() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-accept").await;
    let outsider = fresh_account(&store, "order-accept-outsider").await;
    let (site, catalog) = live_site_with_catalog(&owner, "order-accept", true).await;

    let response = post_order(
        &state,
        catalog.as_str(),
        "203.0.113.11",
        "qty-sourdough=2&name=Ada+Lovelace&email=ada%40example.test&phone=%2B32+2+555+01\
         &note=leave+at+the+door&website=",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page = body_string(response).await;
    assert!(page.contains("Order received"), "success page, got: {page}");

    let orders = owner.site_orders(&site).await.unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].customer_name, "Ada Lovelace");
    assert_eq!(orders[0].customer_email, "ada@example.test");
    assert_eq!(orders[0].customer_phone.as_deref(), Some("+32 2 555 01"));
    assert_eq!(orders[0].note.as_deref(), Some("leave at the door"));
    assert_eq!(
        orders[0].total_cents, 900,
        "the published price, never a posted one"
    );
    let lines = owner.site_order_lines(&site, &orders[0].id).await.unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].item_name, "Sourdough");
    assert_eq!(lines[0].quantity, 2);

    // The order is in the owner's tenant and nowhere else.
    assert!(outsider.site_orders(&site).await.unwrap().is_empty());
}

#[tokio::test]
async fn unknown_draft_and_ordering_off_catalog_ids_are_one_clean_404() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-missing").await;

    // A real catalog on a site that was never published.
    let draft_site = owner
        .create_site("Draft", &unique("order-missing"))
        .await
        .unwrap();
    let draft_catalog = owner
        .create_site_catalog(
            &draft_site,
            &SiteCatalogInput {
                name: "Nothing yet",
                currency: "EUR",
                orders_enabled: true,
            },
        )
        .await
        .unwrap();
    // A live catalog published with ordering switched off.
    let (closed_site, closed_catalog) = live_site_with_catalog(&owner, "order-closed", false).await;

    let valid = "qty-sourdough=1&name=Eve&email=eve%40example.test";
    for catalog_id in [
        SiteCatalogId::generate().as_str(),
        draft_catalog.as_str(),
        closed_catalog.as_str(),
    ] {
        let response = post_order(&state, catalog_id, "203.0.113.12", valid).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "id {catalog_id}");
    }
    assert!(
        owner.site_orders(&closed_site).await.unwrap().is_empty(),
        "a 404 must write nothing"
    );
    assert!(owner.site_orders(&draft_site).await.unwrap().is_empty());
}

#[tokio::test]
async fn the_honeypot_is_a_silent_drop_indistinguishable_from_success() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-honeypot").await;
    let (site, catalog) = live_site_with_catalog(&owner, "order-honeypot", true).await;

    let response = post_order(
        &state,
        catalog.as_str(),
        "203.0.113.13",
        "qty-sourdough=9&name=Bot&email=bot%40example.test&website=https%3A%2F%2Fspam.example",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK, "the bot sees success");
    assert!(body_string(response).await.contains("Order received"));
    assert!(
        owner.site_orders(&site).await.unwrap().is_empty(),
        "but nothing was written"
    );
}

#[tokio::test]
async fn refused_orders_are_400_with_the_reason_and_write_nothing() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-refused").await;
    let (site, catalog) = live_site_with_catalog(&owner, "order-refused", true).await;

    // Broken percent-encoding: unreadable as a form at all.
    let broken = post_order(&state, catalog.as_str(), "203.0.113.14", "name=%zz").await;
    assert_eq!(broken.status(), StatusCode::BAD_REQUEST);

    // Readable, but refused by the write gate — each with its own sentence.
    for (body, reason) in [
        ("qty-sourdough=1&name=&email=a%40b.test", "name"),
        ("qty-sourdough=1&name=Ada&email=nope", "email"),
        ("qty-sourdough=0&name=Ada&email=a%40b.test", "at least one"),
        ("qty-focaccia=1&name=Ada&email=a%40b.test", "Focaccia"),
        ("qty-brioche=1&name=Ada&email=a%40b.test", "reload"),
        (
            "qty-sourdough=lots&name=Ada&email=a%40b.test",
            "could not be read",
        ),
    ] {
        let response = post_order(&state, catalog.as_str(), "203.0.113.14", body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "body {body}");
        let page = body_string(response).await;
        assert!(page.contains(reason), "page names {reason}, got: {page}");
    }

    assert!(owner.site_orders(&site).await.unwrap().is_empty());
}

#[tokio::test]
async fn an_oversized_order_body_is_413() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-oversize").await;
    let (_, catalog) = live_site_with_catalog(&owner, "order-oversize", true).await;

    let huge = format!(
        "qty-sourdough=1&name=Ada&email=a%40b.test&note={}",
        "x".repeat(80 * 1024)
    );
    let response = post_order(&state, catalog.as_str(), "203.0.113.15", &huge).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn the_per_client_rate_limit_answers_429_with_retry_after() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "order-rate").await;
    let (site, catalog) = live_site_with_catalog(&owner, "order-rate", true).await;

    // Exhaust one client's window with honeypot bodies: attempts count against
    // the limiter before anything else, and no rows accumulate.
    let burn = "qty-sourdough=1&name=Bot&email=bot%40example.test&website=x";
    for n in 0..10 {
        let response = post_order(&state, catalog.as_str(), "198.51.100.21", burn).await;
        assert_eq!(response.status(), StatusCode::OK, "attempt {n} in budget");
    }
    let limited = post_order(&state, catalog.as_str(), "198.51.100.21", burn).await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = limited
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("Retry-After header in seconds");
    assert!((1..=600).contains(&retry_after));

    // A different client is untouched — and its order lands.
    let other = post_order(
        &state,
        catalog.as_str(),
        "198.51.100.22",
        "qty-sourdough=1&name=Grace&email=grace%40example.test",
    )
    .await;
    assert_eq!(other.status(), StatusCode::OK);
    let orders = owner.site_orders(&site).await.unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].customer_name, "Grace");
}
