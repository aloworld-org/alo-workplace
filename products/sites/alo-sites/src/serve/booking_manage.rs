//! `/b/manage/{token}` — the visitor's own view of one appointment: seeing
//! it, putting it in their calendar, and cancelling it (item S3.03b).
//!
//! The token is the capability [`alo_store::site_booking_manage`] minted with
//! the reservation; it travels only in the visitor's confirmation (the chat
//! card, the confirmation page, the `.ics` description). Three routes hang
//! off it, all resolved **on the serving site only** — a token minted on one
//! site is the generic 404 on every other host:
//!
//! - `GET  /b/manage/{token}` — the appointment page: what was booked and
//!   when, the calendar download, and the cancel button while it still
//!   stands. Cancellation is a POST behind a real button, never a GET side
//!   effect: mail scanners and link previewers follow GETs.
//! - `POST /b/manage/{token}/cancel` — withdraws the reservation; the slot
//!   is free again and the owner's calendar event is removed. Told honestly
//!   when it was already cancelled or has already started.
//! - `GET  /b/manage/{token}/calendar.ics` — the appointment as an RFC 5545
//!   document the visitor's own calendar can import; its description carries
//!   the cancellation link, so the undo travels with the meeting itself.
//!
//! Everything here is `no-store` and unindexed: a confirmation is nobody
//! else's, and free time changes by the minute.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use time::OffsetDateTime;

use alo_store::{CancelOutcome, ManagedAppointment, PublishedSite, local_wall_clock};

use crate::render::html::esc;
use crate::render::{UiStrings, strings_for};

use super::AppState;
use super::ics::{ics_escape, ics_fold, ics_time};
use super::rendered::minimal_page;

/// The appointment page.
pub(super) async fn show(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some((site, appointment)) = resolve(&state, host.as_deref(), &token).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let when = local_stamp(&appointment);
    if !appointment.booked {
        let body = format!(
            "<p>{} — {}</p>\n<p>{}</p>\n",
            esc(&appointment.booking_name),
            esc(&when),
            esc(strings.booking_manage_already_text)
        );
        return page(
            StatusCode::OK,
            strings,
            strings.booking_manage_cancelled_title,
            &body,
        );
    }
    let mut body = format!(
        "<p>{} — {}</p>\n<p><a href=\"/b/manage/{token}/calendar.ics\">{}</a></p>\n",
        esc(&appointment.booking_name),
        esc(&when),
        esc(strings.booking_add_calendar),
        token = esc(&token),
    );
    if appointment.starts_at > OffsetDateTime::now_utc() {
        body.push_str(&format!(
            "<form action=\"/b/manage/{}/cancel\" method=\"post\">\
             <p><button type=\"submit\">{}</button></p></form>\n",
            esc(&token),
            esc(strings.booking_cancel)
        ));
    }
    page(StatusCode::OK, strings, strings.booking_manage_title, &body)
}

/// The cancellation itself.
pub(super) async fn cancel(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    // The same public-write limiter as the forms and the booking POST: a
    // cancel is rare, so a burst of them is only ever a script. English for
    // this one page — the limiter answers before any site is resolved.
    if let Err(wait) = state
        .rate
        .allow(&super::forms::client_key(&request), Instant::now())
    {
        let strings = &crate::render::EN;
        let mut response = page(
            StatusCode::TOO_MANY_REQUESTS,
            strings,
            strings.booking_rate_limited_title,
            &format!("<p>{}</p>\n", esc(strings.booking_rate_limited_text)),
        );
        if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return response;
    }
    let Some(site) = resolve_site(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let outcome = match state
        .store
        .cancel_managed_appointment(&site, &token, OffsetDateTime::now_utc())
        .await
    {
        Ok(Some(outcome)) => outcome,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(%error, "appointment cancellation failed");
            return super::unavailable();
        }
    };
    match outcome {
        CancelOutcome::Cancelled { booking_name } => {
            let body = format!(
                "<p>{}</p>\n<p>{}</p>\n",
                esc(&booking_name),
                esc(strings.booking_manage_cancelled_text)
            );
            page(
                StatusCode::OK,
                strings,
                strings.booking_manage_cancelled_title,
                &body,
            )
        }
        CancelOutcome::AlreadyCancelled { booking_name } => {
            let body = format!(
                "<p>{}</p>\n<p>{}</p>\n",
                esc(&booking_name),
                esc(strings.booking_manage_already_text)
            );
            page(
                StatusCode::OK,
                strings,
                strings.booking_manage_cancelled_title,
                &body,
            )
        }
        CancelOutcome::TooLate { booking_name } => {
            let body = format!(
                "<p>{}</p>\n<p>{}</p>\n",
                esc(&booking_name),
                esc(strings.booking_manage_too_late_text)
            );
            page(
                StatusCode::CONFLICT,
                strings,
                strings.booking_manage_title,
                &body,
            )
        }
    }
}

