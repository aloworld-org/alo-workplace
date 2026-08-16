//! `/shop` — the public stock shop (ADR 0041, item S3.05a3): the live goods
//! listing, the offer page with its buy form and delivery address, the
//! hosted-payment handoff, and the buyer's return page.
//!
//! This is the ticket door's machinery ([`super::tickets`]) pointed at goods
//! on a shelf, and it deliberately reuses that module's helpers — the Host
//! resolution, the page shell, the refusal, redirect and rate-limit shapes —
//! rather than growing its own. What differs is what must: every price and
//! every shelf count is the owning seams' answer at the moment of the
//! request (`no-store`, never cached bytes), the buy form carries a delivery
//! address because a stock sale ships somewhere, and "paid" means the goods
//! were claimed off the real shelf by the S3.05a2 machinery — or the order
//! closes visibly with the store's own refund sentence, which the return
//! page owes the buyer verbatim.
//!
//! The wire contract is the ticket door's: unresolvable host or foreign ids
//! are one uniform 404, an unreadable body 400, an oversized body 413, a
//! rate-limited visitor 429 with `Retry-After`, a refused purchase the
//! store's own sentence (400 validation, 409 goods), and the honeypot's
//! silent drop answers exactly like a real buyer's first hop and reserves
//! nothing. An installation with no payment provider says so instead of
//! rendering a form that could only fail. The provider's webhook is the
//! shared `POST /_alo/pay` ([`super::tickets::webhook`]), which resolves
//! stock orders as well as ticket ones.

use std::sync::Arc;
use std::time::Instant;

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use time::OffsetDateTime;

use alo_store::inv_stock_sale::STOCK_HOLD_MAX_UNITS;
use alo_store::{
    PublicStockItem, PublishedSite, ShipTo, SitePaymentError, SitePaymentRequest,
    SiteStockOrderState, StoreError,
};

use crate::render::html::esc;
use crate::render::money::format_price;
use crate::render::{UiStrings, strings_for};

use super::AppState;
use super::tickets::{
    esc_path, host_header, page, rate_limited, refusal_page, refused, resolve_host, see_other,
    site_host,
};

/// The fixed field contract of the buy form: how many, who is buying, where
/// the goods go, and the visually hidden honeypot (`website`), exactly as on
/// the ticket, form, order and booking doors.
#[derive(Default)]
struct CheckoutBody {
    quantity: String,
    name: String,
    email: String,
    address: String,
    city: String,
    postcode: String,
    country: String,
    website: String,
}

/// `GET /shop` — the live listing: every offer the owning seams answer for
/// right now, and the site's own delivery price under them.
pub(super) async fn listing(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let host = host_header(&request);
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let now = OffsetDateTime::now_utc();
    let items = match state.store.public_stock_items(&site, now).await {
        Ok(items) => items,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "shop listing read failed");
            return super::unavailable();
        }
    };
    let mut body = String::new();
    if items.is_empty() {
        body.push_str(&format!("<p>{}</p>\n", esc(strings.shop_empty)));
    } else {
        body.push_str("<ul class=\"shop-list\">\n");
        for item in &items {
            body.push_str("<li>\n");
            body.push_str(&format!(
                "<h2><a href=\"/shop/{}\">{}</a></h2>\n",
                esc(item.id.as_str()),
                esc(&item.name)
            ));
            push_item_facts(&mut body, item, strings);
            body.push_str("</li>\n");
        }
        body.push_str("</ul>\n");
        let currency = items[0].currency.clone();
        push_delivery_line(&mut body, &state, &site, &currency, strings).await;
    }
    page(StatusCode::OK, strings, strings.shop_title, &body)
}

/// `GET /shop/{item}` — one offer and, while goods and a provider exist, the
/// form that buys them.
pub(super) async fn offer(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
    request: Request,
) -> Response {
    let host = host_header(&request);
    let Some(site) = resolve_host(&state, host.as_deref()).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let strings = strings_for(&site.default_locale);
    let Some(item) = read_item(&state, &site, &item_id).await else {
        return super::not_found(state.unknown_host.clone());
    };
    let mut body = String::new();
    push_item_facts(&mut body, &item, strings);
    push_delivery_line(&mut body, &state, &site, &item.currency, strings).await;
    if item.available_units <= 0 {
        // The facts above already said "sold out"; nothing to add but no form.
    } else if state.payments.is_none() {
        // No provider is wired into this installation: the honest sentence,
        // never a form whose submit could only fail.
        body.push_str(&format!("<p>{}</p>\n", esc(strings.shop_unconfigured)));
    } else {
        push_buy_form(&mut body, &item, strings);
    }
    body.push_str(&format!(
        "<p><a href=\"/shop\">{}</a></p>\n",
        esc(strings.shop_back)
    ));
    page(StatusCode::OK, strings, &item.name, &body)
}

