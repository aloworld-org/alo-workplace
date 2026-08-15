//! In-process integration tests of the assistant's action transcript
//! (ADR 0040, item S3.03e): real questions through the real router (backed
//! by a localhost fixture speaking the OpenAI-compatible wire — never a live
//! model), real leads through `/_alo/chat/lead`, then the tenant's own read
//! of what the assistant did.
//!
//! The load-bearing properties: an answer records its citations — the fact's
//! pages, exactly what the visitor was shown; the free retrieval refusal
//! records nothing (off-topic strangers cannot churn the ledger) while a
//! consulted model's refusal records one entry; a saved and a known lead
//! record their one-word entries; and the serving Host decides whose
//! transcript is written — a stranger tenant sees nothing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteChatActionKind, SiteId, SitePublicStore, Store};

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
        b"chat-actions-tests-analytics-secret",
    );
    (store, state)
}

/// A fresh tenant's account door.
async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@chat-actions.test"))
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

/// A live site whose home page gives the corpus real vocabulary.
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
                          "subheading": "We sell handmade rye bread in Ghent",
                          "image": null,
                          "primary_cta": null, "secondary_cta": null}]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, sub)
}

/// Switches the assistant on with a roomy ceiling.
async fn assistant_on(acc: &AccountStore, site: &SiteId) {
    let month = alo_store::chat_month_key(time::OffsetDateTime::now_utc());
    acc.set_site_chat_settings(site, true, 500, &month)
        .await
        .unwrap();
}

/// A localhost fixture speaking the OpenAI-compatible chat-completions wire.
async fn model_backend(content: &str) -> (String, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let router = axum::Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(
                |axum::extract::State((content, hits)): axum::extract::State<(
                    String,
                    Arc<AtomicUsize>,
                )>| async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    axum::Json(json!({
                        "choices": [{"message": {"role": "assistant", "content": content}}]
                    }))
                },
            ),
        )
        .with_state((content.to_owned(), Arc::clone(&hits)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (base_url, hits)
}

/// Points the tenant's default AI provider at `base_url`.
async fn configure_backend(acc: &AccountStore, base_url: &str) {
    acc.upsert_ai_provider(
        &format!("prov-{}", SiteId::generate().as_str()),
        "openai_compatible",
        "Fixture backend",
        base_url,
        "test-model",
        None,
        true,
    )
    .await
    .unwrap();
    let id = acc.list_ai_providers().await.unwrap()[0].id.clone();
    acc.set_default_ai_provider(&id).await.unwrap();
}

/// One public POST through the real router. `ip` becomes `X-Forwarded-For`.
async fn post_json(
    state: &Arc<AppState>,
    sub: &str,
    path: &str,
    ip: &str,
    body: Value,
) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, format!("{sub}.{APEX}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", ip)
        .body(Body::from(body.to_string()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn an_answer_records_the_pages_the_fact_came_from() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "act-answer").await;
    let (site, sub) = live_site(&acc, "actanswer").await;
    assistant_on(&acc, &site).await;
    let (base_url, _) =
        model_backend(r#"{"answer":"We sell handmade rye bread.","citations":[1]}"#).await;
    configure_backend(&acc, &base_url).await;

    let response = post_json(
        &state,
        &sub,
        "/_alo/chat",
        "203.0.113.80",
        json!({"question": "What do you sell?", "visitor": "visitor-act-0001"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json_body(response).await["state"], "answer");

    // The transcript holds the act, and the fact's source page — the same
    // citation the visitor was shown.
    let transcript = acc.site_chat_actions(&site).await.unwrap();
    assert_eq!(transcript.len(), 1);
    assert_eq!(transcript[0].kind, SiteChatActionKind::Answered);
    assert_eq!(transcript[0].citations.len(), 1);
    assert_eq!(transcript[0].citations[0].title, "Home");
    assert_eq!(transcript[0].citations[0].path.as_deref(), Some("/"));
    assert_eq!(transcript[0].fact, None);
}

#[tokio::test]
async fn free_refusals_record_nothing_and_consulted_refusals_record_one_entry() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "act-refuse").await;
    let (site, sub) = live_site(&acc, "actrefuse").await;
    assistant_on(&acc, &site).await;
    // This backend cannot cite: its answers become refusals.
    let (base_url, hits) = model_backend(r#"{"answer":"Trust me.","citations":[]}"#).await;
    configure_backend(&acc, &base_url).await;

    // Off-topic: refused at retrieval, the model never consulted — and the
    // transcript untouched, so free traffic cannot churn the bounded ledger.
    let response = post_json(
        &state,
        &sub,
        "/_alo/chat",
        "203.0.113.81",
        json!({"question": "quantum flux capacitor maintenance", "visitor": "visitor-act-0002"}),
    )
    .await;
    assert_eq!(json_body(response).await["state"], "refusal");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert!(acc.site_chat_actions(&site).await.unwrap().is_empty());

    // On-topic but uncited: the model was consulted, the refusal is an act
    // the tenant may audit.
    let response = post_json(
        &state,
        &sub,
        "/_alo/chat",
        "203.0.113.82",
        json!({"question": "What do you sell?", "visitor": "visitor-act-0003"}),
    )
    .await;
    assert_eq!(json_body(response).await["state"], "refusal");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    let transcript = acc.site_chat_actions(&site).await.unwrap();
    assert_eq!(transcript.len(), 1);
    assert_eq!(transcript[0].kind, SiteChatActionKind::Refused);
}

#[tokio::test]
async fn leads_record_their_entries_and_only_on_the_serving_hosts_tenant() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "act-lead-a").await;
    let other = fresh_account(&store, "act-lead-b").await;
    let (site, sub) = live_site(&acc, "actleada").await;
    let (site_b, _sub_b) = live_site(&other, "actleadb").await;

    let lead = json!({
        "visitor": "visitor-act-0004",
        "name": "Vera Visitor",
        "email": "vera@newco.example",
        "company": "Newco BV",
    });
    let response = post_json(&state, &sub, "/_alo/chat/lead", "203.0.113.83", lead.clone()).await;
    assert_eq!(json_body(response).await["state"], "lead_saved");
    // The same address again: CRM answers "we know you", and the transcript
    // records that one bit — never which record.
    let response = post_json(&state, &sub, "/_alo/chat/lead", "203.0.113.84", lead).await;
    assert_eq!(json_body(response).await["state"], "lead_known");

    let transcript = acc.site_chat_actions(&site).await.unwrap();
    let kinds: Vec<SiteChatActionKind> = transcript.iter().map(|entry| entry.kind).collect();
    assert_eq!(
        kinds,
        vec![SiteChatActionKind::LeadKnown, SiteChatActionKind::LeadSaved],
        "newest first: known, then saved"
    );
    // Neither entry can say who the visitor was.
    assert!(transcript.iter().all(|entry| entry.fact.is_none()));

    // The serving Host decided the tenant: the stranger's transcript is
    // untouched by any of it.
    assert!(other.site_chat_actions(&site_b).await.unwrap().is_empty());
}
