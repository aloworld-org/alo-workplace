//! `GET /b/:booking_id` and `POST /b/:booking_id` — the public booking flow,
//! the third write on the public surface and the sibling of [`super::orders`].
//!
//! Two requests, no JavaScript. The published page carries a day field; the
//! `GET` answers with that day's free times as radio buttons, the service's own
//! questions beneath them, and one button; the `POST` takes the appointment and
//! lands the visitor on a confirmation. Free time cannot be baked into the
//! published page — those bytes are cached per publish, and a Tuesday afternoon
//! is not — so it is read live here, uncacheable and no-store.
//!
//! The wire contract mirrors the order door's, because a visitor meets both the
//! same way: an unresolvable service id is the generic `404` (unknown,
//! unpublished, switched off, and calendar-deleted are indistinguishable), an
//! unreadable body is `400`, an oversized body `413`, a rate-limited visitor
//! `429` with `Retry-After`, and a slot that was free when the page rendered
//! and is not any more is `409` with the store's own sentence. The honeypot's
//! silent drop is answered exactly like a success.
//!
//! Privacy: the rate-limit key is used transiently ([`super::rate`]); nothing
//! about the visitor's connection is logged or stored. An appointment is what
//! was typed into it, and the time it is for.

use std::sync::Arc;
use std::time::Instant;

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use alo_store::{
    BookingRequest, PublicBookingService, SiteBookingFieldKind, StoreError, local_day,
    local_wall_clock,
};

use crate::render::html::esc;
use crate::render::{EN, UiStrings};

use super::AppState;
use super::rendered::minimal_page;

/// The most an encoded booking body may carry. Sized from the store's answer
/// caps at worst-case encoding — eight 2 000-character answers at four UTF-8
/// bytes each, tripled by percent-encoding — with headroom.
pub(super) const BOOKING_BODY_MAX_BYTES: usize = 256 * 1024;

/// How a day travels on the wire, in both directions: the `<input type=date>`
/// value and the hidden field the booking form posts back.
const DAY_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");
/// How an offered time is written for the visitor, in the service's own zone.
const TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");

/// `?date=YYYY-MM-DD` — the day whose free times to show. Absent means today in
/// the service's own zone, which is what a link with no query should answer.
#[derive(Deserialize)]
pub(super) struct DayQuery {
    #[serde(default)]
    date: Option<String>,
}

/// The fixed field contract of a rendered booking form. `slot` is the exact
/// instant the visitor picked (RFC 3339, from the radio we rendered), `website`
/// the visually-hidden honeypot, and everything else is either a visitor field
/// or one of the service's own questions, which arrive as `q-<question key>`.
#[derive(Default)]
struct BookingBody {
    slot: String,
    name: String,
    email: String,
    website: String,
    answers: Vec<(String, String)>,
}

/// The prefix a rendered booking form puts before a question key.
const ANSWER_PREFIX: &str = "q-";

/// Shows one day's free times, with the form that takes one of them.
pub(super) async fn offer(
    State(state): State<Arc<AppState>>,
    Path(booking_id): Path<String>,
    Query(query): Query<DayQuery>,
) -> Response {
    let Some(service) = resolve(&state, &booking_id).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let now = OffsetDateTime::now_utc();
    let day = match query.date.as_deref() {
        Some(raw) => match Date::parse(raw.trim(), DAY_FORMAT) {
            Ok(day) => day,
            // A day we cannot read is not an error worth a page of its own:
            // offer today instead, which is what the visitor wanted anyway.
            Err(_) => today_in(&service, now),
        },
        None => today_in(&service, now),
    };
    let slots = match state.store.public_booking_slots(&service, day, now).await {
        Ok(slots) => slots,
        Err(error) => {
            tracing::error!(%error, "booking availability read failed");
            return super::unavailable();
        }
    };
    let body = day_document(&service, day, &slots, &EN);
    page(StatusCode::OK, &EN, &service.published.name, &body)
}

