//! In-process integration tests of `GET /b/:booking_id` and
//! `POST /b/:booking_id`: real fixtures through the real store into the compose
//! Postgres, real requests through the real router.
//!
//! The mandatory isolation case is the foreign, unknown, unpublished and
//! switched-off service id yielding one clean 404, plus proof that a booking
//! lands only in the owning tenant. The rest pin the wire contract: a day page
//! that offers exactly the free times, the honeypot's silent drop, the taken
//! slot's 409, a refused answer's 400, and the whole arc from a published page
//! to an appointment in the owner's calendar.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{
    AccountStore, BlobStore, SiteBookingField, SiteBookingFieldKind, SiteBookingId,
    SiteBookingInput, SiteBookingWindow, SiteId, SitePublicStore, Store,
};
use serde_json::json;
use time::{Date, Duration, OffsetDateTime};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5433/alo".to_owned())
}

async fn harness() -> (Store, Arc<AppState>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url())
        .await
        .expect("connect to test postgres (is compose up? is DATABASE_URL set?)");
    let blobs = BlobStore::in_memory(25 * 1024 * 1024);
    let store = Store::new(pool.clone(), blobs.clone());
    store.migrate().await.expect("run migrations");
    let state = AppState::new(
        SitePublicStore::new(pool, blobs),
        APEX.to_owned(),
        b"booking-flow-tests-analytics-secret",
    );
    (store, state)
}

async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@bookings.test"))
        .await
        .unwrap();
    store.for_account(tenant, user)
}

fn unique(tag: &str) -> String {
    format!(
        "{tag}-{}x",
        SiteId::generate().as_str().to_lowercase().replace('_', "-")
    )
}

/// The next Wednesday at least a week out, so no notice or horizon rule ever
/// interferes and the day is stable whenever the suite runs.
fn wednesday_ahead() -> Date {
    let mut day = OffsetDateTime::now_utc().date() + Duration::days(7);
    while day.weekday() != time::Weekday::Wednesday {
        day += Duration::days(1);
    }
    day
}

/// A live site offering one bookable service on its home page, open Wednesdays
/// 09:00–11:00 in Brussels at half an hour each.
async fn live_site_with_booking(
    acc: &AccountStore,
    tag: &str,
    fields: &[SiteBookingField],
    active: bool,
) -> (SiteId, SiteBookingId, String) {
    let subdomain = unique(tag);
    let site = acc.create_site("Studio", &subdomain).await.unwrap();
    let home = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    let calendar = acc.ensure_personal_calendar().await.unwrap();
    let hours = [SiteBookingWindow {
        weekday: 3,
        start_minute: 540,
        end_minute: 660,
    }];
    let booking = acc
        .create_site_booking(
            &site,
            &SiteBookingInput {
                name: "Consultation",
                description: Some("Half an hour, in the studio."),
                calendar: &calendar,
                time_zone: "Europe/Brussels",
                duration_minutes: 30,
                buffer_minutes: 0,
                notice_minutes: 0,
                horizon_days: 365,
                location: Some("Second floor"),
                hours: &hours,
                fields,
                active,
            },
        )
        .await
        .unwrap();
    acc.set_page_sections(
        &site,
        &home,
        json!({
            "schema_version": 1,
            "sections": [{
                "type": "booking",
                "booking_id": booking.as_str(),
                "heading": "Come and talk to us"
            }]
        }),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, booking, subdomain)
}

