//! In-process integration tests of `POST /_alo/chat/lead` (item S3.03d):
//! real fixtures through the real store into the compose Postgres, real
//! requests through the real router.
//!
//! The mandatory isolation case is the serving Host deciding the tenant —
//! the same visitor on two tenants' sites raises two separate leads, and an
//! unknown host is the uniform 404. The rest pin the wire contract (saved,
//! known, invalid-with-verbatim-detail, the visitor-token gate) and the
//! attribution promise: a successful capture moves exactly one aggregate
//! counter, keyed by the site's own id, in a table whose columns cannot hold
//! a visitor.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, DealFilter, SiteId, SitePublicStore, Store};
use serde_json::{Value, json};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5433/alo".to_owned())
}

async fn harness() -> (Store, Arc<AppState>, sqlx::PgPool) {
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
        b"chat-lead-tests-analytics-secret",
    );
    (store, state, pool)
}

async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@chat-lead.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

/// A live site with one published home page; returns its id and subdomain.
async fn live_site(acc: &AccountStore, tag: &str) -> (SiteId, String) {
    let subdomain = format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    );
    let site = acc.create_site("Studio", &subdomain).await.unwrap();
    acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, subdomain)
}

async fn post_lead(state: &Arc<AppState>, host: &str, body: Value) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri("/_alo/chat/lead")
        .header(header::HOST, host)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn state_of(response: Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn lead_body(visitor: &str, name: &str, email: &str) -> Value {
    json!({"visitor": visitor, "name": name, "email": email, "company": "Newco BV"})
}

/// The whole arc on the wire: capture → the card stands in the owning
/// tenant's CRM with the site's words and the host as source → exactly one
/// aggregate 'chat' submit counter moved, keyed by the site's own id.
#[tokio::test]
async fn a_captured_lead_stands_in_crm_and_counts_once_in_the_aggregate() {
    let (store, state, pool) = harness().await;
    let acc = fresh_account(&store, "arc").await;
    let (site, subdomain) = live_site(&acc, "arc").await;
    let host = format!("{subdomain}.{APEX}");

    let (status, body) = state_of(
        post_lead(
            &state,
            &host,
            lead_body("visitor-token-1", "Vera Visitor", "vera@newco.example"),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "lead_saved");

    // The card is CRM's own record in the owning tenant.
    let deals = acc.crm_deals(&DealFilter::default()).await.unwrap();
    assert_eq!(deals.len(), 1);
    assert_eq!(deals[0].title, "Website enquiry — Studio");
    assert_eq!(deals[0].contact_name, "Vera Visitor");
    assert_eq!(deals[0].contact_email, "vera@newco.example");
    assert_eq!(deals[0].company_name, "Newco BV");
    assert_eq!(deals[0].source, host);
    assert_eq!(deals[0].value_cents, 0);

    // Exactly one aggregate submit, keyed by the site's own id — and the
    // tenant's own funnel read reports it under the 'chat' source.
    let (source_id, hits): (String, i64) = sqlx::query_as(
        "SELECT source_id, hits FROM site_conversion_daily \
         WHERE tenant_id = $1 AND site_id = $2 AND source_kind = 'chat' AND stage = 'submit'",
    )
    .bind(acc.tenant().as_str())
    .bind(site.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(source_id, site.as_str());
    assert_eq!(hits, 1);
    let today = time::OffsetDateTime::now_utc().date();
    let report = acc
        .site_conversions(&site, today, today)
        .await
        .unwrap()
        .unwrap();
    let chat = report
        .sources
        .iter()
        .find(|source| source.kind == "chat")
        .expect("the chat source is reported");
    assert_eq!(chat.submits, 1);
    assert_eq!(chat.id, site.as_str());
}

/// The attribution table cannot carry a visitor: its columns are the site's
/// key, the day, the source words and a tally — nothing else. A journey
/// column added later fails this test until its privacy is argued.
#[tokio::test]
async fn the_aggregate_table_has_no_column_a_visitor_could_travel_in() {
    let (_, _, pool) = harness().await;
    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'site_conversion_daily' ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let names: Vec<&str> = columns.iter().map(|(name,)| name.as_str()).collect();
    assert_eq!(
        names,
        ["day", "hits", "site_id", "source_id", "source_kind", "stage", "tenant_id"]
    );
}

/// The serving Host decides the tenant: the same visitor on two tenants'
/// sites is two separate leads, each visible only to its own tenant.
#[tokio::test]
async fn the_host_header_decides_which_tenant_gets_the_lead() {
    let (store, state, _) = harness().await;
    let a = fresh_account(&store, "iso-a").await;
    let b = fresh_account(&store, "iso-b").await;
    let (_, sub_a) = live_site(&a, "iso-a").await;
    let (_, sub_b) = live_site(&b, "iso-b").await;

    for sub in [&sub_a, &sub_b] {
        let (status, body) = state_of(
            post_lead(
                &state,
                &format!("{sub}.{APEX}"),
                lead_body("visitor-token-2", "Vera", "vera@sameaddress.example"),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["state"], "lead_saved");
    }
    for acc in [&a, &b] {
        let deals = acc.crm_deals(&DealFilter::default()).await.unwrap();
        assert_eq!(deals.len(), 1, "one lead per tenant, never a crossing");
    }
}

/// A known address is answered, not filed twice — and the wire says only
/// "known", never which record or what kind of record made it so.
#[tokio::test]
async fn a_second_capture_from_the_same_address_answers_known() {
    let (store, state, _) = harness().await;
    let acc = fresh_account(&store, "known").await;
    let (_, subdomain) = live_site(&acc, "known").await;
    let host = format!("{subdomain}.{APEX}");

    for (round, expected) in ["lead_saved", "lead_known"].into_iter().enumerate() {
        let (status, body) = state_of(
            post_lead(
                &state,
                &host,
                lead_body("visitor-token-3", "Vera", "vera@twice.example"),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "round {round}: {body}");
        assert_eq!(body["state"], expected, "round {round}");
        assert_eq!(body.as_object().unwrap().len(), 1, "state and nothing else");
    }
    assert_eq!(acc.crm_deals(&DealFilter::default()).await.unwrap().len(), 1);
}

/// A field CRM refuses comes back 400 with the store's own sentence in
/// `detail` — the visitor can actually fix it — and nothing was written.
#[tokio::test]
async fn a_refused_field_is_a_verbatim_400_and_writes_nothing() {
    let (store, state, _) = harness().await;
    let acc = fresh_account(&store, "refused").await;
    let (_, subdomain) = live_site(&acc, "refused").await;

    let (status, body) = state_of(
        post_lead(
            &state,
            &format!("{subdomain}.{APEX}"),
            lead_body("visitor-token-4", "Vera", "not-an-address"),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["state"], "invalid");
    assert!(
        body["detail"].as_str().unwrap().contains("valid address"),
        "{body}"
    );
    assert!(acc.crm_deals(&DealFilter::default()).await.unwrap().is_empty());
}

/// The gates in front of the capture: an unknown host is the uniform 404, a
/// missing or malformed visitor token is 400, and a field past the widget's
/// own cap is 400 — all before anything reaches CRM.
#[tokio::test]
async fn the_gates_hold_before_crm_is_reached() {
    let (store, state, _) = harness().await;
    let acc = fresh_account(&store, "gates").await;
    let (_, subdomain) = live_site(&acc, "gates").await;
    let host = format!("{subdomain}.{APEX}");

    let unknown = post_lead(
        &state,
        &format!("nobody-here.{APEX}"),
        lead_body("visitor-token-5", "Vera", "vera@nowhere.example"),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);

    let no_token = post_lead(&state, &host, json!({"name": "V", "email": "v@x.example"})).await;
    assert_eq!(no_token.status(), StatusCode::BAD_REQUEST);

    let oversized = post_lead(
        &state,
        &host,
        lead_body("visitor-token-6", &"n".repeat(201), "vera@long.example"),
    )
    .await;
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);

    assert!(acc.crm_deals(&DealFilter::default()).await.unwrap().is_empty());
}