/// `POST /shop/{item}` — starts a purchase (typo gate → reserve → order, the
/// only order that cannot oversell) and hands the buyer to the provider's
/// hosted page.
pub(super) async fn checkout(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
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
                |strings| strings.shop_title,
            )
            .await;
        }
        Err(_) => {
            return refusal_page(
                &state,
                host.as_deref(),
                StatusCode::BAD_REQUEST,
                |strings| strings.shop_title,
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
        return see_other(&format!("/shop/{}", esc_path(&item_id)));
    }

    let Ok(units) = body.quantity.trim().parse::<i64>() else {
        return refused(
            StatusCode::BAD_REQUEST,
            strings,
            strings.shop_title,
            strings.form_malformed_text,
        );
    };

    // The provider is checked before the store takes a hold: an unconfigured
    // shop must not reserve goods it can never sell.
    let Some(provider) = state.payments.clone() else {
        return refused(
            StatusCode::SERVICE_UNAVAILABLE,
            strings,
            strings.shop_title,
            strings.shop_unconfigured,
        );
    };

    let ship_to = ShipTo {
        line: body.address,
        city: body.city,
        postcode: body.postcode,
        country: body.country,
    };
    let now = OffsetDateTime::now_utc();
    let checkout = match state
        .store
        .public_begin_stock_checkout(
            &site,
            &item_id,
            units,
            &body.name,
            &body.email,
            &ship_to,
            now,
        )
        .await
    {
        Ok(Some(checkout)) => checkout,
        Ok(None) => return super::not_found(state.unknown_host.clone()),
        Err(StoreError::Validation(reason)) => {
            return refused(
                StatusCode::BAD_REQUEST,
                strings,
                strings.shop_title,
                &reason,
            );
        }
        Err(StoreError::Conflict(reason)) => {
            return refused(StatusCode::CONFLICT, strings, strings.shop_title, &reason);
        }
        Err(error) => {
            tracing::error!(site = %site.site, %error, "stock checkout failed");
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
        redirect_url: format!("https://{public_host}/shop/order/{}", checkout.order),
        webhook_url: format!("https://{public_host}/_alo/pay"),
    };
    let created = match provider.create_payment(payment).await {
        Ok(created) => created,
        Err(SitePaymentError::Unconfigured) => {
            return refused(
                StatusCode::SERVICE_UNAVAILABLE,
                strings,
                strings.shop_title,
                strings.shop_unconfigured,
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
        .public_open_stock_payment(
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

/// `GET /shop/order/{order}` — the buyer's return page: fetch, settle, say.
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
    } else if order.state == SiteStockOrderState::Paid {
        body.push_str(&format!("<p>{}</p>\n", esc(strings.shop_order_paid)));
    } else {
        // A closed order that is not a sale. When money moved and could not
        // be honoured, the machinery wrote the honest sentence — paid after
        // the hold lapsed, or the goods gone — and the buyer must hear it;
        // otherwise the payment simply came to nothing.
        match &order.failure {
            Some(failure) => body.push_str(&format!("<p>{}</p>\n", esc(failure))),
            None => body.push_str(&format!("<p>{}</p>\n", esc(strings.shop_order_dead))),
        }
        body.push_str(&format!(
            "<p><a href=\"/shop\">{}</a></p>\n",
            esc(strings.shop_back)
        ));
    }
    page(StatusCode::OK, strings, strings.shop_order_title, &body)
}

/// Applies a fetched status to whichever stock order holds `payment_id` —
/// the return page's half of what the shared webhook does.
async fn settle(
    state: &AppState,
    payment_id: &str,
    status: alo_store::SitePaymentStatus,
) -> Result<(), StoreError> {
    let Some(target) = state.store.public_stock_payment_target(payment_id).await? else {
        return Ok(());
    };
    state
        .store
        .public_settle_stock_payment(&target, status, OffsetDateTime::now_utc())
        .await
}

/// The facts every surface states about an offer: what one unit costs — and,
/// only when the shelf is empty, that it is empty. The count itself is never
/// printed: "how many are left" is the seam's business, and the form's `max`
/// already caps what a buyer can ask for.
fn push_item_facts(out: &mut String, item: &PublicStockItem, strings: &UiStrings) {
    let price = format_price(item.unit_price_cents, &item.currency, strings);
    if item.unit.is_empty() {
        out.push_str(&format!("<p class=\"shop-price\">{}</p>\n", esc(&price)));
    } else {
        out.push_str(&format!(
            "<p class=\"shop-price\">{} / {}</p>\n",
            esc(&price),
            esc(&item.unit)
        ));
    }
    if item.available_units <= 0 {
        out.push_str(&format!(
            "<p class=\"shop-sold-out\">{}</p>\n",
            esc(strings.tix_sold_out)
        ));
    }
}

/// The site's flat delivery price, stated wherever a buyer weighs the cost —
/// in the currency of the goods it accompanies (one tenant prices everything
/// in one accounting currency). A read that fails costs the line, never the
/// page.
async fn push_delivery_line(
    out: &mut String,
    state: &AppState,
    site: &PublishedSite,
    currency: &str,
    strings: &UiStrings,
) {
    let cents = match state.store.public_stock_shipping_cents(site).await {
        Ok(cents) => cents,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "shop shipping read failed");
            return;
        }
    };
    if cents <= 0 {
        return;
    }
    out.push_str(&format!(
        "<p class=\"shop-delivery\">+ {} {}</p>\n",
        esc(&format_price(cents, currency, strings)),
        esc(strings.shop_delivery)
    ));
}

/// The buy form: the honeypot, how many, who is buying, where it ships, one
/// button.
fn push_buy_form(out: &mut String, item: &PublicStockItem, strings: &UiStrings) {
    let max = item.available_units.min(STOCK_HOLD_MAX_UNITS);
    out.push_str(&format!(
        "<form action=\"/shop/{}\" method=\"post\">\n",
        esc(item.id.as_str())
    ));
    out.push_str(&format!(
        "<p class=\"hp\" aria-hidden=\"true\"><label for=\"shop-website\">{}</label>\
         <input id=\"shop-website\" name=\"website\" type=\"text\" tabindex=\"-1\" \
         autocomplete=\"off\"></p>\n",
        esc(strings.form_website)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-quantity\">{}</label>\
         <input id=\"shop-quantity\" name=\"quantity\" type=\"number\" min=\"1\" max=\"{max}\" \
         step=\"1\" value=\"1\" required inputmode=\"numeric\"></p>\n",
        esc(strings.shop_quantity_label)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-name\">{}</label>\
         <input id=\"shop-name\" name=\"name\" type=\"text\" required maxlength=\"200\" \
         autocomplete=\"name\"></p>\n",
        esc(strings.form_name)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-email\">{}</label>\
         <input id=\"shop-email\" name=\"email\" type=\"email\" required maxlength=\"254\" \
         autocomplete=\"email\"></p>\n",
        esc(strings.form_email)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-address\">{}</label>\
         <input id=\"shop-address\" name=\"address\" type=\"text\" required maxlength=\"200\" \
         autocomplete=\"street-address\"></p>\n",
        esc(strings.shop_address_label)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-city\">{}</label>\
         <input id=\"shop-city\" name=\"city\" type=\"text\" required maxlength=\"100\" \
         autocomplete=\"address-level2\"></p>\n",
        esc(strings.shop_city_label)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-postcode\">{}</label>\
         <input id=\"shop-postcode\" name=\"postcode\" type=\"text\" required maxlength=\"20\" \
         autocomplete=\"postal-code\"></p>\n",
        esc(strings.shop_postcode_label)
    ));
    out.push_str(&format!(
        "<p><label for=\"shop-country\">{}</label>\
         <input id=\"shop-country\" name=\"country\" type=\"text\" required minlength=\"2\" \
         maxlength=\"2\" autocomplete=\"country\"></p>\n",
        esc(strings.shop_country_label)
    ));
    out.push_str(&format!(
        "<p><button type=\"submit\">{}</button></p>\n</form>\n",
        esc(strings.tix_pay)
    ));
}

async fn read_item(
    state: &AppState,
    site: &PublishedSite,
    item_id: &str,
) -> Option<PublicStockItem> {
    match state
        .store
        .public_stock_item(site, item_id, OffsetDateTime::now_utc())
        .await
    {
        Ok(item) => item,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "shop offer read failed");
            None
        }
    }
}

