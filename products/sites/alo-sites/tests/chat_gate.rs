//! In-process integration tests of `POST /_alo/chat` (ADR 0040 §3, item
//! S3.02c) — the assistant's cost-and-abuse gate on the public surface. The
//! load-bearing properties: an unknown host and a site whose assistant is
//! off (the default) are one uniform 404; a question that passes every gate
//! is answered with the typed `unavailable` state carrying the site's own
//! published contact page (the answering pipeline is the S3.02e seam); the
//! per-visitor and per-IP budgets refuse with 429 + `Retry-After`; and
//! malformed input never reaches the database gate.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};
use tower::ServiceExt;

use alo_sites::serve::chat::{
    CHAT_BODY_MAX_BYTES, CHAT_IP_MAX_PER_WINDOW, CHAT_MAX_QUESTION_CHARS,
    CHAT_VISITOR_MAX_PER_WINDOW,
};
use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteId, SitePublicStore, Store};

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
        b"chat-gate-tests-analytics-secret",
    );
    (store, state)
}

/// A fresh tenant's account door.
async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@chat.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

/// A subdomain that is unique per test run (the Postgres is shared).
fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

/// A live site: a home page without a contact form and a `/contact` page
/// carrying one, both published.
async fn live_site(acc: &AccountStore, tag: &str) -> (SiteId, String) {
    let sub = unique(tag);
    let site = acc.create_site("Chat Co", &sub).await.unwrap();
    let home = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &home,
        json!({
            "schema_version": 1,
            "sections": [{"type": "hero", "heading": "Chat Co",
                          "subheading": null, "image": null,
                          "primary_cta": null, "secondary_cta": null}]
        }),
    )
    .await
    .unwrap();
    let contact = acc
        .create_site_page(&site, "Contact", "contact", false)
        .await
        .unwrap();
    acc.set_page_sections(
        &site,
        &contact,
        json!({
            "schema_version": 1,
            "sections": [{"type": "contact_form", "heading": "Talk to us",
                          "body": null, "form_id": null,
                          "success_message": "Thanks"}]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, sub)
}

/// One chat POST through the real router. `ip` becomes `X-Forwarded-For`.
async fn post_chat(state: &Arc<AppState>, sub: &str, ip: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri("/_alo/chat")
        .header(header::HOST, format!("{sub}.{APEX}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", ip)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

fn chat_body(question: &str, visitor: &str) -> String {
    json!({"question": question, "visitor": visitor}).to_string()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn off_is_the_default_and_reads_as_not_found() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-off").await;
    let (site, sub) = live_site(&acc, "chatoff").await;

    // A live site whose tenant never switched the assistant on, an
    // explicitly disabled one, and a host that does not exist are the same
    // uniform 404 — no existence leak, fail closed.
    let body = chat_body("What do you sell?", "visitor-aaaa0001");
    let response = post_chat(&state, &sub, "203.0.113.10", &body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "default is off");

    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, false, 500, &month)
        .await
        .unwrap();
    let response = post_chat(&state, &sub, "203.0.113.10", &body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "explicit off");

    let response = post_chat(&state, &unique("ghost"), "203.0.113.10", &body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "unknown host");
}

#[tokio::test]
async fn enabled_and_exhausted_answer_unavailable_with_the_contact_page() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-gate").await;
    let (site, sub) = live_site(&acc, "chatgate").await;
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, true, 100, &month)
        .await
        .unwrap();

    // Within budget: the gate opens, and — until S3.02e wires the answering
    // pipeline — the honest wire state is unavailable, with a human offered.
    let body = chat_body("What do you sell?", "visitor-bbbb0001");
    let response = post_chat(&state, &sub, "203.0.113.20", &body).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let value = json_body(response).await;
    assert_eq!(value["state"], "unavailable");
    assert_eq!(
        value["contactPath"], "/contact",
        "the offer is the site's own published contact page"
    );

    // The ceiling spent: same graceful state on the wire (never a quiet
    // degradation, and exhausted is indistinguishable from unwired — no
    // budget leak to strangers), and the gate underneath really is closed.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url())
        .await
        .unwrap();
    let public = SitePublicStore::new(pool, BlobStore::in_memory(1024 * 1024));
    let resolved = public.resolve_published(&sub).await.unwrap().unwrap();
    assert!(
        public
            .record_chat_spend(&resolved, &month, 100)
            .await
            .unwrap()
    );
    assert_eq!(
        public.chat_gate(&resolved, &month).await.unwrap(),
        alo_store::ChatGate::Exhausted
    );
    let response = post_chat(
        &state,
        &sub,
        "203.0.113.21",
        &chat_body("Hello?", "visitor-bbbb0002"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value = json_body(response).await;
    assert_eq!(value["state"], "unavailable");
    assert_eq!(value["contactPath"], "/contact");
}

#[tokio::test]
async fn a_site_without_a_contact_page_offers_no_path() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-nopath").await;
    let sub = unique("chatnopath");
    let site = acc.create_site("No Path Co", &sub).await.unwrap();
    acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.publish_site(&site).await.unwrap();
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, true, 500, &month)
        .await
        .unwrap();

    let response = post_chat(
        &state,
        &sub,
        "203.0.113.30",
        &chat_body("Anyone there?", "visitor-cccc0001"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value = json_body(response).await;
    assert_eq!(value["state"], "unavailable");
    assert!(
        value["contactPath"].is_null(),
        "no dead link when no contact page is published: {value}"
    );
}

#[tokio::test]
async fn the_visitor_budget_refuses_across_addresses() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-vlimit").await;
    let (site, sub) = live_site(&acc, "chatvlimit").await;
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, true, 1_000, &month)
        .await
        .unwrap();

    // One visitor token, a different address every time: the per-visitor
    // budget is what runs out, exactly at its cap.
    let body = chat_body("Question", "visitor-dddd0001");
    for n in 0..CHAT_VISITOR_MAX_PER_WINDOW {
        let response = post_chat(&state, &sub, &format!("203.0.113.{}", 100 + n), &body).await;
        assert_eq!(response.status(), StatusCode::OK, "question {n} in budget");
    }
    let refused = post_chat(&state, &sub, "203.0.113.90", &body).await;
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        refused.headers().contains_key(header::RETRY_AFTER),
        "the refusal carries a Retry-After hint"
    );

    // Another visitor on one of those same addresses is unaffected.
    let other = post_chat(
        &state,
        &sub,
        "203.0.113.100",
        &chat_body("Question", "visitor-dddd0002"),
    )
    .await;
    assert_eq!(other.status(), StatusCode::OK);
}

