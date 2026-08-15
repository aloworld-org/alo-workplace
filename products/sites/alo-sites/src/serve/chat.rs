//! `POST /_alo/chat` — the visitor assistant's public endpoint, shipped
//! gates-first (ADR 0040 §3, item S3.02c): before any model is ever contacted
//! this path enforces the per-IP and per-visitor rate limits, the site's
//! enable switch (off by default, absence reads as off), and the monthly
//! spending ceiling. At the ceiling the assistant does not degrade quietly —
//! the response is a typed `unavailable` carrying the path of the site's own
//! published contact page, so the widget can offer a human instead.
//!
//! Past the gates sits the answering pipeline (S3.02e): deterministic
//! retrieval over the site's published grounding corpus plus the tenant's
//! own configured model backend (`alo-ai::site_chat`), held to ADR 0040 §1's
//! rule — an answer that cannot cite is a refusal, and every delivered
//! answer names the published pages it came from. Spend is recorded against
//! the site's monthly ceiling only when the model was actually contacted: an
//! off-topic question refuses at retrieval and costs the tenant nothing.
//!
//! Wire contract: an unknown host and a site whose assistant is off are the
//! same generic 404 as any other miss (no existence leak). A malformed body,
//! an invalid visitor token, and an empty or oversized question are 400 (413
//! when the body itself exceeds the route's cap); a rate-limited client is
//! 429 with `Retry-After`. Everything else is 200 with a JSON state object,
//! `no-store`:
//!
//! - `{"state":"answer","text":…,"citations":[{"title":…,"path":…}]}` — a
//!   cited answer; `path` is site-relative, or `null` for a knowledge
//!   document (which has no public URL and is named by title alone).
//! - `{"state":"booking","service":…,"direct":…,"formPath":…,"days":[…]}` —
//!   the visitor asked to book and the model offered one of the site's own
//!   published services (S3.03b): the reply carries that service and its
//!   next real free times, read live from the Agenda seam. The reservation
//!   itself is `POST /_alo/chat/book` ([`super::chat_book`]) — the model can
//!   offer, never act.
//! - `{"state":"lead"}` — the visitor asked to be contacted (S3.03d): the
//!   widget shows its own name-and-address form, and the capture itself is
//!   `POST /_alo/chat/lead` ([`super::chat_lead`]) — again offer, never act.
//! - `{"state":"refusal","contactPath":…}` — the assistant will not answer
//!   this question (nothing retrievable, an uncited reply, a booking offer
//!   for a service never listed, or the model's own refusal — deliberately
//!   indistinguishable to a stranger).
//! - `{"state":"unavailable","contactPath":…}` — the ceiling is spent, no
//!   backend is configured, or the backend failed; the widget offers the
//!   site's contact page instead.
//!
//! Privacy: the visitor token and client address key the in-memory limiters
//! transiently ([`super::rate`]) and are never stored or logged; the
//! question itself is sent to the tenant's configured backend and nowhere
//! else, and is never logged either. What the tenant can audit afterwards is
//! the action transcript (S3.03e, [`alo_store::site_chat_actions`]): each
//! act or offer with the published fact it used and the page that fact came
//! from — never the conversation's words, never the visitor.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use alo_ai::{SiteChatError, SiteChatRefusal, SiteChatReply, SiteChatVoice};
use alo_store::{
    ChatActionCitation, ChatGate, NewChatAction, PublicBookingService, PublishedSite,
    SiteChatAppearance, chat_month_key, local_day, local_wall_clock,
};
use time::OffsetDateTime;

use super::forms::client_key;
use super::{AppState, host};

/// The most bytes a chat request body may carry: the question cap at
/// worst-case UTF-8 plus JSON framing and the visitor token, with headroom.
pub const CHAT_BODY_MAX_BYTES: usize = 16 * 1024;

/// The most characters a visitor's question may carry — mirrors
/// `alo-ai`'s question cap: anonymous input feeds a metered model call.
pub const CHAT_MAX_QUESTION_CHARS: usize = 2_000;

/// Questions one visitor token may ask per [`CHAT_VISITOR_WINDOW`]. A real
/// conversation stays well under it; a script gains nothing by holding one
/// token.
pub const CHAT_VISITOR_MAX_PER_WINDOW: usize = 15;
/// The sliding window visitor questions are counted in.
pub const CHAT_VISITOR_WINDOW: Duration = Duration::from_secs(600);