async fn read_order(
    state: &AppState,
    site: &PublishedSite,
    order_id: &str,
) -> Option<alo_store::PublicStockOrderStatus> {
    match state.store.public_stock_order(site, order_id).await {
        Ok(order) => order,
        Err(error) => {
            tracing::error!(site = %site.site, %error, "stock order read failed");
            None
        }
    }
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
            "address" => body.address.clone_from(value),
            "city" => body.city.clone_from(value),
            "postcode" => body.postcode.clone_from(value),
            "country" => body.country.clone_from(value),
            "website" => body.website.clone_from(value),
            _ => {}
        }
    }
    body
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
            ("address", "Keizersgracht 1"),
            ("city", "Amsterdam"),
            ("postcode", "1015 CS"),
            ("country", "nl"),
            ("website", ""),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
        let body = decode(&posted);
        assert_eq!(body.quantity, "2");
        assert_eq!(body.name, "Maud Adams");
        assert_eq!(body.email, "maud@example.org");
        assert_eq!(body.address, "Keizersgracht 1");
        assert_eq!(body.city, "Amsterdam");
        assert_eq!(body.postcode, "1015 CS");
        assert_eq!(body.country, "nl");
        assert!(body.website.is_empty());
    }

    #[test]
    fn item_facts_price_a_unit_and_say_sold_out_only_when_it_is() {
        let item = |available: i64| PublicStockItem {
            id: alo_store::SiteShopItemId::new("it-1"),
            name: "Field guide".to_owned(),
            unit: "piece".to_owned(),
            unit_price_cents: 2_400,
            currency: "EUR".to_owned(),
            available_units: available,
        };
        let mut stocked = String::new();
        push_item_facts(&mut stocked, &item(3), &crate::render::EN);
        assert!(stocked.contains("/ piece"), "{stocked}");
        assert!(!stocked.contains("Sold out"), "{stocked}");
        let mut empty = String::new();
        push_item_facts(&mut empty, &item(0), &crate::render::EN);
        assert!(empty.contains("Sold out"), "{empty}");
    }
}