#[tokio::test]
async fn the_address_budget_refuses_token_cycling() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-iplimit").await;
    let (site, sub) = live_site(&acc, "chatiplimit").await;
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, true, 1_000, &month)
        .await
        .unwrap();

    // One address cycling a fresh visitor token per request: the looser
    // per-address budget is the bound that actually holds.
    for n in 0..CHAT_IP_MAX_PER_WINDOW {
        let body = chat_body("Question", &format!("visitor-eeee{n:04}"));
        let response = post_chat(&state, &sub, "203.0.113.200", &body).await;
        assert_eq!(response.status(), StatusCode::OK, "question {n} in budget");
    }
    let refused = post_chat(
        &state,
        &sub,
        "203.0.113.200",
        &chat_body("Question", "visitor-eeee9999"),
    )
    .await;
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(refused.headers().contains_key(header::RETRY_AFTER));
}

#[tokio::test]
async fn malformed_input_never_reaches_the_gate() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "chat-input").await;
    let (site, sub) = live_site(&acc, "chatinput").await;
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(&site, true, 500, &month)
        .await
        .unwrap();

    // Broken JSON.
    let response = post_chat(&state, &sub, "203.0.113.40", "{not json").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Visitor tokens: too short, too long, forbidden characters.
    for visitor in ["short", &"v".repeat(65), "spaces are bad!"] {
        let response = post_chat(&state, &sub, "203.0.113.41", &chat_body("Q?", visitor)).await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "token {visitor:?}"
        );
    }

    // Questions: empty, whitespace-only, over the cap.
    for question in ["", "   ", &"q".repeat(CHAT_MAX_QUESTION_CHARS + 1)] {
        let response = post_chat(
            &state,
            &sub,
            "203.0.113.42",
            &chat_body(question, "visitor-ffff0001"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // A body over the route's cap is refused before buffering.
    let oversized = chat_body(&"q".repeat(CHAT_BODY_MAX_BYTES), "visitor-ffff0002");
    let response = post_chat(&state, &sub, "203.0.113.43", &oversized).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