/// Questions one client address may ask per [`CHAT_IP_WINDOW`] — looser than
/// the visitor budget because an office behind one NAT address is many
/// visitors, and it is the bound a token-cycling script actually hits.
pub const CHAT_IP_MAX_PER_WINDOW: usize = 60;
/// The sliding window per-address questions are counted in.
pub const CHAT_IP_WINDOW: Duration = Duration::from_secs(600);

/// A visitor token is client-generated and opaque; the only thing required
/// of it is a bounded, header-safe shape.
const VISITOR_TOKEN_CHARS: std::ops::RangeInclusive<usize> = 8..=64;

#[derive(Deserialize)]
struct ChatBody {
    #[serde(default)]
    question: String,
    /// The widget's random per-visitor token: rate-limit key only, never
    /// stored (the privacy model keeps identity out of this surface).
    #[serde(default)]
    visitor: String,
}

/// Handles one visitor question end to end: address limit, parse, visitor
/// limit, host resolution, then the enable/ceiling gate.
pub(super) async fn ask(State(state): State<Arc<AppState>>, request: Request) -> Response {
    if let Err(wait) = state
        .chat_ip_rate
        .allow(&client_key(&request), Instant::now())
    {
        return rate_limited(wait);
    }

    let host_header = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let body = match Json::<ChatBody>::from_request(request, &()).await {
        Ok(Json(body)) => body,
        // The body never buffered (over the route's cap): 413. Any other
        // rejection — wrong content type, broken JSON — is one terse 400.
        Err(JsonRejection::BytesRejection(_)) => {
            return state_json(StatusCode::PAYLOAD_TOO_LARGE, json!({"state": "invalid"}));
        }
        Err(_) => return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"})),
    };

    if !valid_visitor(&body.visitor) {
        return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}));
    }
    if let Err(wait) = state.chat_visitor_rate.allow(&body.visitor, Instant::now()) {
        return rate_limited(wait);
    }

    let question = body.question.trim();
    if question.is_empty() || question.chars().count() > CHAT_MAX_QUESTION_CHARS {
        return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}));
    }

    let Some(scope) = host_header.and_then(|value| host::scope(&value, &state.sites_domain)) else {
        return super::not_found(state.unknown_host.clone());
    };
    let resolved = match super::resolve_scope(&state, &scope).await {
        Ok(Some(site)) => site,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat resolver read failed");
            return super::unavailable();
        }
    };

    let month = chat_month_key(OffsetDateTime::now_utc());
    let gate = match state.store.chat_gate(&resolved, &month).await {
        Ok(gate) => gate,
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat gate read failed");
            return super::unavailable();
        }
    };
    match gate {
        // Off is indistinguishable from nonexistent — the widget only ships
        // on sites whose assistant is on, so a 404 here is only ever a probe.
        ChatGate::Disabled => super::not_found(state.unknown_host.clone()),
        // The ceiling is spent: unavailable, with a human offered instead
        // (never a quiet degradation — ADR 0040 §3).
        ChatGate::Exhausted => unavailable_state(&state, &resolved).await,
        ChatGate::Ready { .. } => answer(&state, &resolved, question, &month).await,
    }
}

/// What one answered question costs the tenant's monthly ceiling, as an
/// estimate in euro cents: the ceiling is spend, not tokens (ADR 0040 §3),
/// and the OpenAI-compatible wire reports no price — so the estimate errs
/// simple and predictable. The €10.00 default ceiling buys ~1000 answers a
/// month; an operator with real per-token prices can revisit the constant.
pub const CHAT_SPEND_PER_QUESTION_CENTS: i64 = 1;