/// Takes one appointment.
pub(super) async fn book(
    State(state): State<Arc<AppState>>,
    Path(booking_id): Path<String>,
    request: Request,
) -> Response {
    if let Err(wait) = state
        .rate
        .allow(&super::forms::client_key(&request), Instant::now())
    {
        return rate_limited(wait, &EN);
    }

    // Decoded as ordered pairs rather than a struct: the answer field names are
    // data (`q-<question key>`), and only the service knows what they are.
    let posted = match Form::<Vec<(String, String)>>::from_request(request, &()).await {
        Ok(Form(pairs)) => pairs,
        Err(FormRejection::BytesRejection(_)) => {
            return message_page(
                StatusCode::PAYLOAD_TOO_LARGE,
                &EN,
                EN.booking_not_booked_title,
                EN.form_malformed_text,
            );
        }
        Err(_) => {
            return message_page(
                StatusCode::BAD_REQUEST,
                &EN,
                EN.booking_not_booked_title,
                EN.form_malformed_text,
            );
        }
    };
    let body = decode(&posted);

    let Some(service) = resolve(&state, &booking_id).await else {
        return super::not_found(state.unknown_host.clone());
    };

    // Honeypot tripped: the field no human sees is filled, so this is bot
    // traffic. Answer exactly like success and reserve nothing.
    if !body.website.trim().is_empty() {
        return message_page(
            StatusCode::OK,
            &EN,
            EN.booking_booked_title,
            EN.booking_booked_text,
        );
    }

    let Ok(starts_at) = OffsetDateTime::parse(
        body.slot.trim(),
        &time::format_description::well_known::Rfc3339,
    ) else {
        return message_page(
            StatusCode::BAD_REQUEST,
            &EN,
            EN.booking_not_booked_title,
            EN.form_malformed_text,
        );
    };

    let request = BookingRequest {
        starts_at,
        visitor_name: &body.name,
        visitor_email: &body.email,
        answers: &body.answers,
    };
    match state
        .store
        .reserve_public_booking(&service, &request, OffsetDateTime::now_utc())
        .await
    {
        Ok(Some(reserved)) => {
            let when = local_stamp(reserved.starts_at, &reserved.time_zone);
            let text = format!(
                "{} {} — {when}. {}",
                EN.booking_booked_text, reserved.booking_name, EN.form_back_hint
            );
            message_page(StatusCode::OK, &EN, EN.booking_booked_title, &text)
        }
        Ok(None) => super::not_found(state.unknown_host.clone()),
        Err(StoreError::Validation(reason)) => refused(StatusCode::BAD_REQUEST, &reason),
        Err(StoreError::Conflict(reason)) => refused(StatusCode::CONFLICT, &reason),
        Err(error) => {
            tracing::error!(%error, "booking reservation failed");
            super::unavailable()
        }
    }
}

/// Resolves a service id, logging nothing that could tell one absence from
/// another.
async fn resolve(state: &AppState, booking_id: &str) -> Option<PublicBookingService> {
    match state.store.public_booking(booking_id).await {
        Ok(service) => service,
        Err(error) => {
            tracing::error!(%error, "booking service read failed");
            None
        }
    }
}

/// Today in the service's own zone — the day an owner means by "today", not the
/// server's.
fn today_in(service: &PublicBookingService, now: OffsetDateTime) -> Date {
    local_day(now, &service.published.time_zone).unwrap_or_else(|| now.date())
}

/// One instant written as a day and a wall clock in the given zone, falling
/// back to UTC when the zone cannot be resolved — a confirmation never shows a
/// blank time.
fn local_stamp(instant: OffsetDateTime, time_zone: &str) -> String {
    match local_wall_clock(instant, time_zone) {
        Some((day, (hour, minute))) => format!(
            "{} {hour:02}:{minute:02}",
            day.format(DAY_FORMAT).unwrap_or_default()
        ),
        None => format!(
            "{} {}",
            instant.date().format(DAY_FORMAT).unwrap_or_default(),
            instant.format(TIME_FORMAT).unwrap_or_default()
        ),
    }
}

