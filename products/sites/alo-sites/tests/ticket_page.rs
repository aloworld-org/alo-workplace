//! In-process integration tests of `GET /t/{token}` and
//! `GET /t/{token}/calendar.ics` (item S3.04d): real paid orders through the
//! real store into the compose Postgres, real requests through the real
//! router.
//!
//! The mandatory isolation case: a ticket token answers only on the host of
//! the site it was minted for — a foreign site's host, an unknown token and
//! a malformed token are one uniform 404. The rest pin the wire contract:
//! the ticket page naming the holder and linking the calendar file, and the
//! `.ics` being a valid RFC 5545 document carrying the event start.
//!
//! The fulfilment claim is a global system sweep, so a concurrently running
//! store suite may legitimately claim this test's order first. The token
//! helper tolerates that: the claim's insert is what mints the token, so
//! whoever claimed it, the row exists and the token is read back from it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{
    AccountStore, BlobStore, NewProduct, SiteId, SitePaymentStatus, SitePublicStore, Store,
};
use time::{Duration, OffsetDateTime};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

/// The database this suite runs against.
///
/// Delegates to `alo_test_db`, which refuses the database the product
/// runs on: suites create and drop their own, they never write into `alo`.
fn database_url() -> String {
    alo_test_db::url()
}

async fn harness() -> (Store, sqlx::PgPool, Arc<AppState>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");
    let state = AppState::new(
        SitePublicStore::new(pool.clone(), blobs),
        APEX.to_owned(),
        b"ticket-page-tests-analytics-secret",
    );
    (store, pool, state)
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

/// A published site with one paid two-seat ticket order, returning the
/// serving subdomain and the minted ticket token.
async fn sold_out_venue(store: &Store, pool: &sqlx::PgPool, tag: &str) -> (String, String) {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@tickets.test"))
        .await
        .unwrap();
    let acc: AccountStore = store.for_account(tenant, user);
    let sub = unique(tag);
    let site = acc.create_site("Letterpress Studio", &sub).await.unwrap();
    acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.publish_site(&site).await.unwrap();
    let product = acc
        .create_billing_product(&NewProduct {
            name: "Letterpress workshop".to_owned(),
            unit: "seat".to_owned(),
            unit_price_cents: 8_500,
            vat_rate_bp: 2100,
            ..Default::default()
        })
        .await
        .unwrap();
    let now = OffsetDateTime::now_utc();
    let event = acc
        .create_site_ticket_event(&site, &product, now + Duration::days(7), 10)
        .await
        .unwrap();
    let hold = acc
        .take_ticket_hold(&site, &event, 2, Duration::minutes(10), now)
        .await
        .unwrap();
    let order = acc
        .create_ticket_order(&site, &hold.id, "Maud Adams", "maud@example.org", now)
        .await
        .unwrap();
    acc.open_ticket_payment(
        &site,
        &order.id,
        &format!("fixpay-page-{}", order.id.as_str()),
        "https://checkout.fixture.invalid/page",
    )
    .await
    .unwrap();
    acc.apply_ticket_payment(&site, &order.id, SitePaymentStatus::Paid, now)
        .await
        .unwrap();

    // Mint the token: claim it ourselves, or — if a concurrently running
    // claiming suite got there first — read the token its claim minted.
    let token = loop {
        let claims = store.claim_ticket_fulfilments(50).await.unwrap();
        if let Some(claim) = claims.into_iter().find(|claim| claim.order == order.id) {
            break claim.token;
        }
        let minted: Option<String> =
            sqlx::query_scalar("SELECT token FROM site_ticket_fulfilments WHERE order_id = $1")
                .bind(order.id.as_str())
                .fetch_optional(pool)
                .await
                .unwrap();
        if let Some(token) = minted {
            break token;
        }
    };
    (sub, token)
}

async fn get_on(state: &Arc<AppState>, subdomain: &str, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, format!("{subdomain}.{APEX}"))
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn body_of(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn the_ticket_page_names_the_holder_and_links_the_calendar() {
    let (store, pool, state) = harness().await;
    let (sub, token) = sold_out_venue(&store, &pool, "page").await;

    let response = get_on(&state, &sub, &format!("/t/{token}")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cache = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert_eq!(cache, "no-store", "a ticket is nobody else's");
    let html = body_of(response).await;
    assert!(html.contains("Maud Adams"));
    assert!(html.contains(&format!("/t/{token}/calendar.ics")));

    let calendar = get_on(&state, &sub, &format!("/t/{token}/calendar.ics")).await;
    assert_eq!(calendar.status(), StatusCode::OK);
    let content_type = calendar
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(content_type.starts_with("text/calendar"));
    let ics = body_of(calendar).await;
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(ics.contains("DTSTART:"));
    assert!(ics.contains(&format!("UID:{token}@sites.alo")));
}

#[tokio::test]
async fn a_token_answers_only_on_the_site_it_was_minted_for() {
    let (store, pool, state) = harness().await;
    let (_, token) = sold_out_venue(&store, &pool, "walled").await;
    let (other_sub, _) = sold_out_venue(&store, &pool, "other").await;

    for path in [format!("/t/{token}"), format!("/t/{token}/calendar.ics")] {
        let foreign = get_on(&state, &other_sub, &path).await;
        assert_eq!(
            foreign.status(),
            StatusCode::NOT_FOUND,
            "a foreign site's host must not serve the ticket at {path}"
        );
    }
    let unknown = get_on(&state, &other_sub, "/t/never-minted-token").await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let malformed = get_on(&state, &other_sub, "/t/a%20token%3Bdrop").await;
    assert_eq!(malformed.status(), StatusCode::NOT_FOUND);
}