/// The Ready arm: retrieval over the published corpus, the tenant's own
/// model backend, and the citation rule — with spend recorded exactly when
/// the backend was actually contacted.
async fn answer(
    state: &Arc<AppState>,
    site: &PublishedSite,
    question: &str,
    month: &str,
) -> Response {
    let config = match state.store.tenant_ai_config(site).await {
        Ok(Some(row)) => alo_ai::AiConfig {
            base_url: row.base_url,
            model: row.model,
            api_key: row.api_key,
            enabled: row.enabled,
        },
        // No configured backend: the assistant is honestly unavailable.
        Ok(None) => return unavailable_state(state, site).await,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "chat backend config read failed");
            return unavailable_state(state, site).await;
        }
    };
    let corpus = match state.store.site_grounding_corpus(site).await {
        Ok(corpus) => corpus,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "chat corpus read failed");
            return unavailable_state(state, site).await;
        }
    };
    // What can be booked here (S3.03b): the site's published services, whose
    // names — and nothing else — the model is shown so it can *offer* one.
    // A failed read answers without booking rather than not at all.
    let services = match state.store.published_availability(site).await {
        Ok(services) => services,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "chat bookable services read failed");
            Vec::new()
        }
    };
    let service_names: Vec<String> = services
        .iter()
        .map(|service| service.published.name.clone())
        .collect();
    // The tenant's voice (ADR 0040 §5): tone and note shape the prompt's
    // style guidance — the answering rules stay absolute in `alo-ai`. A
    // failed read answers in the default voice rather than not at all.
    let appearance = chat_appearance(state, site).await;
    let voice = SiteChatVoice {
        tone: appearance.tone,
        note: appearance.tone_note.as_deref(),
    };
    match alo_ai::answer_site_question(&config, question, &corpus, &service_names, &voice).await {
        Ok(SiteChatReply::Answer { text, citations }) => {
            record_spend(state, site, month).await;
            // The fact's sources, resolved once: what the wire cites is
            // exactly what the tenant's transcript records (S3.03e).
            let cited: Vec<ChatActionCitation> = citations
                .iter()
                .map(|citation| ChatActionCitation {
                    title: citation.title.clone(),
                    path: alo_ai::citation_path(&citation.citation, &site.default_locale),
                })
                .collect();
            record_action(state, site, &NewChatAction::answered(&cited)).await;
            let citations: Vec<serde_json::Value> = cited
                .iter()
                .map(|citation| json!({"title": citation.title, "path": citation.path}))
                .collect();
            state_json(
                StatusCode::OK,
                json!({"state": "answer", "text": text, "citations": citations}),
            )
        }
        // The model may only *offer* a booking (ADR 0040 §2, S3.03b): the
        // service is one of the published list by construction of the parse,
        // and everything the visitor sees next — the free times — is read
        // from Agenda's seam here, never from the model.
        Ok(SiteChatReply::OfferBooking { service }) => {
            record_spend(state, site, month).await;
            match services.get(service - 1) {
                Some(service) => {
                    record_action(
                        state,
                        site,
                        &NewChatAction::booking_offered(&service.published.name),
                    )
                    .await;
                    booking_state(state, service).await
                }
                // The list changed between the read and the reply.
                None => unavailable_state(state, site).await,
            }
        }
        // The visitor asked to be contacted (S3.03d): the widget shows its
        // own details form and the capture is `POST /_alo/chat/lead` — the
        // model offers, deterministic code acts, exactly as with booking.
        // The offer itself is counted in the site's aggregate 'chat'
        // funnel: a server-observed fact with no visitor in it.
        Ok(SiteChatReply::OfferLead) => {
            record_spend(state, site, month).await;
            record_lead_offer(state, site).await;
            record_action(state, site, &NewChatAction::lead_offered()).await;
            state_json(StatusCode::OK, json!({"state": "lead"}))
        }
        Ok(SiteChatReply::Refusal(refusal)) => {
            // An off-topic question refused at retrieval never reached the
            // backend and costs nothing; the model's own refusals did.
            // The transcript follows the same line (S3.03e): a consulted
            // model's refusal is an act worth showing the tenant, while the
            // free retrieval refusal would let off-topic strangers churn
            // the bounded ledger for nothing.
            if !matches!(refusal, SiteChatRefusal::NoSources) {
                record_spend(state, site, month).await;
                record_action(state, site, &NewChatAction::refused()).await;
            }
            let contact_path = contact_path(state, site).await;
            state_json(
                StatusCode::OK,
                json!({"state": "refusal", "contactPath": contact_path}),
            )
        }
        // Belt over the route's own validation.
        Err(SiteChatError::EmptyQuestion | SiteChatError::QuestionTooLong) => {
            state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}))
        }
        // Disabled, unconfigured, or unreachable: nothing was billed.
        Err(SiteChatError::Inference(error)) => {
            tracing::warn!(site = %site.site, %error, "chat backend call failed");
            unavailable_state(state, site).await
        }
        // The backend answered (and billed) but out of contract.
        Err(error) => {
            tracing::warn!(site = %site.site, %error, "chat reply was out of contract");
            record_spend(state, site, month).await;
            unavailable_state(state, site).await
        }
    }
}