/// The body of the free-times document: the day that was asked for, what is
/// free on it, and — when something is — the form that takes one.
fn day_document(
    service: &PublicBookingService,
    day: Date,
    slots: &[alo_store::BookingSlot],
    strings: &UiStrings,
) -> String {
    let published = &service.published;
    let day_text = day.format(DAY_FORMAT).unwrap_or_default();
    let mut out = format!(
        "<p class=\"booking-length\">{} {}</p>\n",
        published.duration_minutes,
        esc(strings.booking_minutes)
    );
    if let Some(location) = &published.location {
        out.push_str(&format!(
            "<p class=\"booking-where\">{}: {}</p>\n",
            esc(strings.booking_where),
            esc(location)
        ));
    }
    out.push_str(&format!(
        "<h2>{} — {}</h2>\n",
        esc(strings.booking_times_title),
        esc(&day_text)
    ));
    if slots.is_empty() {
        out.push_str(&format!("<p>{}</p>\n", esc(strings.booking_no_times)));
        return out;
    }
    out.push_str(&format!(
        "<form action=\"/b/{}\" method=\"post\">\n<fieldset>\n<legend>{}</legend>\n",
        esc(published.booking_id.as_str()),
        esc(strings.booking_pick_time)
    ));
    for (index, slot) in slots.iter().enumerate() {
        let value = slot
            .starts_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let label = local_clock(slot.starts_at, &published.time_zone);
        out.push_str(&format!(
            "<p><input id=\"slot-{index}\" type=\"radio\" name=\"slot\" value=\"{}\"{}>\
             <label for=\"slot-{index}\">{}</label></p>\n",
            esc(&value),
            if index == 0 { " checked" } else { "" },
            esc(&label)
        ));
    }
    out.push_str("</fieldset>\n");
    out.push_str(&format!(
        "<p class=\"hp\" aria-hidden=\"true\"><label for=\"booking-website\">{}</label>\
         <input id=\"booking-website\" name=\"website\" type=\"text\" tabindex=\"-1\" \
         autocomplete=\"off\"></p>\n",
        esc(strings.form_website)
    ));
    out.push_str(&format!(
        "<p><label for=\"booking-name\">{}</label>\
         <input id=\"booking-name\" name=\"name\" type=\"text\" required maxlength=\"200\" \
         autocomplete=\"name\"></p>\n",
        esc(strings.form_name)
    ));
    out.push_str(&format!(
        "<p><label for=\"booking-email\">{}</label>\
         <input id=\"booking-email\" name=\"email\" type=\"email\" required maxlength=\"254\" \
         autocomplete=\"email\"></p>\n",
        esc(strings.form_email)
    ));
    for field in &published.fields {
        push_question(&mut out, field);
    }
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n</form>\n",
        esc(strings.booking_book)
    ));
    out
}

/// One of the service's own questions, in the kind it was defined as.
fn push_question(out: &mut String, field: &alo_store::SiteBookingField) {
    let id = format!("q-{}", field.key);
    let required = if field.required { " required" } else { "" };
    out.push_str(&format!(
        "<p><label for=\"{}\">{}</label>",
        esc(&id),
        esc(&field.label)
    ));
    match field.kind {
        SiteBookingFieldKind::LongText => out.push_str(&format!(
            "<textarea id=\"{}\" name=\"{}\" maxlength=\"2000\"{required}></textarea>",
            esc(&id),
            esc(&id)
        )),
        SiteBookingFieldKind::Choice => {
            out.push_str(&format!(
                "<select id=\"{}\" name=\"{}\"{required}>",
                esc(&id),
                esc(&id)
            ));
            if !field.required {
                out.push_str("<option value=\"\"></option>");
            }
            for option in &field.options {
                out.push_str(&format!(
                    "<option value=\"{}\">{}</option>",
                    esc(option),
                    esc(option)
                ));
            }
            out.push_str("</select>");
        }
        SiteBookingFieldKind::Phone => out.push_str(&format!(
            "<input id=\"{}\" name=\"{}\" type=\"tel\" maxlength=\"2000\"{required}>",
            esc(&id),
            esc(&id)
        )),
        SiteBookingFieldKind::Text => out.push_str(&format!(
            "<input id=\"{}\" name=\"{}\" type=\"text\" maxlength=\"2000\"{required}>",
            esc(&id),
            esc(&id)
        )),
    }
    out.push_str("</p>\n");
}

