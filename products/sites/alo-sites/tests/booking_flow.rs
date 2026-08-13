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
) -> (SiteId, SiteBookingId) {
    let site = acc.create_site("Studio", &unique(tag)).await.unwrap();
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
    (site, booking)
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
    let (site, booking) = live_site_with_booking(&owner, "booking-arc", &[], true).await;
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
    let (_site, booking) = live_site_with_booking(&owner, "booking-twice", &[], true).await;
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
    let (site, offered) = live_site_with_booking(&owner, "booking-absent", &[], true).await;
    let (_asleep_site, asleep) = live_site_with_booking(&owner, "booking-asleep", &[], false).await;
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
    let (site, booking) = live_site_with_booking(&owner, "booking-guards", &fields, true).await;
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
    let (_site, booking) = live_site_with_booking(&owner, "booking-closed", &[], true).await;
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
