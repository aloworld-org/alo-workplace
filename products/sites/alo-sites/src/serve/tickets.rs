//! `/tix` — the public ticket shop (ADR 0041, item S3.04f): the live event
//! listing, the offer page with its buy form, the hosted-payment handoff, the
//! buyer's return page, and the provider's webhook.
//!
//! Four doors, no JavaScript:
//!
//! - `GET /tix` — every upcoming event of the serving site, priced by the
//!   Billing seam at this instant. Live state, `no-store`: a price or a seat
//!   count must never be cached bytes.
//! - `GET /tix/{event}` + `POST /tix/{event}` — one event and the form that
//!   starts a purchase. The POST runs the S3.04f1 machinery in the only safe
//!   order (typo gate → hold → order), asks the provider for a hosted
//!   payment, and answers `303 See Other` to the provider's page — the card
//!   is typed there, never here.
//! - `GET /tix/order/{order}` — the page the provider sends the buyer back
//!   to. It **fetches** the payment's status from the provider and settles
//!   with that answer (fetch-not-believe), then says where the order stands
//!   and, once fulfilment has minted it, links the ticket.
//! - `POST /_alo/pay` — the webhook. A doorbell, not a message: the body
//!   names a payment id and nothing else; the status is always fetched from
//!   the provider ([`alo_store::site_payments`]), so an unauthenticated POST
//!   can make alo *look*, never make it *believe*.
//!
//! The wire contract mirrors the booking door's: an unresolvable host or a
//! foreign event/order id is the generic 404 (unknown, foreign and another
//! site's are indistinguishable), an unreadable body 400, an oversized body
//! 413, a rate-limited visitor 429 with `Retry-After`, and a refused
//! purchase carries the store's own sentence (400 validation, 409 seats).
//! The honeypot's silent drop answers exactly like the redirect a real buyer
//! gets, and reserves nothing. An installation with no payment provider says
//! so on the offer page instead of rendering a form that could only fail.
//!
//! Privacy: the rate-limit key is used transiently ([`super::rate`]); the
//! return page never echoes the buyer's own name or address (holding a
//! return URL proves less than being the buyer); logs carry ids only.
//!
//! The stock shop (`/shop`, [`super::shop`], item S3.05a3) is this door's
//! sibling and reuses the machinery here — the Host resolution, the page
//! shell, the refusal and redirect shapes, and the one `/_alo/pay` webhook,
//! which resolves a payment id against ticket orders first and stock orders
//! second (the id spaces are disjoint; a payment belongs to exactly one).

use std::sync::Arc;
use std::time::Instant;

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use time::OffsetDateTime;

use alo_store::{
    PublicTicketEvent, PublishedSite, SitePaymentError, SitePaymentRequest, StoreError,
    TICKET_HOLD_MAX_QUANTITY,
};

use crate::render::html::esc;
use crate::render::money::format_price;
use crate::render::{UiStrings, strings_for};

use super::AppState;
use super::rendered::minimal_page;

/// The most an encoded checkout body may carry: four short fields at
/// worst-case encoding, with headroom.
pub(super) const CHECKOUT_BODY_MAX_BYTES: usize = 8 * 1024;

/// The most a webhook body may carry: providers in the Mollie shape post one
/// form-encoded payment id.
pub(super) const WEBHOOK_BODY_MAX_BYTES: usize = 4 * 1024;

/// The fixed field contract of the buy form. `website` is the visually
/// hidden honeypot, exactly as on the form, order and booking doors.
#[derive(Default)]
struct CheckoutBody {
    quantity: String,
    name: String,
    email: String,
    website: String,
}

/// `GET /tix` — the live listing.
pub(super) async fn listing(State(state): State<Arc<AppState>>, request: Request) -> Response {
    // The Host is copied out before any await: the request body is not
    // `Sync`, so nothing here may hold the request across one.
    let host = host_header(&request);
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let events = match state
        .store
        .public_ticket_events(&site, OffsetDateTime::now_utc())
        .await
    {
        Ok(events) => events,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "ticket listing read failed");
            return super::unavailable();
        }
    };
    let mut body = String::new();
    if events.is_empty() {
        body.push_str(&format!("<p>{}</p>\n", esc(strings.tix_empty)));
    } else {
        body.push_str("<ul class=\"tix-list\">\n");
        for event in &events {
            body.push_str("<li>\n");
            body.push_str(&format!(
                "<h2><a href=\"/tix/{}\">{}</a></h2>\n",
                esc(event.id.as_str()),
                esc(&event.name)
            ));
            push_event_facts(&mut body, event, strings);
            body.push_str("</li>\n");
        }
        body.push_str("</ul>\n");
    }
    page(StatusCode::OK, strings, strings.tix_title, &body)
}