/// The appointment as an iCalendar document, while the reservation stands —
/// a cancelled one has nothing to import and answers the uniform 404.
pub(super) async fn calendar(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some((_, appointment)) = resolve(&state, host.as_deref(), &token).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let host = host.unwrap_or_default();
    if !appointment.booked {
        return super::not_found(state.unknown_host.clone());
    }
    let document = ics_document(&appointment, &host, &token, OffsetDateTime::now_utc());
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/calendar; charset=utf-8"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"appointment.ics\""),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        document,
    )
        .into_response()
}

/// The raw Host header, read before any await — the request body is not
/// `Sync`, so nothing here may hold the request across one.
fn host_header(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// Host → published site, one uniform absence for every kind of miss.
async fn resolve_site(state: &Arc<AppState>, host: Option<&str>) -> Option<PublishedSite> {
    let scope = super::host::scope(host?, &state.sites_domain)?;
    match super::resolve_scope(state, &scope).await {
        Ok(site) => site,
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "manage resolver read failed");
            None
        }
    }
}

/// Host + token → the one appointment the token stands for **on this site**.
async fn resolve(
    state: &Arc<AppState>,
    host: Option<&str>,
    token: &str,
) -> Option<(PublishedSite, ManagedAppointment)> {
    let site = resolve_site(state, host).await?;
    match state.store.managed_appointment(&site, token).await {
        Ok(Some(appointment)) => Some((site, appointment)),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(%error, "managed appointment read failed");
            None
        }
    }
}

/// The appointment's start as a day and wall clock in its own published
/// zone, falling back to the UTC clock — never blank.
fn local_stamp(appointment: &ManagedAppointment) -> String {
    match local_wall_clock(appointment.starts_at, &appointment.time_zone) {
        Some((day, (hour, minute))) => format!("{day} {hour:02}:{minute:02}"),
        None => appointment
            .starts_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }
}

/// One complete RFC 5545 document for one appointment. The description and
/// URL both carry the manage link, so the cancellation travels **inside**
/// the visitor's own calendar entry.
fn ics_document(
    appointment: &ManagedAppointment,
    host: &str,
    token: &str,
    now: OffsetDateTime,
) -> String {
    let manage = format!("https://{host}/b/manage/{token}");
    let mut out = String::new();
    for line in [
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//alo//sites//EN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "METHOD:PUBLISH".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{}@sites.alo", appointment.id.as_str()),
        format!("DTSTAMP:{}", ics_time(now)),
        format!("DTSTART:{}", ics_time(appointment.starts_at)),
        format!("DTEND:{}", ics_time(appointment.ends_at)),
        format!("SUMMARY:{}", ics_escape(&appointment.booking_name)),
        format!("DESCRIPTION:{}", ics_escape(&format!("Cancel: {manage}"))),
        format!("URL:{}", ics_escape(&manage)),
        "END:VEVENT".to_owned(),
        "END:VCALENDAR".to_owned(),
    ] {
        out.push_str(&ics_fold(&line));
        out.push_str("\r\n");
    }
    out
}

/// A manage document: never cacheable, never indexed.
fn page(status: StatusCode, strings: &UiStrings, title: &str, body: &str) -> Response {
    (
        status,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        minimal_page(strings.lang, title, body),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use alo_store::SiteBookingAppointmentId;

    use super::*;

    fn appointment() -> ManagedAppointment {
        ManagedAppointment {
            id: SiteBookingAppointmentId::new("appt-1".to_owned()),
            booking_name: "Consultation; morning, with Ada\\Bob".to_owned(),
            starts_at: time::macros::datetime!(2026-09-16 07:00 UTC),
            ends_at: time::macros::datetime!(2026-09-16 07:30 UTC),
            time_zone: "Europe/Brussels".to_owned(),
            booked: true,
        }
    }

    #[test]
    fn the_document_is_folded_crlf_and_carries_the_cancel_link() {
        let now = time::macros::datetime!(2026-09-01 08:00 UTC);
        let doc = ics_document(&appointment(), "studio.sites.test", "tok-123", now);
        assert!(doc.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(doc.ends_with("END:VCALENDAR\r\n"));
        assert!(doc.contains("DTSTART:20260916T070000Z\r\n"));
        assert!(doc.contains("DTEND:20260916T073000Z\r\n"));
        assert!(doc.contains("DTSTAMP:20260901T080000Z\r\n"));
        assert!(doc.contains("UID:appt-1@sites.alo\r\n"));
        assert!(doc.contains("https://studio.sites.test/b/manage/tok-123"));
        // Every line respects the 75-octet limit.
        for line in doc.split("\r\n") {
            assert!(line.len() <= 75, "overlong line: {line}");
        }
    }

    #[test]
    fn text_values_are_escaped_per_rfc_5545() {
        let doc = ics_document(
            &appointment(),
            "s.sites.test",
            "t",
            time::macros::datetime!(2026-09-01 08:00 UTC),
        );
        assert!(doc.contains("SUMMARY:Consultation\\; morning\\, with Ada\\\\Bob"));
    }

    #[test]
    fn a_time_is_shown_in_the_appointments_own_clock() {
        assert_eq!(local_stamp(&appointment()), "2026-09-16 09:00");
    }
}
