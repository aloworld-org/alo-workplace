//! The page beacon end to end (S2.08a2): what the published page sends, what
//! the collect endpoint accepts, and — the load-bearing part — everything it
//! refuses. Assertions read the real migrated Postgres schema, so a beacon
//! that ever grew a visitor identity or an unbounded bucket would fail here.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteId, SitePublicStore, Store};

const APEX: &str = "beacon.test";
const SECRET: &[u8] = b"beacon-integration-fixture-secret";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5432/alo".to_owned())
}

async fn harness() -> (Store, PgPool, Arc<AppState>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres");
    let blobs = BlobStore::in_memory(1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");
    let state = AppState::new(
        SitePublicStore::new(pool.clone(), blobs),
        APEX.to_owned(),
        SECRET,
    );
    (store, pool, state)
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}",
        SiteId::generate()
            .as_str()
            .to_ascii_lowercase()
            .replace('_', "-")
    )
}

async fn account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("beacon-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@beacon.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

async fn publish(acc: &AccountStore, subdomain: &str) -> SiteId {
    let site = acc.create_site("BEACON", subdomain).await.unwrap();
    let page = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &page,
        json!({
            "schema_version": 1,
            "sections": [{"type": "hero", "heading": "BEACON"}]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    site
}

/// One beacon POST from `client`, exactly as `navigator.sendBeacon` sends it.
async fn beacon(
    state: &Arc<AppState>,
    host: &str,
    client: &str,
    body: &'static str,
) -> axum::http::Response<Body> {
    let request = Request::builder()
        .method("POST")
        .uri("/_alo/collect")
        .header(header::HOST, host)
        .header("x-forwarded-for", format!("203.0.113.4, {client}"))
        .header(header::CONTENT_TYPE, "text/plain;charset=UTF-8")
        .body(Body::from(body))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

/// Every dimension row a site has accumulated, ordered for comparison.
async fn dimensions(pool: &PgPool, site: &SiteId) -> Vec<(String, String, i64)> {
    sqlx::query_as::<_, (String, String, i64)>(
        "SELECT dimension, value, hits FROM site_analytics_dimension_daily \
         WHERE site_id = $1 ORDER BY dimension, value",
    )
    .bind(site.as_str())
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_beacon_reports_only_a_read_bucket_and_a_destination_domain() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant")).await;
    let subdomain = unique("site");
    let site = publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // A reader who spent 137 seconds on the page and then followed a link out.
    for body in ["t=137", "o=News.Example", "o=news.example", "t=4"] {
        let response = beacon(&state, &host, "198.51.100.21", body).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT, "body {body}");
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }

    assert_eq!(
        dimensions(&pool, &site).await,
        vec![
            // The destination is one lowercase host, counted twice.
            ("outbound".to_owned(), "news.example".to_owned(), 2),
            // 137 seconds is stored as a bucket; 4 seconds is its own.
            ("read_time".to_owned(), "0-10s".to_owned(), 1),
            ("read_time".to_owned(), "1-3m".to_owned(), 1),
        ],
        "the exact duration and the full link never reach storage"
    );

    // A beacon is not a page view, and creates no visitor identity of any
    // kind: four reports, and both visitor tables are still empty.
    for table in [
        "site_analytics_daily",
        "site_analytics_daily_visitors",
        "site_analytics_visitor_day",
    ] {
        let rows = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE site_id = $1"
        ))
        .bind(site.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rows, 0, "{table} gained a row from a beacon");
    }
}

#[tokio::test]
async fn the_published_page_carries_the_beacon_and_a_scriptless_visit_still_counts() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-noscript")).await;
    let subdomain = unique("site-noscript");
    let site = publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    let response = app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, &host)
                .header("x-forwarded-for", "203.0.113.4, 198.51.100.31")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        html.contains("navigator.sendBeacon(\"/_alo/collect\""),
        "the published page must carry the beacon"
    );

    // The visit was recorded by the server, with no beacon sent at all: a
    // visitor who runs no scripts is a fully counted page view.
    let view = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT path, hits, unique_visitors FROM site_analytics_daily WHERE site_id = $1",
    )
    .bind(site.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(view, ("/".to_owned(), 1, 1));
    assert!(
        dimensions(&pool, &site)
            .await
            .iter()
            .all(|(dimension, _, _)| dimension != "read_time" && dimension != "outbound"),
        "a page view must not invent a beacon dimension"
    );
}

#[tokio::test]
async fn the_endpoint_refuses_what_it_cannot_defend() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-refuse")).await;
    let subdomain = unique("site-refuse");
    let site = publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // A payload that is not one of the two reports, in every shape it comes
    // in: unknown keys, a path, a URL, markup, and an empty body.
    for body in [
        "",
        "path=/prices",
        "o=%3Cscript%3Ealert(1)%3C%2Fscript%3E",
        "o=https%3A%2F%2Fnews.example%2Fprivate",
        "o=someone%40example.test",
        "t=-1",
        "visitor=abc123",
    ] {
        assert_eq!(
            beacon(&state, &host, "198.51.100.41", body).await.status(),
            StatusCode::BAD_REQUEST,
            "body {body:?} was accepted"
        );
    }

    // An unknown host is the same terse 404 a page request gets, so the
    // endpoint cannot be used to enumerate which sites exist.
    for unknown_host in [
        format!("{}.{APEX}", unique("nobody")),
        "not-our-domain.test".to_owned(),
    ] {
        assert_eq!(
            beacon(&state, &unknown_host, "198.51.100.42", "t=30")
                .await
                .status(),
            StatusCode::NOT_FOUND,
            "host {unknown_host} answered a beacon"
        );
    }

    // Over the route's body cap: refused without being buffered.
    let oversized = "o=".to_owned() + &"a".repeat(4096);
    let response = app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/_alo/collect")
                .header(header::HOST, &host)
                .header("x-forwarded-for", "203.0.113.4, 198.51.100.43")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

    assert!(
        dimensions(&pool, &site).await.is_empty(),
        "a refused beacon must write nothing"
    );
}

#[tokio::test]
async fn beacon_traffic_has_its_own_budget() {
    let (store, _pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-rate")).await;
    let subdomain = unique("site-rate");
    publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // The budget is spent on refusals too — the limiter runs before the body
    // is even read, which is the point of putting it first.
    let mut refused = None;
    for attempt in 0..=alo_sites::serve::rate::BEACON_MAX_PER_WINDOW {
        let response = beacon(&state, &host, "198.51.100.51", "nope=1").await;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            refused = Some((attempt, response));
            break;
        }
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let (attempt, response) = refused.expect("the beacon budget must run out");
    assert_eq!(attempt, alo_sites::serve::rate::BEACON_MAX_PER_WINDOW);
    assert!(
        response.headers().contains_key(header::RETRY_AFTER),
        "a refusal must say when to come back"
    );

    // Another client is unaffected: the ceiling is per address, not per site.
    assert_eq!(
        beacon(&state, &host, "198.51.100.52", "t=30")
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
}
