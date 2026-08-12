//! Password-protected pages on the public service (ADR 0036, S2.06a): real
//! fixtures written through the real store into the compose Postgres, real
//! requests through the real router.
//!
//! What is proved here is the gate itself: a protected page never hands its
//! bytes to anyone who has not passed the password, the answer it does hand
//! out is safe to sit in front of a shared cache, a session opens exactly the
//! page it was minted for on the host it was minted on, changing the password
//! ends it, and guessing is rate-limited.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use serde_json::json;
use tower::ServiceExt;

use alo_sites::serve::{AppState, app};
use alo_store::{AccountStore, BlobStore, SiteId, SitePageId, SitePublicStore, Store};

/// The apex the tests serve under (`SITES_DOMAIN` in production).
const APEX: &str = "sites.test";

/// The password every fixture page is protected with.
const PASSWORD: &str = "kaneelstokjes 2026";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://alo:alo-dev-only@127.0.0.1:5432/alo".to_owned())
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
        b"protected-pages-tests-deployment-secret",
    );
    (store, state)
}

async fn fresh_account(store: &Store, tag: &str) -> AccountStore {
    let tenant = store.create_tenant(&format!("t-{tag}")).await.unwrap();
    let user = store
        .for_tenant(tenant.clone())
        .create_user(&format!("{tag}@protected.test"))
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

/// A published site: a public home page carrying `marker`, and a `/prices`
/// page carrying `secret` that the caller may then protect.
async fn publish_site(
    acc: &AccountStore,
    sub: &str,
    marker: &str,
    secret: &str,
) -> (SiteId, SitePageId) {
    let site = acc.create_site("Acme", sub).await.unwrap();
    let home = acc.create_site_page(&site, "Home", "", true).await.unwrap();
    acc.set_page_sections(
        &site,
        &home,
        json!({"schema_version": 1, "sections": [{"type": "hero", "heading": marker}]}),
    )
    .await
    .unwrap();
    let prices = acc
        .create_site_page(&site, "Prices", "prices", false)
        .await
        .unwrap();
    acc.set_page_sections(
        &site,
        &prices,
        json!({"schema_version": 1, "sections": [{"type": "hero", "heading": secret}]}),
    )
    .await
    .unwrap();
    acc.publish_site(&site).await.unwrap();
    (site, prices)
}

async fn get(state: &Arc<AppState>, host: &str, path: &str, cookie: Option<&str>) -> Response {
    let mut request = Request::builder().uri(path).header(header::HOST, host);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    app(Arc::clone(state))
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// One password attempt, as the unlock form posts it. `client` becomes the
/// forwarded address the rate limiter keys on.
async fn attempt(
    state: &Arc<AppState>,
    host: &str,
    path: &str,
    password: &str,
    client: &str,
) -> Response {
    let body = format!("password={}", urlencode(password));
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::HOST, host)
        .header("x-forwarded-for", client)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    app(Arc::clone(state)).oneshot(request).await.unwrap()
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_owned(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

async fn body_string(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn header_str<'a>(response: &'a Response, name: &header::HeaderName) -> &'a str {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap())
        .unwrap_or("")
}

/// The `Cookie` header a browser would send back after a `Set-Cookie`.
fn echo(response: &Response) -> String {
    header_str(response, &header::SET_COOKIE)
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The whole visitor arc: asked, refused, opened, and — once the owner changes
/// the password — asked again.
#[tokio::test]
async fn a_protected_page_opens_only_with_the_password_and_closes_when_it_changes() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "gate").await;
    let sub = unique("gate");
    let host = format!("{sub}.{APEX}");
    let (site, prices) = publish_site(&acc, &sub, "PUBLIC-HOME", "SECRET-PRICES").await;

    // Before any password, the page is an ordinary cacheable public page.
    let open = get(&state, &host, "/prices", None).await;
    assert_eq!(open.status(), StatusCode::OK);
    assert!(body_string(open).await.contains("SECRET-PRICES"));

    acc.set_site_page_password(&site, &prices, PASSWORD)
        .await
        .unwrap();

    // ---- the ask -----------------------------------------------------------
    let asked = get(&state, &host, "/prices", None).await;
    assert_eq!(asked.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        header_str(&asked, &header::CACHE_CONTROL),
        "no-store",
        "the unlock screen must never be cached"
    );
    assert_eq!(header_str(&asked, &header::VARY), "Cookie");
    assert_eq!(header_str(&asked, &header::WWW_AUTHENTICATE), "Form");
    assert!(
        asked.headers().get(header::ETAG).is_none(),
        "a gated answer carries no validator to revalidate against"
    );
    let screen = body_string(asked).await;
    assert!(screen.contains("This page is protected"), "{screen}");
    assert!(
        screen.contains("name=\"password\""),
        "the screen asks for the password: {screen}"
    );
    assert!(
        !screen.contains("SECRET-PRICES") && !screen.contains("Prices"),
        "nothing behind the password leaks into the ask: {screen}"
    );
    // The rest of the site is untouched.
    let home = get(&state, &host, "/", None).await;
    assert_eq!(home.status(), StatusCode::OK);
    assert_eq!(
        header_str(&home, &header::CACHE_CONTROL),
        "public, max-age=60"
    );
    assert!(body_string(home).await.contains("PUBLIC-HOME"));

    // ---- a wrong password ---------------------------------------------------
    let refused = attempt(&state, &host, "/prices", "not the password", "203.0.113.7").await;
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
    assert!(
        refused.headers().get(header::SET_COOKIE).is_none(),
        "a refusal mints nothing"
    );
    let refused_body = body_string(refused).await;
    assert!(
        refused_body.contains("does not open this page"),
        "{refused_body}"
    );
    assert!(!refused_body.contains("SECRET-PRICES"), "{refused_body}");

    // ---- the right password -------------------------------------------------
    let opened = attempt(&state, &host, "/prices", PASSWORD, "203.0.113.7").await;
    assert_eq!(opened.status(), StatusCode::SEE_OTHER);
    assert_eq!(header_str(&opened, &header::LOCATION), "/prices");
    assert_eq!(header_str(&opened, &header::CACHE_CONTROL), "no-store");
    let set_cookie = header_str(&opened, &header::SET_COOKIE).to_owned();
    assert!(set_cookie.contains("HttpOnly"), "{set_cookie}");
    assert!(set_cookie.contains("Secure"), "{set_cookie}");
    assert!(set_cookie.contains("SameSite=Lax"), "{set_cookie}");
    assert!(
        !set_cookie.contains("kaneelstokjes"),
        "the session is not the password: {set_cookie}"
    );
    let cookie = echo(&opened);

    // ---- the page, for the visitor holding that session ---------------------
    let served = get(&state, &host, "/prices", Some(&cookie)).await;
    assert_eq!(served.status(), StatusCode::OK);
    assert_eq!(
        header_str(&served, &header::CACHE_CONTROL),
        "private, no-store",
        "an unlocked page must not be stored by any cache"
    );
    assert_eq!(header_str(&served, &header::VARY), "Cookie");
    assert!(served.headers().get(header::ETAG).is_none());
    assert!(body_string(served).await.contains("SECRET-PRICES"));

    // A tampered session is no session.
    let tampered = cookie.replace(|c: char| c.is_ascii_hexdigit(), "0");
    let refused = get(&state, &host, "/prices", Some(&tampered)).await;
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);

    // ---- changing the password ends the session -----------------------------
    acc.set_site_page_password(&site, &prices, "een heel ander wachtwoord")
        .await
        .unwrap();
    let closed = get(&state, &host, "/prices", Some(&cookie)).await;
    assert_eq!(closed.status(), StatusCode::UNAUTHORIZED);
    assert!(
        header_str(&closed, &header::SET_COOKIE).contains("Max-Age=0"),
        "the dead session is cleared instead of travelling forever"
    );
    assert!(!body_string(closed).await.contains("SECRET-PRICES"));

    // ---- lifting it makes the page public again -----------------------------
    acc.remove_site_page_password(&site, &prices).await.unwrap();
    let public = get(&state, &host, "/prices", None).await;
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(
        header_str(&public, &header::CACHE_CONTROL),
        "public, max-age=60"
    );
    assert!(body_string(public).await.contains("SECRET-PRICES"));

    acc.delete_site(&site).await.unwrap();
}

