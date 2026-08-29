//! The S3.04g arc, in-process on the real store: a visitor asks the site's
//! assistant about an event, is offered tickets **at the catalog seam's own
//! price** (the model's canned reply names only an event number — the prompt
//! it saw carried no price to restate), follows the offer to the shop's own
//! checkout, pays on the fixture provider's hosted page, and afterwards the
//! ticket, the CRM contact and the Billing invoice all exist — each through
//! its owning module's door, none written by the conversation itself.
//!
//! The wall beside the arc: the same "tickets" verb on a site with nothing
//! on sale is refused, so a model reply can never index into another
//! tenant's events — the list it is parsed against is the serving Host's
//! and nothing else.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{
    AccountStore, BlobStore, DealFilter, FixtureSitePayments, NewBillingSettings, PipelineSeed,
    SiteChatActionKind, SiteId, SitePaymentProvider, SitePaymentStatus, SitePublicStore, StageSeed,
    Store, TicketFulfilWords,
};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

/// The database this suite runs against.
///
/// Delegates to `alo_test_db`, which refuses the database the product
/// runs on: suites create and drop their own, they never write into `alo`.
fn database_url() -> String {
    alo_test_db::url()
}

/// One tenant's live venue: a published site with a tickets section, one
/// priced product, one upcoming event, the assistant switched on, and the
/// app serving it through the fixture payment provider.
struct Venue {
    account: AccountStore,
    store: Store,
    state: Arc<AppState>,
    provider: Arc<FixtureSitePayments>,
    site: SiteId,
    host: String,
}

