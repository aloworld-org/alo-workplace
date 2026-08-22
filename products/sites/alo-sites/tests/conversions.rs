//! The conversion funnel end to end (S2.10a): what a published page with a
//! form on it reports, what the collect endpoint stores, what the submission
//! endpoint counts for itself, and everything both refuse. Assertions read the
//! real migrated Postgres schema, so a funnel that ever grew a visitor
//! identity or an unbounded source set would fail here.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteFormId, SiteId, SitePublicStore, Store};

const APEX: &str = "conversions.test";
const SECRET: &[u8] = b"conversions-integration-fixture-secret";

/// The database this suite runs against.
///
/// Delegates to `alo_test_db`, which refuses the database the product
/// runs on: suites create and drop their own, they never write into `alo`.
fn database_url() -> String {
    alo_test_db::url()
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
        .create_tenant(&format!("conversions-{tag}"))
        .await
        .unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@conversions.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

/// A live site whose home page carries a real contact form.
async fn publish_with_form(acc: &AccountStore, subdomain: &str) -> (SiteId, SiteFormId) {
    let site = acc.create_site("Contactable", subdomain).await.unwrap();
    let page = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    let form = acc.create_site_form(&site, "Contact").await.unwrap();
    acc.set_page_sections(
        &site,
        &page,
        json!({
            "schema_version": 1,
            "sections": [
                {"type": "hero", "heading": "Contactable"},
                {
                    "type": "contact_form",
                    "heading": "Say hello",
                    "form_id": form.as_str(),
                    "success_message": "Thank you"
                }
            ]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, form)
}

/// One beacon POST from `client`, exactly as `navigator.sendBeacon` sends it.
async fn beacon(state: &Arc<AppState>, host: &str, client: &str, body: String) -> Response {
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

/// One urlencoded submission POST through the real router.
async fn post_form(state: &Arc<AppState>, form_id: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/f/{form_id}"))
        .header(header::HOST, format!("whatever.{APEX}"))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

/// One GET of a published page.
async fn page(state: &Arc<AppState>, host: &str, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Every conversion row a site has accumulated, ordered for comparison.
async fn rows(pool: &PgPool, site: &SiteId) -> Vec<(String, String, String, i64)> {
    sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT source_kind, source_id, stage, hits FROM site_conversion_daily \
         WHERE site_id = $1 ORDER BY source_id, stage",
    )
    .bind(site.as_str())
    .fetch_all(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_page_reports_its_own_form_and_the_submit_is_counted_at_the_write() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("tenant")).await;
    let subdomain = unique("site");
    let (site, form) = publish_with_form(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // The published page carries the form's own id in its markup — which is
    // exactly what the beacon reports back, so no tracking id is needed.
    let served = body_string(page(&state, &host, "/").await).await;
    assert!(
        served.contains(&format!("action=\"/f/{}\"", form.as_str())),
        "the page did not render its form"
    );
    assert!(
        served.contains("&s=view"),
        "the page carries no conversion beacon"
    );

    // Two visitors saw the form, one of them started filling it in.
    for body in [
        format!("c={}&s=view", form.as_str()),
        format!("c={}&s=view", form.as_str()),
        format!("c={}&s=start", form.as_str()),
    ] {
        let response = beacon(&state, &host, "198.51.100.41", body.clone()).await;
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

    // One of them sent the message. The submit is counted by the endpoint
    // that wrote it, never claimed by the browser.
    let response = post_form(
        &state,
        form.as_str(),
        "198.51.100.41",
        "name=Ada&email=ada%40example.test&message=Hello+there&website=",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        rows(&pool, &site).await,
        vec![
            (
                "form".to_owned(),
                form.as_str().to_owned(),
                "start".to_owned(),
                1
            ),
            (
                "form".to_owned(),
                form.as_str().to_owned(),
                "submit".to_owned(),
                1
            ),
            (
                "form".to_owned(),
                form.as_str().to_owned(),
                "view".to_owned(),
                2
            ),
        ]
    );

    // A conversion count creates no identity anywhere. The one page request
    // above counted a page view like any other, with its day-scoped token —
    // the beacons and the submission that followed added nothing to it.
    for (table, expected) in [
        ("site_analytics_daily_visitors", 1),
        ("site_analytics_visitor_day", 1),
    ] {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT COUNT(*) FROM {table} WHERE site_id = $1"
        ))
        .bind(site.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count, expected,
            "{table} gained a row from a conversion beacon"
        );
    }
}

#[tokio::test]
async fn a_foreign_or_invented_source_is_a_quiet_two_oh_four_that_writes_nothing() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("host")).await;
    let subdomain = unique("host-site");
    let (site, _) = publish_with_form(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    // A second tenant's form id is public in that tenant's own markup, so a
    // hostile visitor has it. Posted against this host it moves nothing.
    let other_owner = account(&store, &unique("other")).await;
    let other_subdomain = unique("other-site");
    let (other_site, other_form) = publish_with_form(&other_owner, &other_subdomain).await;

    for body in [
        format!("c={}&s=view", other_form.as_str()),
        format!("c={}&s=start", other_form.as_str()),
        format!("c={}&s=view", SiteFormId::generate().as_str()),
    ] {
        let response = beacon(&state, &host, "198.51.100.42", body.clone()).await;
        assert_eq!(
            response.status(),
            StatusCode::NO_CONTENT,
            "a refusal must be indistinguishable from a count: {body}"
        );
    }
    assert!(rows(&pool, &site).await.is_empty());
    assert!(
        rows(&pool, &other_site).await.is_empty(),
        "a beacon sent to one host counted on another tenant's site"
    );

    // An unknown Host still cannot reach any site at all.
    let response = beacon(
        &state,
        &format!("nobody.{APEX}"),
        "198.51.100.43",
        format!("c={}&s=view", other_form.as_str()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_and_over_claiming_conversion_bodies_are_refused() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("bad")).await;
    let subdomain = unique("bad-site");
    let (site, form) = publish_with_form(&owner, &subdomain).await;
    let host = format!("{subdomain}.{APEX}");

    for body in [
        // Half a report, in both directions.
        format!("c={}", form.as_str()),
        "s=view".to_owned(),
        format!("c={}&s=", form.as_str()),
        // A stage word that is not one.
        format!("c={}&s=opened", form.as_str()),
        format!("c={}&s=VIEW", form.as_str()),
        // The one stage a browser may not claim: it is counted at the write.
        format!("c={}&s=submit", form.as_str()),
        // Values that are not ids.
        "c=%3Cscript%3E&s=view".to_owned(),
        "c=one%20two&s=view".to_owned(),
        "c=..%2F..%2Fetc%2Fpasswd&s=view".to_owned(),
    ] {
        let response = beacon(&state, &host, "198.51.100.44", body.clone()).await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "body {body} was accepted"
        );
    }
    assert!(rows(&pool, &site).await.is_empty());
}

#[tokio::test]
async fn a_dropped_submission_counts_no_submit() {
    let (store, pool, state) = harness().await;
    let owner = account(&store, &unique("drop")).await;
    let subdomain = unique("drop-site");
    let (site, form) = publish_with_form(&owner, &subdomain).await;

    // The honeypot answers exactly like success and writes nothing — so it
    // must also count nothing, or the funnel would report conversions the
    // owner has no message for.
    let response = post_form(
        &state,
        form.as_str(),
        "198.51.100.45",
        "name=Bot&email=bot%40example.test&message=spam&website=http%3A%2F%2Fspam.example",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    // A refused submission does not count either.
    let response = post_form(
        &state,
        form.as_str(),
        "198.51.100.46",
        "name=&email=&message=",
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    assert!(rows(&pool, &site).await.is_empty());
}