/// A session is bound to one page on one host: it cannot be carried to another
/// site, and another site's protected page cannot be opened with it.
#[tokio::test]
async fn a_session_opens_one_page_on_one_host_and_nothing_else() {
    let (store, state) = harness().await;
    let first = fresh_account(&store, "iso-one").await;
    let second = fresh_account(&store, "iso-two").await;
    let sub_a = unique("iso-a");
    let sub_b = unique("iso-b");
    let host_a = format!("{sub_a}.{APEX}");
    let host_b = format!("{sub_b}.{APEX}");
    let (site_a, prices_a) = publish_site(&first, &sub_a, "ALPHA-HOME", "ALPHA-PRICES").await;
    let (site_b, prices_b) = publish_site(&second, &sub_b, "BETA-HOME", "BETA-PRICES").await;
    first
        .set_site_page_password(&site_a, &prices_a, PASSWORD)
        .await
        .unwrap();
    second
        .set_site_page_password(&site_b, &prices_b, PASSWORD)
        .await
        .unwrap();

    let opened = attempt(&state, &host_a, "/prices", PASSWORD, "203.0.113.9").await;
    assert_eq!(opened.status(), StatusCode::SEE_OTHER);
    let cookie_a = echo(&opened);
    assert!(
        get(&state, &host_a, "/prices", Some(&cookie_a))
            .await
            .status()
            == StatusCode::OK
    );

    // The same cookie on the other tenant's host opens nothing — and the name
    // rewritten to that site's page id fails on the signature, which is the
    // property a browser's own cookie scoping is only the first line of.
    let closed = get(&state, &host_b, "/prices", Some(&cookie_a)).await;
    assert_eq!(closed.status(), StatusCode::UNAUTHORIZED);
    let carried = cookie_a.replace(prices_a.as_str(), prices_b.as_str());
    let forged = get(&state, &host_b, "/prices", Some(&carried)).await;
    assert_eq!(forged.status(), StatusCode::UNAUTHORIZED);
    assert!(!body_string(forged).await.contains("BETA-PRICES"));

    // And the home page of the other site is still perfectly public.
    let home_b = get(&state, &host_b, "/", None).await;
    assert_eq!(home_b.status(), StatusCode::OK);
    assert!(body_string(home_b).await.contains("BETA-HOME"));

    first.delete_site(&site_a).await.unwrap();
    second.delete_site(&site_b).await.unwrap();
}