/// `GET /tix/{event}` — one event and, while seats and a provider exist, the
/// form that buys them.
pub(super) async fn offer(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let Some(event) = read_event(&state, &site, &event_id).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let mut body = String::new();
    push_event_facts(&mut body, &event, strings);
    if event.remaining <= 0 {
        body.push_str(&format!(
            "<p class=\"tix-sold-out\">{}</p>\n",
            esc(strings.tix_sold_out)
        ));
    } else if state.payments.is_none() {
        // No provider is wired into this installation: the honest sentence,
        // never a form whose submit could only fail.
        body.push_str(&format!("<p>{}</p>\n", esc(strings.tix_unconfigured)));
    } else {
        push_buy_form(&mut body, &event, strings);
    }
    body.push_str(&format!(
        "<p><a href=\"/tix\">{}</a></p>\n",
        esc(strings.tix_back)
    ));
    page(StatusCode::OK, strings, &event.name, &body)
}

/// `POST /tix/{event}` — starts a purchase and hands the buyer to the
/// provider's hosted page.
pub(super) async fn checkout(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
    request: Request,
) -> Response {
    if let Err(wait) = state
        .rate
        .allow(&super::forms::client_key(&request), Instant::now())
    {
        return rate_limited(wait, &crate::render::EN);
    }
    // Host and headers are read before the body consumes the request.
    let host = host_header(&request);
    let posted = match Form::<Vec<(String, String)>>::from_request(request, &()).await {
        Ok(Form(pairs)) => pairs,
        Err(FormRejection::BytesRejection(_)) => {
            return refusal_page(
                &state,
                host.as_deref(),
                StatusCode::PAYLOAD_TOO_LARGE,
                |strings| strings.tix_title,
            )
            .await;
        }
        Err(_) => {
            return refusal_page(
                &state,
                host.as_deref(),
                StatusCode::BAD_REQUEST,
                |strings| strings.tix_title,
            )
            .await;
        }
    };
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let body = decode(&posted);

    // Honeypot tripped: the field no human sees is filled. Answer exactly
    // like a real buyer's first hop — back to the offer — and hold nothing.
    if !body.website.trim().is_empty() {
        return see_other(&format!("/tix/{}", esc_path(&event_id)));
    }

    let Ok(quantity) = body.quantity.trim().parse::<i32>() else {
        return refused(
            StatusCode::BAD_REQUEST,
            strings,
            strings.tix_title,
            strings.form_malformed_text,
        );
    };

    // The provider is checked before the store takes a hold: an unconfigured
    // shop must not reserve seats it can never sell.
    let Some(provider) = state.payments.clone() else {
        return refused(
            StatusCode::SERVICE_UNAVAILABLE,
            strings,
            strings.tix_title,
            strings.tix_unconfigured,
        );
    };

    let now = OffsetDateTime::now_utc();
    let checkout = match state
        .store
        .public_begin_ticket_checkout(&site, &event_id, quantity, &body.name, &body.email, now)
        .await
    {
        Ok(Some(checkout)) => checkout,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(StoreError::Validation(reason)) => {
            return refused(StatusCode::BAD_REQUEST, strings, strings.tix_title, &reason);
        }
        Err(StoreError::Conflict(reason)) => {
            return refused(StatusCode::CONFLICT, strings, strings.tix_title, &reason);
        }
        Err(error) => {
            tracing::error!(site = %site.site, %error, "ticket checkout failed");
            return super::unavailable();
        }
    };

    // The hosted handoff. The webhook URL is this host's only because a URL
    // needs a host to be reachable — the webhook route itself trusts no Host
    // and resolves purely by payment id.
    let public_host = site_host(&site, &host);
    let payment = SitePaymentRequest {
        idempotency_key: checkout.order.as_str().to_owned(),
        amount_cents: checkout.amount_cents,
        currency: checkout.currency.clone(),
        description: checkout.description.clone(),
        redirect_url: format!("https://{public_host}/tix/order/{}", checkout.order),
        webhook_url: format!("https://{public_host}/_alo/pay"),
    };
    let created = match provider.create_payment(payment).await {
        Ok(created) => created,
        Err(SitePaymentError::Unconfigured) => {
            return refused(
                StatusCode::SERVICE_UNAVAILABLE,
                strings,
                strings.tix_title,
                strings.tix_unconfigured,
            );
        }
        Err(error) => {
            // The order sits open with its hold; both lapse on the hold's own
            // TTL. Nothing to tell the visitor but "not now".
            tracing::error!(order = %checkout.order, %error, "hosted payment creation failed");
            return super::unavailable();
        }
    };
    match state
        .store
        .public_open_ticket_payment(
            &site,
            &checkout.order,
            &created.provider_payment_id,
            &created.checkout_url,
        )
        .await
    {
        Ok(Some(())) => {}
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(error) => {
            tracing::error!(order = %checkout.order, %error, "recording the hosted payment failed");
            return super::unavailable();
        }
    }
    see_other(&created.checkout_url)
}