/// How many days ahead the conversation offers free times from, capped under
/// the service's own horizon: enough to book this week, small enough that the
/// reply stays a chat bubble. The full week-by-week view is one click away on
/// the service's own booking page.
const BOOKING_OFFER_DAYS: i64 = 14;
/// At most this many days with free time are offered in the conversation.
const BOOKING_OFFER_MAX_DAYS: usize = 3;
/// At most this many times per offered day.
const BOOKING_OFFER_MAX_TIMES: usize = 4;

/// The `booking` state: one published service the visitor may book from the
/// conversation, with its next real free times. `direct` is `false` when the
/// service asks required questions of its own — those are answered on its
/// booking page (linked as `formPath`), not squeezed into a chat bubble.
pub(super) async fn booking_state(
    state: &Arc<AppState>,
    service: &PublicBookingService,
) -> Response {
    let now = OffsetDateTime::now_utc();
    let zone = &service.published.time_zone;
    let mut days: Vec<serde_json::Value> = Vec::new();
    let horizon = i64::from(service.published.horizon_days.max(0)).min(BOOKING_OFFER_DAYS);
    let today = local_day(now, zone).unwrap_or_else(|| now.date());
    for offset in 0..=horizon {
        if days.len() >= BOOKING_OFFER_MAX_DAYS {
            break;
        }
        let day = today.saturating_add(time::Duration::days(offset));
        let slots = match state.store.public_booking_slots(service, day, now).await {
            Ok(slots) => slots,
            Err(error) => {
                tracing::error!(%error, "chat booking slots read failed");
                break;
            }
        };
        if slots.is_empty() {
            continue;
        }
        let times: Vec<serde_json::Value> = slots
            .iter()
            .take(BOOKING_OFFER_MAX_TIMES)
            .filter_map(|slot| {
                let at = slot
                    .starts_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .ok()?;
                let label = match local_wall_clock(slot.starts_at, zone) {
                    Some((_, (hour, minute))) => format!("{hour:02}:{minute:02}"),
                    None => at.clone(),
                };
                Some(json!({"at": at, "label": label}))
            })
            .collect();
        days.push(json!({"date": day.to_string(), "times": times}));
    }
    let direct = !service.published.fields.iter().any(|field| field.required);
    state_json(
        StatusCode::OK,
        json!({
            "state": "booking",
            "service": {
                "id": service.published.booking_id.as_str(),
                "name": service.published.name,
                "minutes": service.published.duration_minutes,
            },
            "direct": direct,
            "formPath": format!("/b/{}", service.published.booking_id.as_str()),
            "days": days,
        }),
    )
}

/// The typed `unavailable`, always carrying the site's own contact page when
/// it has one — the graceful refusal ADR 0040 §3 requires.
async fn unavailable_state(state: &Arc<AppState>, site: &PublishedSite) -> Response {
    let contact_path = contact_path(state, site).await;
    state_json(
        StatusCode::OK,
        json!({"state": "unavailable", "contactPath": contact_path}),
    )
}

/// Adds one question's estimated cost to the site's monthly ledger. A failed
/// write is logged and never fails the visitor's answer: the ceiling is an
/// abuse bound, not an accounting system, and the gate re-reads live truth
/// on the next question.
async fn record_spend(state: &Arc<AppState>, site: &PublishedSite, month: &str) {
    if let Err(error) = state
        .store
        .record_chat_spend(site, month, CHAT_SPEND_PER_QUESTION_CENTS)
        .await
    {
        tracing::error!(site = %site.site, %error, "chat spend record failed");
    }
}