async fn get_day(state: &Arc<AppState>, booking: &str, date: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(format!("/b/{booking}?date={date}"))
        .header(header::HOST, format!("whatever.{APEX}"))
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn post_booking(state: &Arc<AppState>, booking: &str, client: &str, body: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/b/{booking}"))
        .header(header::HOST, format!("whatever.{APEX}"))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_owned()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The RFC 3339 instant a slot at `hour_local` on `day` is, in Brussels — the
/// exact value the day page renders into its radio buttons.
fn slot_value(day: Date, page: &str, hour_local: &str) -> String {
    let needle = format!(">{hour_local}<");
    assert!(
        page.contains(&needle),
        "the page offers {hour_local} on {day}: {page}"
    );
    // The value attribute immediately precedes the label carrying the clock.
    let cut = page.split(&needle).next().unwrap();
    let value = cut.rsplit("value=\"").next().unwrap();
    value.split('"').next().unwrap().to_owned()
}

#[tokio::test]
async fn the_day_page_offers_the_free_times_and_a_booking_reaches_the_owners_calendar() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "booking-arc").await;
    let outsider = fresh_account(&store, "booking-arc-outsider").await;
    let (site, booking, _) = live_site_with_booking(&owner, "booking-arc", &[], true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();

    let response = get_day(&state, booking.as_str(), &date).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap(),
        "no-store",
        "free time is never cached"
    );
    let page = body_string(response).await;
    assert!(page.contains("Consultation"), "{page}");
    assert!(page.contains("Second floor"), "{page}");
    for clock in ["09:00", "09:30", "10:00", "10:30"] {
        assert!(
            page.contains(&format!(">{clock}<")),
            "missing {clock}: {page}"
        );
    }
    assert!(!page.contains(">11:00<"), "nothing runs past the window");

    let slot = slot_value(day, &page, "09:30");
    let response = post_booking(
        &state,
        booking.as_str(),
        "203.0.113.21",
        &format!(
            "slot={}&name=Ada+Lovelace&email=ada%40example.test&website=",
            urlencode(&slot)
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let confirmation = body_string(response).await;
    assert!(
        confirmation.contains("Appointment booked"),
        "{confirmation}"
    );
    assert!(confirmation.contains("09:30"), "{confirmation}");

    // The taken time is gone from the next visitor's page…
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(!page.contains(">09:30<"), "the slot is taken: {page}");
    assert!(page.contains(">10:00<"), "the rest still stands: {page}");

    // …it is in the owner's calendar…
    let from = day.with_hms(0, 0, 0).unwrap().assume_utc();
    let events = owner
        .events_in_range(from, from + Duration::days(1))
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].summary.contains("Ada Lovelace"), "{events:?}");

    // …and nowhere near anyone else's.
    assert!(
        outsider
            .events_in_range(from, from + Duration::days(1))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        outsider.site_bookings(&site).await.unwrap().is_empty(),
        "a foreign account sees no service of ours"
    );
}

#[tokio::test]
async fn the_same_time_cannot_be_booked_twice() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "booking-twice").await;
    let (_site, booking, _) = live_site_with_booking(&owner, "booking-twice", &[], true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    let slot = slot_value(day, &page, "09:00");
    let body = format!(
        "slot={}&name=Ada&email=ada%40example.test&website=",
        urlencode(&slot)
    );

    let first = post_booking(&state, booking.as_str(), "203.0.113.31", &body).await;
    assert_eq!(first.status(), StatusCode::OK);
    let second = post_booking(&state, booking.as_str(), "203.0.113.32", &body).await;
    assert_eq!(
        second.status(),
        StatusCode::CONFLICT,
        "the second visitor is told, not silently double-booked"
    );
    let said = body_string(second).await;
    assert!(said.contains("taken") || said.contains("free"), "{said}");
}