/// `GET /tix/order/{order}` — the buyer's return page: fetch, settle, say.
pub(super) async fn order_status(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let Some(mut order) = read_order(&state, &site, &order_id).await else {
        return super::not_found(state.unknown_host.clone());
    };

    // Fetch-on-return: the provider may know the outcome before its webhook
    // lands. Only an open order with a payment is worth asking about, and a
    // provider that cannot answer right now costs nothing — the page shows
    // the stored state and the webhook settles later.
    if order.state.is_open()
        && let (Some(payment_id), Some(provider)) =
            (order.provider_payment_id.clone(), state.payments.clone())
    {
        match provider.payment_status(payment_id.clone()).await {
            Ok(status) => {
                if let Err(error) = settle(&state, &payment_id, status).await {
                    tracing::error!(order = %order_id, %error, "settling on return failed");
                }
                if let Some(fresh) = read_order(&state, &site, &order_id).await {
                    order = fresh;
                }
            }
            Err(error) => {
                tracing::warn!(order = %order_id, %error, "payment status fetch on return failed");
            }
        }
    }

    let mut body = String::new();
    if order.state.is_open() {
        body.push_str(&format!("<p>{}</p>\n", esc(strings.tix_order_open)));
        if let Some(checkout_url) = &order.checkout_url {
            body.push_str(&format!(
                "<p><a class=\"button\" href=\"{}\">{}</a></p>\n",
                esc(checkout_url),
                esc(strings.tix_pay)
            ));
        }
    } else if order.state == alo_store::SiteTicketOrderState::Paid {
        match &order.ticket_token {
            Some(token) => body.push_str(&format!(
                "<p><a class=\"button\" href=\"/t/{}\">{}</a></p>\n",
                esc(token),
                esc(strings.ticket_page_title)
            )),
            None => body.push_str(&format!("<p>{}</p>\n", esc(strings.tix_order_paid_wait))),
        }
    } else {
        body.push_str(&format!("<p>{}</p>\n", esc(strings.tix_order_dead)));
        body.push_str(&format!(
            "<p><a href=\"/tix\">{}</a></p>\n",
            esc(strings.tix_back)
        ));
    }
    page(StatusCode::OK, strings, strings.tix_order_title, &body)
}

