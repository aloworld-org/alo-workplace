//! The heatmap half of the page beacon end to end (S2.09a): what a published
//! page sends when it is clicked and scrolled, what the collect endpoint
//! stores, and everything it refuses. Assertions read the real migrated
//! Postgres schema, so a heatmap that ever grew a visitor identity, an
//! unbounded page set, or a raw coordinate would fail here.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteId, SitePublicStore, Store};

const APEX: &str = "heatmap.test";
const SECRET: &[u8] = b"heatmap-integration-fixture-secret";

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
    let tenant = store
        .create_tenant(&format!("heatmap-{tag}"))
        .await
        .unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@heatmap.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

async fn publish(acc: &AccountStore, subdomain: &str) -> SiteId {
    let site = acc.create_site("HEATMAP", subdomain).await.unwrap();
    let page = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &page,
        json!({
            "schema_version": 1,
            "sections": [{"type": "hero", "heading": "HEATMAP"}]
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
        .header("x-forwarded-for", format!("203.0.113.9, {client}"))
        .header(header::CONTENT_TYPE, "text/plain;charset=UTF-8")
        .body(Body::from(body))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

/// Every heatmap row a site has accumulated, ordered for comparison.
async fn cells(pool: &PgPool, site: &SiteId) -> Vec<(String, String, String, i16, i16, i64)> {
    sqlx::query_as::<_, (String, String, String, i16, i16, i64)>(
        "SELECT path, viewport, metric, grid_x, grid_y, hits \
         FROM site_analytics_heatmap_daily WHERE site_id = $1 \
         ORDER BY path, viewport, metric, grid_y, grid_x",
    )
    .bind(site.as_str())
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn clicks_and_scrolls_become_cells_on_the_host_s_own_site() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant")).await;
    let subdomain = unique("site");
    let site = publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // A second site, on a second tenant, whose host is never used below: no
    // beacon may ever reach it, because the body has no field naming a site.
    let other_owner = account(&store, &unique("other")).await;
    let other_subdomain = unique("other-site");
    let other_site = publish(&other_owner, &other_subdomain).await;

    // Two nearby clicks on a phone, one on a desktop, and a scroll to 88%.
    for body in [
        "x=500&y=250&p=%2Fprices&w=390",
        "x=505&y=253&p=%2Fprices&w=414",
        "x=10&y=990&p=%2Fprices&w=1440",
        "d=880&p=%2Fprices&w=390",
    ] {
        let response = beacon(&state, &host, "198.51.100.41", body).await;
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
        cells(&pool, &site).await,
        vec![
            (
                "/prices".to_owned(),
                "desktop".to_owned(),
                "click".to_owned(),
                0,
                63,
                1
            ),
            (
                "/prices".to_owned(),
                "phone".to_owned(),
                "click".to_owned(),
                16,
                16,
                2
            ),
            (
                "/prices".to_owned(),
                "phone".to_owned(),
                "scroll".to_owned(),
                0,
                8,
                1
            ),
        ],
        "the pixel position and the screen size never reach storage"
    );
    assert!(
        cells(&pool, &other_site).await.is_empty(),
        "a beacon sent to one host reached another tenant's site"
    );

    // A heatmap event is not a page view and creates no identity of any kind.
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
        assert_eq!(rows, 0, "{table} gained a row from a heatmap beacon");
    }
}

#[tokio::test]
async fn the_published_page_reports_where_it_was_clicked_and_how_far_it_was_read() {
    let (store, _pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-page")).await;
    let subdomain = unique("site-page");
    publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    let response = app(Arc::clone(&state))
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, &host)
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
    for fragment in [
        "encodeURIComponent(location.pathname)",
        "\"x=\" + permille(event.pageX",
        "send(\"d=\" + reach + shape())",
        "clicks < 20",
    ] {
        assert!(
            html.contains(fragment),
            "the published page must carry the heatmap beacon ({fragment})"
        );
    }
    assert!(
        !html.contains("document.cookie") && !html.contains("localStorage"),
        "the beacon must keep no identity on the visitor's machine"
    );
}

#[tokio::test]
async fn an_incomplete_or_hostile_heatmap_report_is_refused() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-refuse")).await;
    let subdomain = unique("site-refuse");
    let site = publish(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    for body in [
        // A click or a scroll with no page, or a page with no event.
        "x=500&y=250&w=390",
        "d=880&w=390",
        "p=%2Fprices&w=390",
        // A click with no viewport, and half a click.
        "x=500&y=250&p=%2Fprices",
        "x=500&p=%2Fprices&w=390",
        // Numbers that are not measurements.
        "x=-5&y=250&p=%2Fprices&w=390",
        "d=88.5&p=%2Fprices&w=390",
        // Paths that are not page paths of this site.
        "d=880&p=https%3A%2F%2Felsewhere.example%2Fx&w=390",
        "d=880&p=%2Fprices%3Futm_campaign%3Dspring&w=390",
        "d=880&p=%3Cscript%3E&w=390",
    ] {
        let response = beacon(&state, &host, "198.51.100.42", body).await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "body {body} was accepted"
        );
    }
    assert!(
        cells(&pool, &site).await.is_empty(),
        "a refused report still wrote a row"
    );

    // An unresolvable host is the same terse miss a page request gets, and
    // reveals nothing about which sites exist.
    let response = beacon(
        &state,
        &format!("{}.{APEX}", unique("nobody")),
        "198.51.100.43",
        "x=500&y=250&p=%2Fprices&w=390",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap()
            .is_empty()
    );
}