/// A slot's start as a wall clock in the service's zone, falling back to the
/// UTC clock when the zone cannot be resolved — a time is never shown blank.
fn local_clock(instant: OffsetDateTime, time_zone: &str) -> String {
    match local_wall_clock(instant, time_zone) {
        Some((_, (hour, minute))) => format!("{hour:02}:{minute:02}"),
        None => instant.format(TIME_FORMAT).unwrap_or_default(),
    }
}

/// Sorts the posted pairs into [`BookingBody`], keeping every `q-…` answer in
/// the order it was posted. An unknown field is ignored rather than refused: a
/// page from an older publish may carry one, and losing a booking over it would
/// be the wrong answer to a stale page.
fn decode(posted: &[(String, String)]) -> BookingBody {
    let mut body = BookingBody::default();
    for (key, value) in posted {
        match key.as_str() {
            "slot" => body.slot.clone_from(value),
            "name" => body.name.clone_from(value),
            "email" => body.email.clone_from(value),
            "website" => body.website.clone_from(value),
            key => {
                if let Some(question) = key.strip_prefix(ANSWER_PREFIX) {
                    body.answers.push((question.to_owned(), value.clone()));
                }
            }
        }
    }
    body
}

/// A refused booking: the store's own sentence plus how to recover.
fn refused(status: StatusCode, reason: &str) -> Response {
    let text = format!("{reason}. {}", EN.form_back_hint);
    message_page(status, &EN, EN.booking_not_booked_title, &text)
}

/// The 429, with the limiter's `Retry-After` hint in seconds.
fn rate_limited(wait_seconds: u64, strings: &UiStrings) -> Response {
    let mut response = message_page(
        StatusCode::TOO_MANY_REQUESTS,
        strings,
        strings.booking_rate_limited_title,
        strings.booking_rate_limited_text,
    );
    if let Ok(value) = HeaderValue::from_str(&wait_seconds.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

/// A one-sentence result page, exactly like the order door's.
fn message_page(status: StatusCode, strings: &UiStrings, title: &str, text: &str) -> Response {
    page(status, strings, title, &format!("<p>{}</p>\n", esc(text)))
}

/// A booking document: never cacheable (free time changes by the minute, and a
/// confirmation is nobody else's), and never indexed.
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

    use super::*;

    fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn answers_decode_in_posted_order_and_unknown_fields_are_ignored() {
        let body = decode(&pairs(&[
            ("slot", "2026-09-16T07:00:00Z"),
            ("q-phone", "+32 2 555 01"),
            ("name", "Ada"),
            ("surprise", "field"),
            ("q-cut", "Wet"),
            ("email", "ada@example.test"),
            ("website", ""),
        ]));
        assert_eq!(body.slot, "2026-09-16T07:00:00Z");
        assert_eq!(body.name, "Ada");
        assert_eq!(body.email, "ada@example.test");
        assert_eq!(
            body.answers,
            vec![
                ("phone".to_owned(), "+32 2 555 01".to_owned()),
                ("cut".to_owned(), "Wet".to_owned()),
            ]
        );
    }

    #[test]
    fn a_time_is_shown_in_the_services_own_clock() {
        let instant = time::macros::datetime!(2026-09-16 07:00 UTC);
        assert_eq!(local_clock(instant, "Europe/Brussels"), "09:00");
        // An unresolvable zone still shows a time rather than nothing.
        assert_eq!(local_clock(instant, "Middle/Earth"), "07:00");
    }
}
