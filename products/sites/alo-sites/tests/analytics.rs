//! End-to-end privacy and tenancy proof for public site analytics. Requests
//! carry deliberately sensitive-looking raw metadata; assertions inspect the
//! real migrated Postgres schema and rows to prove only safe derivatives are
//! retained.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteId, SitePublicStore, Store};

const APEX: &str = "analytics.test";
const ANALYTICS_SECRET: &[u8] = b"analytics-integration-fixture-secret";

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
        ANALYTICS_SECRET,
    );
    (store, pool, state)
}

async fn account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store
        .create_tenant(&format!("analytics-{tag}"))
        .await
        .unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@analytics.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
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

async fn publish(acc: &AccountStore, subdomain: &str, marker: &str) -> SiteId {
    let site = acc.create_site(marker, subdomain).await.unwrap();
    let page = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &page,
        json!({
            "schema_version": 1,
            "sections": [{"type": "hero", "heading": marker}]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    site
}

async fn view(state: &Arc<AppState>, host: &str, client: &str, user_agent: &str) {
    visit_path(
        state,
        host,
        client,
        user_agent,
        "/?private_query=must-not-be-stored",
        &[],
    )
    .await;
}

async fn visit_path(
    state: &Arc<AppState>,
    host: &str,
    client: &str,
    user_agent: &str,
    uri: &str,
    extra_headers: &[(&str, &str)],
) {
    let mut builder = Request::builder()
        .uri(uri)
        .header(header::HOST, host)
        .header("x-forwarded-for", format!("203.0.113.4, {client}"))
        .header(header::USER_AGENT, user_agent)
        .header(
            header::REFERER,
            "https://NEWS.Example/private/customer?token=must-not-be-stored",
        );
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = app(Arc::clone(state)).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dimensions_are_derived_at_the_door_and_the_raw_request_is_dropped() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant-dimensions")).await;
    let subdomain = unique("site-dimensions");
    let site = owner.create_site("DIMENSIONS", &subdomain).await.unwrap();
    let home = owner
        .create_site_page(&site, "Home", "", true)
        .await
        .unwrap();
    let about = owner
        .create_site_page(&site, "About", "about", false)
        .await
        .unwrap();
    for page in [&home, &about] {
        owner
            .set_page_sections(
                &site,
                page,
                json!({
                    "schema_version": 1,
                    "sections": [{"type": "hero", "heading": "DIMENSIONS"}]
                }),
            )
            .await
            .unwrap();
    }
    owner.publish_site(&site).await.unwrap();
    let host = format!("{subdomain}.{APEX}");

    // One visitor arrives on a campaign link from a phone, then reads on.
    visit_path(
        &state,
        &host,
        "198.51.100.11",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) AppleWebKit/605.1.15 Mobile/15E148",
        "/?utm_campaign=Spring+Sale&utm_content=secret-recipient-id&email=someone%40example.test",
        &[("cf-ipcountry", "nl")],
    )
    .await;
    visit_path(
        &state,
        &host,
        "198.51.100.11",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) AppleWebKit/605.1.15 Mobile/15E148",
        "/about",
        &[("cf-ipcountry", "nl")],
    )
    .await;
    // A different visitor, no campaign, no country from the edge, a crawler.
    visit_path(
        &state,
        &host,
        "198.51.100.12",
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
        "/about",
        &[("cf-ipcountry", "XX")],
    )
    .await;

    let rows = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT dimension, value, hits FROM site_analytics_dimension_daily \
         WHERE site_id = $1 ORDER BY dimension, value",
    )
    .bind(site.as_str())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            // The campaign label survives; nothing else from the query does.
            ("campaign".to_owned(), String::new(), 2),
            ("campaign".to_owned(), "spring sale".to_owned(), 1),
            ("country".to_owned(), String::new(), 1),
            ("country".to_owned(), "NL".to_owned(), 2),
            ("device".to_owned(), "bot".to_owned(), 1),
            ("device".to_owned(), "phone".to_owned(), 2),
            // The phone entered on "/" and left from "/about"; the crawler
            // both entered and left on "/about".
            ("entry".to_owned(), "/".to_owned(), 1),
            ("entry".to_owned(), "/about".to_owned(), 1),
            ("exit".to_owned(), "/".to_owned(), 0),
            ("exit".to_owned(), "/about".to_owned(), 2),
        ]
    );

    let cursors = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT visitor_hash, last_path FROM site_analytics_visitor_day \
         WHERE site_id = $1 ORDER BY last_path",
    )
    .bind(site.as_str())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(cursors.len(), 2, "one cursor per visitor-day, not per view");
    for (token, last_path) in &cursors {
        assert_eq!(token.len(), 32);
        assert_ne!(token.as_slice(), b"198.51.100.11");
        assert_eq!(last_path, "/about");
    }

    // The new tables carry exactly these columns and no others: a future
    // migration cannot quietly widen them into request storage.
    let columns = sqlx::query_as::<_, (String, String)>(
        "SELECT table_name, column_name FROM information_schema.columns \
         WHERE table_schema = 'public' \
           AND table_name IN ('site_analytics_dimension_daily', 'site_analytics_visitor_day') \
         ORDER BY table_name, ordinal_position",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let named: Vec<(&str, &str)> = columns
        .iter()
        .map(|(table, column)| (table.as_str(), column.as_str()))
        .collect();
    assert_eq!(
        named,
        vec![
            ("site_analytics_dimension_daily", "tenant_id"),
            ("site_analytics_dimension_daily", "site_id"),
            ("site_analytics_dimension_daily", "day"),
            ("site_analytics_dimension_daily", "dimension"),
            ("site_analytics_dimension_daily", "value"),
            ("site_analytics_dimension_daily", "hits"),
            ("site_analytics_visitor_day", "tenant_id"),
            ("site_analytics_visitor_day", "site_id"),
            ("site_analytics_visitor_day", "day"),
            ("site_analytics_visitor_day", "visitor_hash"),
            ("site_analytics_visitor_day", "last_path"),
        ]
    );

    // The one thing a campaign link most often carries about a person — an
    // address in a tracking parameter — is nowhere in the analytics tables.
    for table in [
        "site_analytics_daily",
        "site_analytics_dimension_daily",
        "site_analytics_visitor_day",
    ] {
        let leaked = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} t WHERE t.site_id = $1 \
             AND t::text LIKE '%example.test%'"
        ))
        .bind(site.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(leaked, 0, "{table} kept a raw query parameter");
    }
}