/// Appends one entry to the tenant's transcript of what the assistant did
/// (S3.03e). Logged and never failed on: the transcript is the tenant's
/// accountability surface, never part of the visitor's answer. What it
/// records is the act and the tenant's own published facts — the type
/// cannot carry the question, the answer text, or the visitor.
pub(super) async fn record_action(
    state: &Arc<AppState>,
    site: &PublishedSite,
    action: &NewChatAction,
) {
    if let Err(error) = state.store.record_chat_action(site, action).await {
        tracing::error!(site = %site.site, %error, "chat action record failed");
    }
}

/// One more offered lead form in the site's aggregate 'chat' counters
/// (S3.03d). Logged and never failed on: the counter is attribution, not the
/// conversation.
async fn record_lead_offer(state: &Arc<AppState>, site: &PublishedSite) {
    if let Err(error) = state
        .store
        .record_public_chat_conversion(
            site,
            OffsetDateTime::now_utc().date(),
            alo_store::ConversionStage::View,
        )
        .await
    {
        tracing::error!(site = %site.site, %error, "chat lead offer count failed");
    }
}

/// The site's assistant appearance, defaults on a failed read — the page
/// (and the answer) always come first.
async fn chat_appearance(state: &Arc<AppState>, site: &PublishedSite) -> SiteChatAppearance {
    match state.store.chat_appearance(site).await {
        Ok(appearance) => appearance,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "chat appearance read failed");
            SiteChatAppearance::default()
        }
    }
}

/// Whether `site`'s published pages should carry the assistant widget right
/// now: yes for an assistant that is on — including one whose ceiling is
/// currently spent, which must say it is unavailable rather than vanish —
/// and no bytes at all otherwise. Enablement and appearance are live state
/// read per request (like page protection), never frozen with the publish. A
/// read failure serves the page without the widget: the page always comes
/// first.
pub(super) async fn widget_if_on(
    state: &Arc<AppState>,
    site: &PublishedSite,
    locale: &str,
) -> Option<String> {
    let month = chat_month_key(OffsetDateTime::now_utc());
    match state.store.chat_gate(site, &month).await {
        Ok(ChatGate::Disabled) => None,
        Ok(ChatGate::Ready { .. } | ChatGate::Exhausted) => {
            let appearance = chat_appearance(state, site).await;
            Some(super::widget::fragment(
                crate::render::strings_for(locale),
                &appearance,
            ))
        }
        Err(error) => {
            tracing::error!(site = %site.site, %error, "chat widget gate read failed");
            None
        }
    }
}

/// The path of the first published default-locale page carrying a
/// `contact_form` section, in navigation order — the concrete "offer the
/// contact form" of ADR 0040 §3. `None` when the site published no contact
/// page; the widget then offers nothing rather than a dead link.
async fn contact_path(state: &AppState, site: &PublishedSite) -> Option<String> {
    let snapshots = match state.store.published_pages(site).await {
        Ok(snapshots) => snapshots,
        Err(error) => {
            tracing::warn!(%error, "chat contact-path read failed");
            return None;
        }
    };
    snapshots
        .iter()
        .filter(|page| page.locale == site.default_locale)
        .find(|page| {
            page.sections["sections"]
                .as_array()
                .is_some_and(|sections| {
                    sections
                        .iter()
                        .any(|section| section["type"] == "contact_form")
                })
        })
        .map(|page| {
            if page.is_home {
                "/".to_owned()
            } else {
                format!("/{}", page.slug)
            }
        })
}

/// Whether a widget-minted visitor token has the only shape the wire
/// accepts: bounded, header-safe, and unable to carry anything but noise.
pub(super) fn valid_visitor(token: &str) -> bool {
    VISITOR_TOKEN_CHARS.contains(&token.chars().count())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The 429, with the limiter's `Retry-After` hint in seconds.
pub(super) fn rate_limited(wait_seconds: u64) -> Response {
    let mut response = state_json(
        StatusCode::TOO_MANY_REQUESTS,
        json!({"state": "rate_limited"}),
    );
    if let Ok(value) = HeaderValue::from_str(&wait_seconds.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

/// One JSON state object, uncacheable — the scripted widget is the only
/// consumer of this path.
pub(super) fn state_json(status: StatusCode, body: serde_json::Value) -> Response {
    (
        status,
        [
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        Json(body),
    )
        .into_response()
}
