//! `/t/{token}` — the buyer's own ticket (ADR 0041, item S3.04d): what was
//! bought, when it happens, and the calendar file that puts it in their own
//! calendar.
//!
//! The token is the capability the fulfilment sweep minted with the sale
//! ([`alo_store::site_ticket_fulfil`]); it travels only in the buyer's own
//! surfaces (the checkout return page, the `.ics`). Two routes hang off it,
//! both resolved **on the serving site only** — a ticket minted on one site
//! is the generic 404 on every other host:
//!
//! - `GET /t/{token}` — the ticket page: the event, the seats, the holder,
//!   and the calendar download.
//! - `GET /t/{token}/calendar.ics` — the event as an RFC 5545 document; its
//!   description carries the ticket link, so the ticket travels inside the
//!   buyer's own calendar entry.
//!
//! Everything here is `no-store` and unindexed: a ticket is nobody else's.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use time::OffsetDateTime;

use alo_store::{PublicTicket, PublishedSite};

use crate::render::html::esc;
use crate::render::{UiStrings, strings_for};

use super::AppState;
use super::ics::{ics_escape, ics_fold, ics_time};
use super::rendered::minimal_page;

/// The ticket page.
pub(super) async fn show(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some((site, ticket)) = resolve(&state, host.as_deref(), &token).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let body = format!(
        "<p>{} — {}</p>\n<p>{}: {}</p>\n<p>{}: {}</p>\n\
         <p><a href=\"/t/{token}/calendar.ics\">{}</a></p>\n",
        esc(shown_description(&ticket, &site.name)),
        esc(&when(&ticket)),
        esc(strings.ticket_holder_label),
        esc(&ticket.holder),
        esc(strings.ticket_seats_label),
        ticket.quantity,
        esc(strings.booking_add_calendar),
        token = esc(&token),
    );
    page(strings, strings.ticket_page_title, &body)
}

/// The ticket as an iCalendar document.
pub(super) async fn calendar(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some((site, ticket)) = resolve(&state, host.as_deref(), &token).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let host = host.unwrap_or_default();
    let document = ics_document(&ticket, &site.name, &host, &token, OffsetDateTime::now_utc());
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/calendar; charset=utf-8"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"ticket.ics\""),
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

/// Host + token → the one ticket the token stands for **on this site**.
async fn resolve(
    state: &Arc<AppState>,
    host: Option<&str>,
    token: &str,
) -> Option<(PublishedSite, PublicTicket)> {
    let scope = super::host::scope(host?, &state.sites_domain)?;
    let site = match super::resolve_scope(state, &scope).await {
        Ok(site) => site?,
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "ticket resolver read failed");
            return None;
        }
    };
    match state.store.public_ticket(&site, token).await {
        Ok(Some(ticket)) => Some((site, ticket)),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(%error, "public ticket read failed");
            None
        }
    }
}

/// What the ticket says it is for: the fulfilment's description, or the
/// site's name in the moment between claim and record.
fn shown_description<'a>(ticket: &'a PublicTicket, site_name: &'a str) -> &'a str {
    if ticket.description.is_empty() {
        site_name
    } else {
        &ticket.description
    }
}

/// The event's start as a UTC day and wall clock — the venue's own zone
/// arrives with the shop sections (S3.04f); until then the honest clock is
/// the universal one.
fn when(ticket: &PublicTicket) -> String {
    let start = ticket.starts_at.to_offset(time::UtcOffset::UTC);
    format!(
        "{} {:02}:{:02} UTC",
        start.date(),
        start.hour(),
        start.minute()
    )
}

/// One complete RFC 5545 document for one ticket. The description and URL
/// both carry the ticket link, so the ticket travels **inside** the buyer's
/// own calendar entry. No DTEND: an admission names when doors open, not
/// when the evening ends.
fn ics_document(
    ticket: &PublicTicket,
    site_name: &str,
    host: &str,
    token: &str,
    now: OffsetDateTime,
) -> String {
    let link = format!("https://{host}/t/{token}");
    let mut out = String::new();
    for line in [
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//alo//sites//EN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "METHOD:PUBLISH".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{token}@sites.alo"),
        format!("DTSTAMP:{}", ics_time(now)),
        format!("DTSTART:{}", ics_time(ticket.starts_at)),
        format!("SUMMARY:{}", ics_escape(shown_description(ticket, site_name))),
        format!("DESCRIPTION:{}", ics_escape(&format!("Ticket: {link}"))),
        format!("URL:{}", ics_escape(&link)),
        "END:VEVENT".to_owned(),
        "END:VCALENDAR".to_owned(),
    ] {
        out.push_str(&ics_fold(&line));
        out.push_str("\r\n");
    }
    out
}

/// A ticket document: never cacheable, never indexed.
fn page(strings: &UiStrings, title: &str, body: &str) -> Response {
    (
        StatusCode::OK,
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

    use super::*;

    const SITE_NAME: &str = "Letterpress Studio";

    fn ticket() -> PublicTicket {
        PublicTicket {
            description: "Letterpress workshop — 2026-09-16".to_owned(),
            starts_at: time::macros::datetime!(2026-09-16 17:00 UTC),
            quantity: 2,
            holder: "Maud; Adams".to_owned(),
        }
    }

    #[test]
    fn the_document_carries_the_ticket_link_and_no_dtend() {
        let now = time::macros::datetime!(2026-09-01 08:00 UTC);
        let doc = ics_document(&ticket(), SITE_NAME, "studio.sites.test", "tok-9", now);
        assert!(doc.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(doc.ends_with("END:VCALENDAR\r\n"));
        assert!(doc.contains("DTSTART:20260916T170000Z\r\n"));
        assert!(!doc.contains("DTEND"));
        assert!(doc.contains("UID:tok-9@sites.alo\r\n"));
        assert!(doc.contains("https://studio.sites.test/t/tok-9"));
        assert!(doc.contains("SUMMARY:Letterpress workshop — 2026-09-16\r\n"));
        for line in doc.split("\r\n") {
            assert!(line.len() <= 75, "overlong line: {line}");
        }
    }

    #[test]
    fn an_unrecorded_description_falls_back_to_the_site_name() {
        let mut bare = ticket();
        bare.description = String::new();
        assert_eq!(shown_description(&bare, SITE_NAME), "Letterpress Studio");
        assert_eq!(
            shown_description(&ticket(), SITE_NAME),
            "Letterpress workshop — 2026-09-16"
        );
    }

    #[test]
    fn the_when_is_the_utc_wall_clock() {
        assert_eq!(when(&ticket()), "2026-09-16 17:00 UTC");
    }
}