async fn venue(tag: &str) -> Venue {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");

    let tenant = store.create_tenant(&format!("tixarc-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("owner@{tag}.tixarc.test"))
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

    // The seller can invoice (the fulfilment act needs a seller country).
    account
        .save_billing_settings(&NewBillingSettings {
            legal_name: "Letterpress BV".to_owned(),
            country: "BE".to_owned(),
            ..NewBillingSettings::default()
        })
        .await
        .unwrap();

    let month = alo_store::chat_month_key(OffsetDateTime::now_utc());
    account
        .set_site_chat_settings(&site, true, 500, &month)
        .await
        .unwrap();

    let provider = Arc::new(FixtureSitePayments::new());
    let payments: Option<Arc<dyn SitePaymentProvider>> =
        Some(Arc::clone(&provider) as Arc<dyn SitePaymentProvider>);
    let state = AppState::with_payments(
        SitePublicStore::new(pool, blobs),
        APEX.to_owned(),
        b"chat-ticket-arc-tests-analytics-key!",
        payments,
    );
    Venue {
        account,
        store,
        state,
        provider,
        site,
        host: format!("{subdomain}.{APEX}"),
    }
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

/// A localhost fixture speaking the OpenAI-compatible chat-completions wire,
/// recording every request body it is shown — the proof of what the model
/// was (and was not) given.
async fn model_backend(content: &str) -> (String, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let router = axum::Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(
                |axum::extract::State((content, seen)): axum::extract::State<(
                    String,
                    Arc<Mutex<Vec<String>>>,
                )>,
                 body: String| async move {
                    seen.lock().unwrap().push(body);
                    axum::Json(json!({
                        "choices": [{"message": {"role": "assistant", "content": content}}]
                    }))
                },
            ),
        )
        .with_state((content.to_owned(), Arc::clone(&seen)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (base_url, seen)
}

/// Points the tenant's default AI provider at `base_url`.
async fn configure_backend(account: &AccountStore, base_url: &str) {
    account
        .upsert_ai_provider(
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
    let id = account.list_ai_providers().await.unwrap()[0].id.clone();
    account.set_default_ai_provider(&id).await.unwrap();
}

async fn get(venue: &Venue, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, venue.host.clone())
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(&venue.state))
        .oneshot(request)
        .await
        .unwrap()
}

async fn post_form(venue: &Venue, path: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, venue.host.clone())
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(&venue.state))
        .oneshot(request)
        .await
        .unwrap()
}

async fn ask(venue: &Venue, client: &str, visitor: &str, question: &str) -> Value {
    let request = Request::builder()
        .method("POST")
        .uri("/_alo/chat")
        .header(header::HOST, venue.host.clone())
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", client)
        .body(Body::from(
            json!({"question": question, "visitor": visitor}).to_string(),
        ))
        .unwrap();
    let response = app(Arc::clone(&venue.state))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

async fn json_body(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
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
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

fn fulfil_words() -> TicketFulfilWords {
    TicketFulfilWords {
        unit: "ticket",
        fallback_item: "Event ticket",
        payment_method: "Hosted checkout",
        crm_title: "Ticket sale",
    }
}

fn crm_seed() -> PipelineSeed {
    PipelineSeed {
        name: "Sales".to_owned(),
        stages: [
            ("New", false, false),
            ("Won", true, false),
            ("Lost", false, true),
        ]
        .into_iter()
        .map(|(name, is_won, is_lost)| StageSeed {
            name: name.to_owned(),
            is_won,
            is_lost,
        })
        .collect(),
    }
}

fn chart_seed() -> alo_store::ChartSeed {
    alo_store::ChartSeed {
        names: alo_store::CHART
            .iter()
            .map(|account| alo_store::ChartName {
                code: account.code.to_owned(),
                name: format!("Account {}", account.code),
            })
            .collect(),
    }
}

/// The whole item in one walk: ask → offer at the seam's price → pay →
/// ticket, contact and invoice.
#[tokio::test]
async fn the_bot_offers_the_seams_price_and_the_sale_makes_everything_exist() {
    let v = venue("arc").await;
    let product = v
        .account
        .create_billing_product(&alo_store::NewProduct {
            name: "Letterpress workshop".to_owned(),
            unit: "seat".to_owned(),
            unit_price_cents: 8_500,
            vat_rate_bp: 2100,
            ..Default::default()
        })
        .await
        .unwrap();
    let event = v
        .account
        .create_site_ticket_event(
            &v.site,
            &product,
            OffsetDateTime::now_utc() + Duration::days(7),
            12,
        )
        .await
        .unwrap();

    // The model's whole contribution is an event *number* — no price, no
    // name, no date. Everything the visitor reads is deterministic code.
    let (base_url, seen) = model_backend(r#"{"tickets":1}"#).await;
    configure_backend(&v.account, &base_url).await;

    let reply = ask(
        &v,
        "203.0.113.90",
        "visitor-tixarc-0001",
        "Can I buy tickets for the letterpress workshop?",
    )
    .await;
    assert_eq!(reply["state"], "tickets", "{reply}");
    assert_eq!(reply["event"]["name"], "Letterpress workshop");
    assert_eq!(reply["event"]["soldOut"], false);
    let price = reply["event"]["price"].as_str().unwrap();
    assert!(
        price.contains("85.00") || price.contains("85,00"),
        "the offer's price is the seam's answer: {price}"
    );
    let offer_path = reply["offerPath"].as_str().unwrap().to_owned();
    assert_eq!(offer_path, format!("/tix/{}", event.as_str()));

    // No price the model invented — provably: the prompt the model was shown
    // named the event (so it could be chosen) and carried no price at all,
    // and its canned reply carried only the number.
    let prompts = seen.lock().unwrap().clone();
    assert_eq!(prompts.len(), 1);
    assert!(
        prompts[0].contains("Ticketed events:"),
        "the event list rode the prompt"
    );
    assert!(prompts[0].contains("Letterpress workshop —"));
    for price_shape in ["85.00", "85,00", "8500", "€"] {
        assert!(
            !prompts[0].contains(price_shape),
            "a price shape leaked into the prompt: {price_shape}"
        );
    }

    // The tenant's transcript holds the offer, with the same label the
    // visitor and the model saw — never the visitor or their words.
    let transcript = v.account.site_chat_actions(&v.site).await.unwrap();
    let offered = transcript
        .iter()
        .find(|action| action.kind == SiteChatActionKind::TicketsOffered)
        .expect("the offer is on the transcript");
    assert!(
        offered
            .fact
            .as_deref()
            .unwrap()
            .starts_with("Letterpress workshop — "),
        "{:?}",
        offered.fact
    );

    // The offer page the reply pointed at: the same price, and the buy form.
    let offer = get(&v, &offer_path).await;
    assert_eq!(offer.status(), StatusCode::OK);
    let page = body_string(offer).await;
    assert!(page.contains("85.00") || page.contains("85,00"), "{page}");
    assert!(page.contains("name=\"quantity\""), "{page}");

    // The visitor buys a seat: 303 to the provider's hosted page, pays
    // there, and the webhook rings.
    let bought = post_form(
        &v,
        &offer_path,
        "203.0.113.91",
        "quantity=1&name=Vera+Visitor&email=vera%40tixarc.example&website=",
    )
    .await;
    assert_eq!(bought.status(), StatusCode::SEE_OTHER);
    let checkout_url = header_value(&bought, header::LOCATION);
    let payment_id = checkout_url.rsplit('/').next().unwrap().to_owned();
    let order_id = payment_id.strip_prefix("fixpay-").unwrap().to_owned();
    v.provider
        .mark(&payment_id, SitePaymentStatus::Paid)
        .unwrap();
    let rung = post_form(&v, "/_alo/pay", "203.0.113.92", &format!("id={payment_id}")).await;
    assert_eq!(rung.status(), StatusCode::OK);

    // Fulfilment makes the sale good. The sweep may claim other suites'
    // paid orders from the shared database; only ours is acted on here.
    let claim = 'claim: {
        for _ in 0..100 {
            let claims = v.store.claim_ticket_fulfilments(100).await.unwrap();
            let found = claims
                .into_iter()
                .find(|claim| claim.order.as_str() == order_id);
            if let Some(claim) = found {
                break 'claim claim;
            }
        }
        panic!("the paid order was never offered to the sweep");
    };
    let outcome = v
        .store
        .fulfil_claimed_ticket(&claim, &fulfil_words(), &crm_seed(), &chart_seed())
        .await
        .unwrap();
    assert!(outcome.invoiced, "the invoice exists");
    assert!(outcome.lead_raised, "the contact exists");

    // The ticket exists and names the buyer.
    let done = get(&v, &format!("/tix/order/{order_id}")).await;
    let page = body_string(done).await;
    let token = page
        .split("href=\"/t/")
        .nth(1)
        .map(|rest| rest.split('"').next().unwrap().to_owned())
        .expect("the return page links the ticket");
    let ticket = body_string(get(&v, &format!("/t/{token}")).await).await;
    assert!(ticket.contains("Vera Visitor"), "{ticket}");

    // The invoice is Billing's document: referencing the order, settled by
    // the hosted payment, VAT carved out of the consumer price.
    let invoices = v.account.billing_invoices(None).await.unwrap();
    let summary = invoices
        .iter()
        .find(|summary| summary.invoice.reference == order_id)
        .expect("the sale has an invoice");
    assert!(summary.invoice.number.is_some());
    assert_eq!(summary.paid_cents, 8_500);
    assert!(summary.totals.vat_cents > 0);

    // The contact is CRM's card, raised through CRM's own seam.
    let deals = v.account.crm_deals(&DealFilter::default()).await.unwrap();
    assert_eq!(deals.len(), 1);
    assert_eq!(deals[0].title, "Ticket sale — Venue");
}

/// The wall: on a site with nothing on sale, the same model verb is refused —
/// a reply can only ever index into the serving Host's own event list.
#[tokio::test]
async fn a_tickets_verb_with_nothing_on_sale_is_refused() {
    let v = venue("wall").await;
    let (base_url, _) = model_backend(r#"{"tickets":1}"#).await;
    configure_backend(&v.account, &base_url).await;

    let reply = ask(
        &v,
        "203.0.113.93",
        "visitor-tixarc-0002",
        "Can I buy tickets for the evenings at the press?",
    )
    .await;
    assert_eq!(reply["state"], "refusal", "{reply}");
}
