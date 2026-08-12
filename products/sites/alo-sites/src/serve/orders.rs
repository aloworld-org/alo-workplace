//! `POST /o/:catalog_id` — the order form of a published catalog, the second
//! write on the public surface and the sibling of [`super::forms`].
//!
//! The wire contract mirrors the contact form's, because a visitor meets both
//! the same way: success and the honeypot's silent drop are `200` with a plain
//! "order received" page, an unresolvable catalog id is the generic `404`
//! (unknown, unpublished, and ordering-off catalogs are indistinguishable), an
//! unreadable body is `400`, an oversized body `413`, and a rate-limited
//! sender `429` with `Retry-After`. Responses are complete little HTML
//! documents because this form has no script behind it at all: the browser
//! navigates to the answer.
//!
//! What is ordered is posted as one `qty-<item handle>` field per item, which
//! is what a `<form>` around a rendered catalog naturally produces. Prices are
//! never posted — they are read from the published snapshot in the store door
//! ([`alo_store::SitePublicStore::place_public_order`]), so a rewritten page
//! cannot invent one.
//!
//! Privacy: the rate-limit key is used transiently ([`super::rate`]); nothing
//! about the visitor's connection is logged or stored. An order is exactly
//! what was typed into it.

use std::sync::Arc;
use std::time::Instant;

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use alo_store::{OrderRequestLine, StoreError, normalize_order_contact};

use crate::render::{EN, UiStrings};

use super::AppState;
use super::rendered::minimal_document;

/// The most an encoded order body may carry. Sized from the store's field caps
/// at worst-case encoding — a 2 000-character note at four UTF-8 bytes each,
/// tripled by percent-encoding, plus fifty quantity fields — with headroom.
pub(super) const ORDER_BODY_MAX_BYTES: usize = 64 * 1024;

/// The prefix a rendered order form puts before an item handle in a field
/// name. Everything else in the body is a contact field or the honeypot.
const QUANTITY_PREFIX: &str = "qty-";

/// One decoded order body: the contact fields, the honeypot, and the quantity
/// fields in the order they were posted (which is the order of the page).
#[derive(Default)]
struct OrderBody {
    name: String,
    email: String,
    phone: String,
    note: String,
    website: String,
    lines: Vec<OrderRequestLine>,
}

/// Handles one order POST end to end: rate limit, parse, honeypot, then the
/// store's snapshot-priced, tenant-scoped write.
pub(super) async fn place(
    State(state): State<Arc<AppState>>,
    Path(catalog_id): Path<String>,
    request: Request,
) -> Response {
    if let Err(wait) = state
        .rate
        .allow(&super::forms::client_key(&request), Instant::now())
    {
        return rate_limited(wait, &EN);
    }

    // Decoded as ordered pairs rather than a struct: the field names are data
    // (`qty-<item handle>`), and the sequence is the sequence of the page.
    let posted = match Form::<Vec<(String, String)>>::from_request(request, &()).await {
        Ok(Form(pairs)) => pairs,
        // The body never buffered (over the route's size limit): 413.
        // Everything else — wrong content type, a broken read — is one
        // malformed-order 400 carrying no parser internals.
        Err(FormRejection::BytesRejection(_)) => {
            return message_page(
                StatusCode::PAYLOAD_TOO_LARGE,
                &EN,
                EN.order_not_sent_title,
                EN.form_malformed_text,
            );
        }
        Err(_) => {
            return message_page(
                StatusCode::BAD_REQUEST,
                &EN,
                EN.order_not_sent_title,
                EN.form_malformed_text,
            );
        }
    };
    let Some(body) = decode(&posted) else {
        return message_page(
            StatusCode::BAD_REQUEST,
            &EN,
            EN.order_not_sent_title,
            EN.form_malformed_text,
        );
    };

    // Honeypot tripped: the field no human sees is filled, so this is bot
    // traffic. Answer exactly like success and write nothing.
    if !body.website.trim().is_empty() {
        return placed(&EN);
    }

    let contact = match normalize_order_contact(&body.name, &body.email, &body.phone, &body.note) {
        Ok(contact) => contact,
        Err(StoreError::Validation(reason)) => return refused(&reason),
        Err(error) => {
            tracing::error!(%error, "order contact gate failed");
            return super::unavailable();
        }
    };

    match state
        .store
        .place_public_order(&catalog_id, &contact, &body.lines)
        .await
    {
        Ok(Some(_)) => placed(&EN),
        Ok(None) => super::not_found(state.unknown_host.clone()),
        Err(StoreError::Validation(reason)) => refused(&reason),
        Err(error) => {
            tracing::error!(%error, "order write failed");
            super::unavailable()
        }
    }
}