/// `POST /_alo/pay` — the provider's doorbell. The body names a payment id;
/// everything believed is fetched from the provider. Host-independent: a
/// webhook has no Host worth trusting, so the target is resolved purely by
/// the payment id and a probe with an id nobody holds learns nothing.
pub(super) async fn webhook(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let posted = match Form::<Vec<(String, String)>>::from_request(request, &()).await {
        Ok(Form(pairs)) => pairs,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let Some(payment_id) = posted
        .iter()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(provider) = state.payments.clone() else {
        // Nothing to fetch from, so nothing to believe — and nothing for the
        // caller to learn.
        return ok_quietly();
    };
    // One payment id belongs to exactly one order — ticket or stock. Both
    // doors are asked; an id nobody holds answers exactly like success.
    enum Target {
        Ticket(alo_store::TicketPaymentTarget),
        Stock(alo_store::StockPaymentTarget),
    }
    let target = match state.store.public_ticket_payment_target(&payment_id).await {
        Ok(Some(target)) => Target::Ticket(target),
        Ok(None) => match state.store.public_stock_payment_target(&payment_id).await {
            Ok(Some(target)) => Target::Stock(target),
            Ok(None) => return ok_quietly(),
            Err(error) => {
                tracing::error!(%error, "webhook stock target lookup failed");
                return super::unavailable();
            }
        },
        Err(error) => {
            tracing::error!(%error, "webhook target lookup failed");
            return super::unavailable();
        }
    };
    let status = match provider.payment_status(payment_id.clone()).await {
        Ok(status) => status,
        Err(error) => {
            // A 503 asks a well-behaved provider to ring again later.
            tracing::error!(%error, "payment status fetch failed");
            return super::unavailable();
        }
    };
    let now = OffsetDateTime::now_utc();
    let settled = match &target {
        Target::Ticket(target) => {
            state
                .store
                .public_settle_ticket_payment(target, status, now)
                .await
        }
        Target::Stock(target) => {
            state
                .store
                .public_settle_stock_payment(target, status, now)
                .await
        }
    };
    match settled {
        Ok(()) | Err(StoreError::NotFound) => ok_quietly(),
        Err(error) => {
            tracing::error!(%error, "webhook settle failed");
            super::unavailable()
        }
    }
}

/// Applies a fetched status to whichever order holds `payment_id` — shared by
/// the webhook and the return page.
async fn settle(
    state: &AppState,
    payment_id: &str,
    status: alo_store::SitePaymentStatus,
) -> Result<(), StoreError> {
    let Some(target) = state.store.public_ticket_payment_target(payment_id).await? else {
        return Ok(());
    };
    state
        .store
        .public_settle_ticket_payment(&target, status, OffsetDateTime::now_utc())
        .await
}

/// The facts every surface states about an event: when it is, what one seat
/// costs, and — only when they are gone — that the seats are gone.
fn push_event_facts(out: &mut String, event: &PublicTicketEvent, strings: &UiStrings) {
    out.push_str(&format!(
        "<p class=\"tix-when\">{}</p>\n",
        esc(&when(event))
    ));
    out.push_str(&format!(
        "<p class=\"tix-price\">{} {}</p>\n",
        esc(&format_price(
            event.unit_price_cents,
            &event.currency,
            strings
        )),
        esc(strings.tix_per_seat)
    ));
    if event.remaining <= 0 {
        out.push_str(&format!(
            "<p class=\"tix-sold-out\">{}</p>\n",
            esc(strings.tix_sold_out)
        ));
    }
}

/// The buy form: the honeypot, how many seats, who is buying, one button.
fn push_buy_form(out: &mut String, event: &PublicTicketEvent, strings: &UiStrings) {
    let max = event.remaining.min(i64::from(TICKET_HOLD_MAX_QUANTITY));
    out.push_str(&format!(
        "<form action=\"/tix/{}\" method=\"post\">\n",
        esc(event.id.as_str())
    ));
    out.push_str(&format!(
        "<p class=\"hp\" aria-hidden=\"true\"><label for=\"tix-website\">{}</label>\
         <input id=\"tix-website\" name=\"website\" type=\"text\" tabindex=\"-1\" \
         autocomplete=\"off\"></p>\n",
        esc(strings.form_website)
    ));
    out.push_str(&format!(
        "<p><label for=\"tix-quantity\">{}</label>\
         <input id=\"tix-quantity\" name=\"quantity\" type=\"number\" min=\"1\" max=\"{max}\" \
         step=\"1\" value=\"1\" required inputmode=\"numeric\"></p>\n",
        esc(strings.ticket_seats_label)
    ));
    out.push_str(&format!(
        "<p><label for=\"tix-name\">{}</label>\
         <input id=\"tix-name\" name=\"name\" type=\"text\" required maxlength=\"200\" \
         autocomplete=\"name\"></p>\n",
        esc(strings.form_name)
    ));
    out.push_str(&format!(
        "<p><label for=\"tix-email\">{}</label>\
         <input id=\"tix-email\" name=\"email\" type=\"email\" required maxlength=\"254\" \
         autocomplete=\"email\"></p>\n",
        esc(strings.form_email)
    ));
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n</form>\n",
        esc(strings.tix_pay)
    ));
}

/// The event's start as a UTC day and wall clock — the venue's own zone is a
/// journaled cut carried from S3.04d. Shared with the conversation's tickets
/// offer ([`super::chat`]), which states the same instant the same way.
pub(super) fn when(event: &PublicTicketEvent) -> String {
    let start = event.starts_at.to_offset(time::UtcOffset::UTC);
    format!(
        "{} {:02}:{:02} UTC",
        start.date(),
        start.hour(),
        start.minute()
    )
}

/// The raw Host header, read before any await.
pub(super) fn host_header(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// Host → the serving site, or `None` for every kind of miss.
pub(super) async fn resolve_host(state: &AppState, host: Option<&str>) -> Option<PublishedSite> {
    let scope = super::host::scope(host?, &state.sites_domain)?;
    match super::resolve_scope(state, &scope).await {
        Ok(site) => site,
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "tix resolver read failed");
            None
        }
    }
}

/// The public host the redirect and webhook URLs are built on: the Host the
/// buyer is on, or the site's own subdomain when the header was unreadable.
pub(super) fn site_host(site: &PublishedSite, host: &Option<String>) -> String {
    host.clone()
        .unwrap_or_else(|| format!("{}.invalid", site.site))
}

