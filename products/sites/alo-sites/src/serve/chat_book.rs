//! `POST /_alo/chat/book` — taking the appointment the conversation offered
//! (ADR 0040 §2, item S3.03b).
//!
//! This is the deterministic half of booking from the chat: the model only
//! ever *offers* a published service ([`super::chat`] turns that offer into
//! real free times), and the reservation itself is this route — plain code
//! over [`alo_store::SitePublicStore::reserve_public_booking`], the same
//! race-safe write the booking page uses. No model output is anywhere in
//! this path; no number the model invented can reach it.
//!
//! Scoping is the point: the service id posted by the widget only resolves
//! **among the serving site's own published services** — a foreign site's
//! service id, however valid globally, is the same generic 404 as an unknown
//! one, so one site's widget can never book another site's calendar.
//!
//! The wire mirrors `/_alo/chat`: same body cap, same visitor-token shape,
//! same two rate limiters (a booking spends conversation budget). A malformed
//! body is 400, a slot already taken is 409 with `{"state":"taken"}`, a
//! validation miss is 400 with the store's own sentence in `detail` (shown
//! verbatim — the visitor can actually fix it), and success is
//! `{"state":"booked", …}` carrying the confirmation the widget renders: the
//! local time, the `.ics` path that puts the meeting in the visitor's own
//! calendar, and the manage path that can cancel it — reversibility as a
//! link, not a promise.
//!
//! Privacy: the visitor's name and address go into the appointment row and
//! nowhere else; nothing about the connection is stored or logged.

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;

use alo_store::{BookingRequest, StoreError, local_wall_clock};

use super::chat::{rate_limited, state_json, valid_visitor};
use super::forms::client_key;
use super::{AppState, host};

#[derive(Deserialize)]
struct BookBody {
    /// The widget's per-visitor token — rate-limit key only, never stored.
    #[serde(default)]
    visitor: String,
    /// The published service id the conversation offered.
    #[serde(default)]
    service: String,
    /// The exact instant picked, RFC 3339 — one of the offered times.
    #[serde(default)]
    slot: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    email: String,
}

/// Books one offered slot: address limit, parse, visitor limit, host
/// resolution, host-scoped service resolution, then the store's race-safe
/// reservation.
pub(super) async fn book(State(state): State<Arc<AppState>>, request: Request) -> Response {
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

    let body = match Json::<BookBody>::from_request(request, &()).await {
        Ok(Json(body)) => body,
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

    let Some(scope) = host_header.and_then(|value| host::scope(&value, &state.sites_domain)) else {
        return super::not_found(state.unknown_host.clone());
    };
    let resolved = match super::resolve_scope(&state, &scope).await {
        Ok(Some(site)) => site,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat booking resolver read failed");
            return super::unavailable();
        }
    };

    // Resolved among THIS site's published services only: the isolation
    // property of the route (see the module doc).
    let services = match state.store.published_availability(&resolved).await {
        Ok(services) => services,
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat booking services read failed");
            return super::unavailable();
        }
    };
    let Some(service) = services
        .iter()
        .find(|service| service.published.booking_id.as_str() == body.service)
    else {
        return super::not_found(state.unknown_host.clone());
    };

    let Ok(starts_at) = OffsetDateTime::parse(
        body.slot.trim(),
        &time::format_description::well_known::Rfc3339,
    ) else {
        return state_json(StatusCode::BAD_REQUEST, json!({"state": "invalid"}));
    };

    let request = BookingRequest {
        starts_at,
        visitor_name: &body.name,
        visitor_email: &body.email,
        answers: &[],
    };
    match state
        .store
        .reserve_public_booking(service, &request, OffsetDateTime::now_utc())
        .await
    {
        Ok(Some(reserved)) => {
            let when = match local_wall_clock(reserved.starts_at, &reserved.time_zone) {
                Some((day, (hour, minute))) => format!("{day} {hour:02}:{minute:02}"),
                None => body.slot.trim().to_owned(),
            };
            state_json(
                StatusCode::OK,
                json!({
                    "state": "booked",
                    "service": reserved.booking_name,
                    "when": when,
                    "icsPath": format!("/b/manage/{}/calendar.ics", reserved.manage_token),
                    "managePath": format!("/b/manage/{}", reserved.manage_token),
                }),
            )
        }
        Ok(None) => super::not_found(state.unknown_host.clone()),
        Err(StoreError::Validation(detail)) => state_json(
            StatusCode::BAD_REQUEST,
            json!({"state": "invalid", "detail": detail}),
        ),
        Err(StoreError::Conflict(_)) => state_json(StatusCode::CONFLICT, json!({"state": "taken"})),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "chat booking reservation failed");
            super::unavailable()
        }
    }
}