#[tokio::test]
async fn a_service_nobody_may_book_is_one_uniform_absence() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "booking-absent").await;
    let (site, offered, _) = live_site_with_booking(&owner, "booking-absent", &[], true).await;
    let (_asleep_site, asleep, _) =
        live_site_with_booking(&owner, "booking-asleep", &[], false).await;
    let day = wednesday_ahead()
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();

    // Unknown, malformed, and switched off are the same 404 on both verbs.
    for id in [
        "not-a-service",
        // An injection attempt, percent-encoded as a client would have to send
        // it; the store's own suite sends the raw string.
        "b%27%3B%20drop%20table%20sites%3B%20--",
        asleep.as_str(),
    ] {
        assert_eq!(
            get_day(&state, id, &day).await.status(),
            StatusCode::NOT_FOUND,
            "{id} must be indistinguishable from unknown"
        );
        assert_eq!(
            post_booking(
                &state,
                id,
                "203.0.113.41",
                "slot=2026-09-16T07%3A00%3A00Z&name=Ada&email=ada%40example.test&website="
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }

    // And a service whose site is unpublished stops answering at once.
    owner.unpublish_site(&site).await.unwrap();
    assert_eq!(
        get_day(&state, offered.as_str(), &day).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn the_honeypot_answers_like_success_and_a_refused_answer_says_what_is_wrong() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "booking-guards").await;
    let fields = [SiteBookingField {
        key: "phone".to_owned(),
        label: "Phone number".to_owned(),
        kind: SiteBookingFieldKind::Phone,
        required: true,
        options: Vec::new(),
    }];
    let (site, booking, _) = live_site_with_booking(&owner, "booking-guards", &fields, true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(
        page.contains("Phone number"),
        "the question is asked: {page}"
    );
    let slot = slot_value(day, &page, "09:00");

    // A bot fills the field no human sees: answered exactly like success, and
    // nothing is written.
    let trapped = post_booking(
        &state,
        booking.as_str(),
        "203.0.113.51",
        &format!(
            "slot={}&name=Bot&email=bot%40example.test&q-phone=1&website=https%3A%2F%2Fspam.test",
            urlencode(&slot)
        ),
    )
    .await;
    assert_eq!(trapped.status(), StatusCode::OK);
    assert!(body_string(trapped).await.contains("Appointment booked"));

    // The required question, unanswered: 400 carrying the label the visitor read.
    let refused = post_booking(
        &state,
        booking.as_str(),
        "203.0.113.52",
        &format!(
            "slot={}&name=Ada&email=ada%40example.test&website=",
            urlencode(&slot)
        ),
    )
    .await;
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    assert!(body_string(refused).await.contains("Phone number"));

    // A body that is not a booking at all: 400, no internals.
    let malformed = post_booking(
        &state,
        booking.as_str(),
        "203.0.113.53",
        "slot=yesterday&name=Ada&email=ada%40example.test",
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    // Nothing of the above reached the owner's calendar.
    let from = day.with_hms(0, 0, 0).unwrap().assume_utc();
    assert!(
        owner
            .events_in_range(from, from + Duration::days(1))
            .await
            .unwrap()
            .is_empty(),
        "neither the bot nor the refused bookings were written"
    );
    let _ = site;
}

#[tokio::test]
async fn a_closed_day_says_so_rather_than_offering_a_form() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "booking-closed").await;
    let (_site, booking, _) = live_site_with_booking(&owner, "booking-closed", &[], true).await;
    // The Thursday after the open Wednesday.
    let closed = wednesday_ahead() + Duration::days(1);
    let date = closed
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(page.contains("Nothing free on that day"), "{page}");
    assert!(!page.contains("<form"), "no form to submit: {page}");
}

/// Percent-encodes the few characters an RFC 3339 instant carries that a form
/// body must not.
fn urlencode(value: &str) -> String {
    value.replace(':', "%3A").replace('+', "%2B")
}

// ---- Booking from the conversation (S3.03b) ----------------------------

/// One JSON POST to the conversation's booking endpoint, on a given site's
/// own host.
async fn post_chat_book(
    state: &Arc<AppState>,
    subdomain: &str,
    client: &str,
    body: serde_json::Value,
) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri("/_alo/chat/book")
        .header(header::HOST, format!("{subdomain}.{APEX}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", client)
        .body(Body::from(body.to_string()))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn get_on(state: &Arc<AppState>, subdomain: &str, path: &str) -> Response {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header(header::HOST, format!("{subdomain}.{APEX}"))
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn post_on(state: &Arc<AppState>, subdomain: &str, path: &str, client: &str) -> Response {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, format!("{subdomain}.{APEX}"))
        .header("x-forwarded-for", client)
        .body(Body::empty())
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

async fn json_body(response: Response) -> serde_json::Value {
    serde_json::from_str(&body_string(response).await).unwrap()
}

#[tokio::test]
async fn booking_from_the_conversation_reserves_and_is_reversible_by_its_links() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "chat-book").await;
    let (site, booking, subdomain) = live_site_with_booking(&owner, "chat-book", &[], true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    let slot = slot_value(day, &page, "09:00");

    // The conversation books the slot: real reservation, and the two
    // reversibility handles in the same reply.
    let response = post_chat_book(
        &state,
        &subdomain,
        "9.1.0.1",
        json!({
            "visitor": "visitor-token-0001",
            "service": booking.as_str(),
            "slot": slot,
            "name": "Ada Lovelace",
            "email": "ada@example.test",
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let booked = json_body(response).await;
    assert_eq!(booked["state"], "booked");
    assert_eq!(booked["service"], "Consultation");
    let manage = booked["managePath"].as_str().unwrap().to_owned();
    let ics = booked["icsPath"].as_str().unwrap().to_owned();
    assert!(manage.starts_with("/b/manage/"));
    assert!(ics.ends_with("/calendar.ics"));

    // The owner's transcript records the act with the published fact it
    // used — the service and the instant, never the visitor (S3.03e).
    let transcript = owner.site_chat_actions(&site).await.unwrap();
    assert_eq!(transcript.len(), 1);
    assert_eq!(transcript[0].kind, alo_store::SiteChatActionKind::Booked);
    assert_eq!(transcript[0].fact.as_deref(), Some("Consultation"));
    let slot_instant =
        time::OffsetDateTime::parse(&slot, &time::format_description::well_known::Rfc3339).unwrap();
    assert_eq!(transcript[0].slot_at, Some(slot_instant));

    // The slot is gone for the next visitor…
    let after = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(
        !after.contains(">09:00<"),
        "the slot must be taken: {after}"
    );
    // …and booking it through the conversation again says taken.
    let clash = post_chat_book(
        &state,
        &subdomain,
        "9.1.0.2",
        json!({
            "visitor": "visitor-token-0002",
            "service": booking.as_str(),
            "slot": slot,
            "name": "Grace Hopper",
            "email": "grace@example.test",
        }),
    )
    .await;
    assert_eq!(clash.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(clash).await["state"], "taken");

    // The manage page shows the appointment with its cancel button.
    let page = get_on(&state, &subdomain, &manage).await;
    assert_eq!(page.status(), StatusCode::OK);
    let page = body_string(page).await;
    assert!(page.contains("Consultation"), "{page}");
    assert!(page.contains("/cancel"), "{page}");

    // The .ics puts the meeting in the visitor's calendar and carries the
    // cancellation link inside the entry itself.
    let calendar = get_on(&state, &subdomain, &ics).await;
    assert_eq!(calendar.status(), StatusCode::OK);
    assert_eq!(
        calendar
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "text/calendar; charset=utf-8"
    );
    let document = body_string(calendar).await;
    assert!(document.starts_with("BEGIN:VCALENDAR\r\n"), "{document}");
    assert!(document.contains("SUMMARY:Consultation"), "{document}");
    assert!(document.contains("DTSTART:"), "{document}");
    // Unfold (RFC 5545 §3.1) before matching: a long host splits the URL
    // across folded lines.
    let unfolded = document.replace("\r\n ", "");
    assert!(
        unfolded.contains(&format!("https://{subdomain}.{APEX}{manage}")),
        "the undo travels with the meeting: {document}"
    );

    // Cancelling by the link frees the slot again.
    let cancelled = post_on(&state, &subdomain, &format!("{manage}/cancel"), "9.1.0.3").await;
    assert_eq!(cancelled.status(), StatusCode::OK);
    let text = body_string(cancelled).await;
    assert!(text.contains("cancelled"), "{text}");
    let freed = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(
        freed.contains(">09:00<"),
        "cancelling frees the time: {freed}"
    );
    // A cancelled appointment has nothing left to import.
    assert_eq!(
        get_on(&state, &subdomain, &ics).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_conversation_booking_is_scoped_to_the_serving_site() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "chat-book-scope-a").await;
    let stranger = fresh_account(&store, "chat-book-scope-b").await;
    let (_site_a, booking_a, sub_a) = live_site_with_booking(&owner, "cb-scope-a", &[], true).await;
    let (_site_b, _booking_b, sub_b) =
        live_site_with_booking(&stranger, "cb-scope-b", &[], true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking_a.as_str(), &date).await).await;
    let slot = slot_value(day, &page, "09:00");

    // Site B's widget cannot book site A's service: the id resolves among
    // the serving site's own services only.
    let crossed = post_chat_book(
        &state,
        &sub_b,
        "9.2.0.1",
        json!({
            "visitor": "visitor-token-0003",
            "service": booking_a.as_str(),
            "slot": slot,
            "name": "Mallory",
            "email": "mallory@example.test",
        }),
    )
    .await;
    assert_eq!(crossed.status(), StatusCode::NOT_FOUND);

    // Book it at home, then probe the token from the foreign host: the
    // manage page, the calendar and the cancel are all the same 404 there,
    // and the appointment survives the probing untouched.
    let booked = json_body(
        post_chat_book(
            &state,
            &sub_a,
            "9.2.0.2",
            json!({
                "visitor": "visitor-token-0004",
                "service": booking_a.as_str(),
                "slot": slot,
                "name": "Ada Lovelace",
                "email": "ada@example.test",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(booked["state"], "booked");
    let manage = booked["managePath"].as_str().unwrap().to_owned();
    let ics = booked["icsPath"].as_str().unwrap().to_owned();
    for response in [
        get_on(&state, &sub_b, &manage).await,
        get_on(&state, &sub_b, &ics).await,
        post_on(&state, &sub_b, &format!("{manage}/cancel"), "9.2.0.3").await,
    ] {
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let still = get_on(&state, &sub_a, &manage).await;
    assert_eq!(still.status(), StatusCode::OK);
    assert!(body_string(still).await.contains("/cancel"));
}

#[tokio::test]
async fn a_conversation_booking_refuses_bad_input_with_the_stores_own_words() {
    let (store, state) = harness().await;
    let owner = fresh_account(&store, "chat-book-guards").await;
    let (_site, booking, subdomain) = live_site_with_booking(&owner, "cb-guards", &[], true).await;
    let day = wednesday_ahead();
    let date = day
        .format(&time::macros::format_description!("[year]-[month]-[day]"))
        .unwrap();
    let page = body_string(get_day(&state, booking.as_str(), &date).await).await;
    let slot = slot_value(day, &page, "09:00");

    // A blank name: the store's sentence rides to the widget verbatim.
    let refused = post_chat_book(
        &state,
        &subdomain,
        "9.3.0.1",
        json!({
            "visitor": "visitor-token-0005",
            "service": booking.as_str(),
            "slot": slot,
            "name": "  ",
            "email": "ada@example.test",
        }),
    )
    .await;
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    let refused = json_body(refused).await;
    assert_eq!(refused["state"], "invalid");
    assert!(
        refused["detail"].as_str().unwrap().contains("name"),
        "{refused}"
    );

    // A malformed visitor token never reaches the store.
    let bad_token = post_chat_book(
        &state,
        &subdomain,
        "9.3.0.2",
        json!({
            "visitor": "x",
            "service": booking.as_str(),
            "slot": slot,
            "name": "Ada",
            "email": "ada@example.test",
        }),
    )
    .await;
    assert_eq!(bad_token.status(), StatusCode::BAD_REQUEST);

    // An unknown service on the right host is the uniform absence.
    let unknown = post_chat_book(
        &state,
        &subdomain,
        "9.3.0.3",
        json!({
            "visitor": "visitor-token-0006",
            "service": "no-such-service-id",
            "slot": slot,
            "name": "Ada",
            "email": "ada@example.test",
        }),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);

    // Nothing was reserved by any of it.
    let untouched = body_string(get_day(&state, booking.as_str(), &date).await).await;
    assert!(untouched.contains(">09:00<"), "{untouched}");
}