/// Guessing is bounded per visitor, and a protected page is never advertised:
/// it stays out of the sitemap, and posting anywhere else is still a 405.
#[tokio::test]
async fn guesses_are_limited_and_protected_pages_are_not_advertised() {
    let (store, state) = harness().await;
    let acc = fresh_account(&store, "guess").await;
    let sub = unique("guess");
    let host = format!("{sub}.{APEX}");
    let (site, prices) = publish_site(&acc, &sub, "PUBLIC-HOME", "SECRET-PRICES").await;

    // The sitemap lists both pages while both are public.
    let sitemap = body_string(get(&state, &host, "/sitemap.xml", None).await).await;
    assert!(sitemap.contains("/prices"), "{sitemap}");

    acc.set_site_page_password(&site, &prices, PASSWORD)
        .await
        .unwrap();

    let sitemap = body_string(get(&state, &host, "/sitemap.xml", None).await).await;
    assert!(
        !sitemap.contains("/prices"),
        "a protected page is not offered to crawlers: {sitemap}"
    );
    assert!(
        sitemap.contains(&format!("https://{host}/")),
        "the public pages are still listed: {sitemap}"
    );

    // Posting to a page that carries no password is not a thing.
    let posted = attempt(&state, &host, "/", PASSWORD, "203.0.113.11").await;
    assert_eq!(posted.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(header_str(&posted, &header::ALLOW), "GET, HEAD");
    let posted = attempt(&state, &host, "/nowhere", PASSWORD, "203.0.113.11").await;
    assert_eq!(posted.status(), StatusCode::METHOD_NOT_ALLOWED);

    // The guesser: the budget is spent, then the answer is a 429 with a hint —
    // and the right password no longer helps until the window frees.
    let guesser = "198.51.100.4";
    for n in 0..8 {
        let refused = attempt(&state, &host, "/prices", "wrong wrong wrong", guesser).await;
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED, "guess {n}");
    }
    let stopped = attempt(&state, &host, "/prices", "wrong wrong wrong", guesser).await;
    assert_eq!(stopped.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(!header_str(&stopped, &header::RETRY_AFTER).is_empty());
    let stopped_body = body_string(stopped).await;
    assert!(stopped_body.contains("Too many attempts"), "{stopped_body}");
    let stopped = attempt(&state, &host, "/prices", PASSWORD, guesser).await;
    assert_eq!(
        stopped.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "the limiter is not a password check"
    );

    // Another visitor is unaffected by the guesser's spent budget.
    let opened = attempt(&state, &host, "/prices", PASSWORD, "198.51.100.5").await;
    assert_eq!(opened.status(), StatusCode::SEE_OTHER);

    acc.delete_site(&site).await.unwrap();
}