async fn read_event(
    state: &AppState,
    site: &PublishedSite,
    event_id: &str,
) -> Option<PublicTicketEvent> {
    match state
        .store
        .public_ticket_event(site, event_id, OffsetDateTime::now_utc())
        .await
    {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "ticket offer read failed");
            None
        }
    }
}

async fn read_order(
    state: &AppState,
    site: &PublishedSite,
    order_id: &str,
) -> Option<alo_store::PublicTicketOrderStatus> {
    match state.store.public_ticket_order(site, order_id).await {
        Ok(order) => order,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "ticket order read failed");
            None
        }
    }
}

/// A refusal on a request whose site could not (or need not) be resolved:
/// still themed by the site when the Host does resolve, English otherwise.
/// `title_of` picks the refusing door's page title from the locale's strings.
pub(super) async fn refusal_page(
    state: &AppState,
    host: Option<&str>,
    status: StatusCode,
    title_of: fn(&UiStrings) -> &'static str,
) -> Response {
    let strings = match resolve_host(state, host).await {
        Some(site) => strings_for(&site.default_locale),
        None => &crate::render::EN,
    };
    refused(
        status,
        strings,
        title_of(strings),
        strings.form_malformed_text,
    )
}

/// A refused purchase: the store's own sentence plus how to recover.
pub(super) fn refused(
    status: StatusCode,
    strings: &UiStrings,
    title: &str,
    reason: &str,
) -> Response {
    let text = format!("{reason}. {}", strings.form_back_hint);
    page(status, strings, title, &format!("<p>{}</p>\n", esc(&text)))
}

/// The 429, with the limiter's `Retry-After` hint in seconds.
pub(super) fn rate_limited(wait_seconds: u64, strings: &UiStrings) -> Response {
    let mut response = page(
        StatusCode::TOO_MANY_REQUESTS,
        strings,
        strings.booking_rate_limited_title,
        &format!("<p>{}</p>\n", esc(strings.booking_rate_limited_text)),
    );
    if let Ok(value) = HeaderValue::from_str(&wait_seconds.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

/// `303 See Other`: the POST is done; what comes next is a GET somewhere
/// else — the provider's hosted page, or the offer the honeypot pretends to
/// have bought from.
pub(super) fn see_other(location: &str) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The webhook's quiet 200: nothing in the body, nothing to learn.
fn ok_quietly() -> Response {
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        "",
    )
        .into_response()
}

/// An event id echoed into a redirect path: the id grammar is enforced by
/// the store, but the redirect is built before that gate, so only path-safe
/// bytes may pass.
pub(super) fn esc_path(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Sorts the posted pairs into [`CheckoutBody`]; unknown fields are ignored
/// (a stale page from an older publish must not lose a sale).
fn decode(posted: &[(String, String)]) -> CheckoutBody {
    let mut body = CheckoutBody::default();
    for (key, value) in posted {
        match key.as_str() {
            "quantity" => body.quantity.clone_from(value),
            "name" => body.name.clone_from(value),
            "email" => body.email.clone_from(value),
            "website" => body.website.clone_from(value),
            _ => {}
        }
    }
    body
}

/// A shop document: live state and personal handles — never cacheable, never
/// indexed.
pub(super) fn page(status: StatusCode, strings: &UiStrings, title: &str, body: &str) -> Response {
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

    #[test]
    fn the_checkout_body_decodes_and_ignores_strangers() {
        let posted: Vec<(String, String)> = [
            ("quantity", "2"),
            ("name", "Maud Adams"),
            ("surprise", "field"),
            ("email", "maud@example.org"),
            ("website", ""),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
        let body = decode(&posted);
        assert_eq!(body.quantity, "2");
        assert_eq!(body.name, "Maud Adams");
        assert_eq!(body.email, "maud@example.org");
        assert!(body.website.is_empty());
    }

    #[test]
    fn the_when_is_the_utc_wall_clock() {
        let event = PublicTicketEvent {
            id: alo_store::SiteTicketEventId::new("ev-1"),
            name: "Letterpress workshop".to_owned(),
            starts_at: time::macros::datetime!(2026-09-16 17:30 UTC),
            unit_price_cents: 8_500,
            currency: "EUR".to_owned(),
            remaining: 4,
        };
        assert_eq!(when(&event), "2026-09-16 17:30 UTC");
    }

    #[test]
    fn a_redirect_path_keeps_only_id_bytes() {
        assert_eq!(esc_path("ev-1_A"), "ev-1_A");
        assert_eq!(esc_path("ev;drop /../x"), "evdropx");
    }
}