#[tokio::test]
async fn aggregates_views_without_pii_and_keeps_host_tenants_isolated() {
    let (store, pool, state) = harness().await;
    let account_a = account(&store, &unique("tenant-a")).await;
    let account_b = account(&store, &unique("tenant-b")).await;
    let sub_a = unique("site-a");
    let sub_b = unique("site-b");
    let site_a = publish(&account_a, &sub_a, "SITE A").await;
    let site_b = publish(&account_b, &sub_b, "SITE B").await;

    // Same transient address, different user agents: two hits but one daily
    // unique. This also proves user-agent data is not part of the token.
    view(
        &state,
        &format!("{sub_a}.{APEX}"),
        "198.51.100.77",
        "SECRET-UA-FIRST",
    )
    .await;
    view(
        &state,
        &format!("{sub_a}.{APEX}"),
        "198.51.100.77",
        "SECRET-UA-SECOND",
    )
    .await;
    // The same visitor on the other tenant's Host creates only that site's
    // own row: Host resolution, not request data, determines tenant scope.
    view(
        &state,
        &format!("{sub_b}.{APEX}"),
        "198.51.100.77",
        "SECRET-UA-THIRD",
    )
    .await;

    let row_a = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT path, referrer_domain, hits, unique_visitors \
         FROM site_analytics_daily WHERE site_id = $1",
    )
    .bind(site_a.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row_a, ("/".to_owned(), "news.example".to_owned(), 2, 1));

    let row_b = sqlx::query_as::<_, (i64, i64)>(
        "SELECT hits, unique_visitors FROM site_analytics_daily WHERE site_id = $1",
    )
    .bind(site_b.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row_b, (1, 1), "the other tenant has its own aggregate");

    let a_rows = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM site_analytics_daily WHERE site_id = $1",
    )
    .bind(site_a.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        a_rows, 1,
        "query strings and user agents make no dimensions"
    );

    let token = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT visitor_hash FROM site_analytics_daily_visitors WHERE site_id = $1",
    )
    .bind(site_a.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(token.len(), 32);
    assert_ne!(token, b"198.51.100.77");

    // Schema allow-list: a future migration cannot quietly introduce raw
    // request storage without breaking this test.
    let columns = sqlx::query_as::<_, (String, String)>(
        "SELECT table_name, column_name FROM information_schema.columns \
         WHERE table_schema = 'public' \
           AND table_name IN ('site_analytics_daily', 'site_analytics_daily_visitors') \
         ORDER BY table_name, ordinal_position",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let actual: Vec<&str> = columns.iter().map(|(_, column)| column.as_str()).collect();
    for forbidden in [
        "ip",
        "ip_address",
        "user_agent",
        "ua",
        "referrer",
        "referer",
        "query",
        "headers",
    ] {
        assert!(
            !actual.contains(&forbidden),
            "PII column {forbidden} exists"
        );
    }
    assert_eq!(
        columns,
        vec![
            ("site_analytics_daily".to_owned(), "tenant_id".to_owned()),
            ("site_analytics_daily".to_owned(), "site_id".to_owned()),
            ("site_analytics_daily".to_owned(), "day".to_owned()),
            ("site_analytics_daily".to_owned(), "path".to_owned()),
            (
                "site_analytics_daily".to_owned(),
                "referrer_domain".to_owned(),
            ),
            ("site_analytics_daily".to_owned(), "hits".to_owned()),
            (
                "site_analytics_daily".to_owned(),
                "unique_visitors".to_owned(),
            ),
            (
                "site_analytics_daily_visitors".to_owned(),
                "tenant_id".to_owned(),
            ),
            (
                "site_analytics_daily_visitors".to_owned(),
                "site_id".to_owned(),
            ),
            ("site_analytics_daily_visitors".to_owned(), "day".to_owned(),),
            (
                "site_analytics_daily_visitors".to_owned(),
                "path".to_owned(),
            ),
            (
                "site_analytics_daily_visitors".to_owned(),
                "referrer_domain".to_owned(),
            ),
            (
                "site_analytics_daily_visitors".to_owned(),
                "visitor_hash".to_owned(),
            ),
        ]
    );
}
