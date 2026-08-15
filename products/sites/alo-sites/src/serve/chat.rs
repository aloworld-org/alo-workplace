//! `POST /_alo/chat` — the visitor assistant's public endpoint, shipped
//! gates-first (ADR 0040 §3, item S3.02c): before any model is ever contacted
//! this path enforces the per-IP and per-visitor rate limits, the site's
//! enable switch (off by default, absence reads as off), and the monthly
//! spending ceiling. At the ceiling the assistant does not degrade quietly —
//! the response is a typed `unavailable` carrying the path of the site's own
//! published contact page, so the widget can offer a human instead.
//!
//! The answering pipeline itself (retrieval + the model call built in
//! `alo-ai::site_chat`) is wired in the widget slice (S3.02e): until then a
//! question that passes every gate receives the same honest `unavailable`,
//! because with no configured model the assistant *is* unavailable. That
//! branch — and only that branch — is where the pipeline call lands.
//!
//! Wire contract: an unknown host and a site whose assistant is off are the
//! same generic 404 as any other miss (no existence leak). A malformed body,
//! an invalid visitor token, and an empty or oversized question are 400 (413
//! when the body itself exceeds the route's cap); a rate-limited client is
//! 429 with `Retry-After`. Everything else is 200 with a JSON state object,
//! `no-store`. Privacy: the visitor token and client address key the
//! in-memory limiters transiently ([`super::rate`]) and are never stored or
//! logged.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use alo_store::{ChatGate, PublishedSite, chat_month_key};
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

    if !VISITOR_TOKEN_CHARS.contains(&body.visitor.chars().count())
        || !body
            .visitor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
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
        // The ceiling is spent, and — until S3.02e wires the answering
        // pipeline into the Ready arm below — an unconfigured model is the
        // same honest state: unavailable, with a human offered instead.
        ChatGate::Exhausted | ChatGate::Ready { .. } => {
            let contact_path = contact_path(&state, &resolved).await;
            state_json(
                StatusCode::OK,
                json!({"state": "unavailable", "contactPath": contact_path}),
            )
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

/// The 429, with the limiter's `Retry-After` hint in seconds.
fn rate_limited(wait_seconds: u64) -> Response {
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
fn state_json(status: StatusCode, body: serde_json::Value) -> Response {
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