/// Sorts the posted pairs into [`OrderBody`], keeping the order of the
/// quantity fields. An unknown field is ignored rather than refused: a page
/// from an older publish may carry fields this build has no name for, and
/// failing a whole order over one would be the wrong answer to a stale page.
/// Returns `None` when a quantity is not a number at all.
fn decode(posted: &[(String, String)]) -> Option<OrderBody> {
    let mut body = OrderBody::default();
    for (key, value) in posted {
        match key.as_str() {
            "name" => body.name.clone_from(value),
            "email" => body.email.clone_from(value),
            "phone" => body.phone.clone_from(value),
            "note" => body.note.clone_from(value),
            "website" => body.website.clone_from(value),
            key => {
                let Some(slug) = key.strip_prefix(QUANTITY_PREFIX) else {
                    continue;
                };
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                // A quantity that is not a number at all is a broken client or
                // a hand-written body; refuse it rather than silently ordering
                // nothing.
                let quantity = value.parse::<i32>().ok()?;
                body.lines.push(OrderRequestLine {
                    item_slug: slug.to_owned(),
                    quantity,
                });
            }
        }
    }
    Some(body)
}

/// The one success answer — also the honeypot's silent drop, so the two are
/// indistinguishable on the wire.
fn placed(strings: &UiStrings) -> Response {
    message_page(
        StatusCode::OK,
        strings,
        strings.order_sent_title,
        strings.order_success,
    )
}

/// A refused order: the store's own field-level sentence plus how to recover.
fn refused(reason: &str) -> Response {
    let text = format!("{reason}. {}", EN.form_back_hint);
    message_page(StatusCode::BAD_REQUEST, &EN, EN.order_not_sent_title, &text)
}

/// The 429, with the limiter's `Retry-After` hint in seconds.
fn rate_limited(wait_seconds: u64, strings: &UiStrings) -> Response {
    let mut response = message_page(
        StatusCode::TOO_MANY_REQUESTS,
        strings,
        strings.order_rate_limited_title,
        strings.order_rate_limited_text,
    );
    if let Ok(value) = HeaderValue::from_str(&wait_seconds.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

/// An order-result page: a minimal uncacheable HTML document, safe to land on
/// from a scriptless form submit.
fn message_page(status: StatusCode, strings: &UiStrings, title: &str, text: &str) -> Response {
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
        minimal_document(strings.lang, title, text),
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
    fn quantities_decode_in_page_order_and_unknown_fields_are_ignored() {
        let body = decode(&pairs(&[
            ("qty-sourdough", "2"),
            ("name", "Ada"),
            ("qty-focaccia", "1"),
            ("email", "ada@example.test"),
            ("phone", ""),
            ("note", "no nuts"),
            ("website", ""),
            ("surprise", "field"),
            ("qty-croissant", "0"),
        ]))
        .unwrap();
        assert_eq!(body.name, "Ada");
        assert_eq!(body.email, "ada@example.test");
        assert_eq!(body.note, "no nuts");
        assert_eq!(
            body.lines,
            vec![
                OrderRequestLine {
                    item_slug: "sourdough".to_owned(),
                    quantity: 2
                },
                OrderRequestLine {
                    item_slug: "focaccia".to_owned(),
                    quantity: 1
                },
                OrderRequestLine {
                    item_slug: "croissant".to_owned(),
                    quantity: 0
                },
            ]
        );
    }

    #[test]
    fn a_quantity_that_is_not_a_number_refuses_the_whole_body() {
        assert!(decode(&pairs(&[("name", "Ada"), ("qty-sourdough", "lots")])).is_none());
        // An empty quantity is what an untouched field posts in some browsers;
        // it means "none of this", not a broken body.
        assert!(
            decode(&pairs(&[("name", "Ada"), ("qty-sourdough", "")]))
                .unwrap()
                .lines
                .is_empty()
        );
    }
}
