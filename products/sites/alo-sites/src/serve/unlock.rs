//! Visitor sessions for password-protected pages (ADR 0036, S2.06a).
//!
//! A visitor who types the right password gets a cookie, not a row: the
//! session is a signed statement — *this host, this page, this password
//! version, until this moment* — that the service can check without a
//! database read and without knowing anything about the person holding it.
//! That is the whole privacy story of the gate: no session table, no visitor
//! identifier, nothing about the visit is retained after the response.
//!
//! Three properties the signature carries, each with a test behind it:
//!
//! - **Host-bound.** The public host is inside the MAC, so a cookie minted on
//!   one site cannot open a page on another even if a browser were persuaded
//!   to send it.
//! - **Page-bound.** The page id is inside the MAC, so unlocking one protected
//!   page never opens its neighbour.
//! - **Password-bound.** The store's opaque protection version is inside the
//!   MAC. Changing (or lifting) the password rotates the version, and every
//!   session opened with the old password stops working on the next request —
//!   which is what makes "change the password" a real revocation.
//!
//! The signing key is derived from the deployment's existing sites secret with
//! a fixed label, so unlock signatures and analytics visitor hashes can never
//! be confused for one another and no new secret has to be deployed.

use std::sync::Arc;
use std::time::Instant;

use axum::Form;
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use time::OffsetDateTime;

use alo_store::PublishedSite;

use crate::render::UiStrings;

use super::AppState;
use super::rendered::RenderedSite;

type HmacSha256 = Hmac<Sha256>;

/// What the unlock screen says above the password field, beyond its standing
/// explanation. The wording itself lives in the renderer's per-locale strings,
/// so the screen speaks the language of the page it stands in front of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnlockNotice {
    /// First ask — nothing has gone wrong yet.
    None,
    /// The password just tried does not open this page.
    WrongPassword,
    /// Too many attempts came from this visitor.
    TooManyAttempts,
}

impl UnlockNotice {
    pub(super) fn text(self, strings: &'static UiStrings) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::WrongPassword => Some(strings.protected_wrong),
            Self::TooManyAttempts => Some(strings.protected_rate_limited),
        }
    }
}

/// Cookie name prefix. The full name carries the page id, so a visitor may
/// hold sessions for several protected pages of one site at once.
const COOKIE_PREFIX: &str = "alo_site_unlock_";

/// How long a session lasts before the visitor is asked again. Long enough to
/// read a protected price list without re-typing, short enough that a shared
/// or forgotten browser does not stay open on it for days.
const SESSION_SECONDS: i64 = 12 * 60 * 60;

/// Domain-separation label for the derived signing key.
const KEY_LABEL: &[u8] = b"alo-sites/page-unlock/v1";

/// The scheme token on the `401`'s `WWW-Authenticate`. RFC 9110 requires the
/// header on every `401`; no browser prompts for an unknown scheme, so the
/// visitor sees our own screen instead of a native credential dialog.
const CHALLENGE_SCHEME: &str = "Form";

/// The fixed field name of the unlock form.
#[derive(Deserialize)]
struct UnlockBody {
    #[serde(default)]
    password: String,
}

/// The `401` that asks for the password: the site's own unlock screen, never
/// cached anywhere, and varying on `Cookie` so no shared cache can hand it to
/// a visitor who already opened the page (or the reverse).
///
/// `stale_page` clears a cookie that arrived for a password that has since
/// changed — otherwise the dead value would travel with every further request.
pub(super) fn challenge(
    site: &RenderedSite,
    path: &str,
    notice: UnlockNotice,
    stale_page: Option<&str>,
) -> Response {
    let mut response = (StatusCode::UNAUTHORIZED, site.challenge(path, notice)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(CHALLENGE_SCHEME),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::VARY, HeaderValue::from_static("Cookie"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if let Some(page) = stale_page
        && let Ok(value) = HeaderValue::from_str(&UnlockSessions::clearing_cookie(page))
    {
        headers.insert(header::SET_COOKIE, value);
    }
    response
}

/// The page itself, once a session opened it. Deliberately not the ordinary
/// cacheable answer: no `ETag` to revalidate against, `private, no-store` so
/// no proxy — and no browser cache a later visitor could reach — keeps the
/// bytes, and `Vary: Cookie` so nothing between us and the visitor may reuse
/// one person's unlocked copy for somebody else.
pub(super) fn unlocked(body: String) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Cookie"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// One password attempt: the `POST` a protected page's own unlock form makes.
///
/// Rate-limited per client key before the password is even read, so a guesser
/// pays the limiter rather than the database. A correct password answers `303`
/// back to the page itself with the session cookie attached — so the visitor
/// lands on the page by an ordinary navigation, and a reload does not re-post
/// the password. Every refusal is the same screen with the same status; only
/// the message differs, and never in a way that says whether the page exists.
pub(super) async fn attempt(
    state: &Arc<AppState>,
    resolved: &PublishedSite,
    site: &RenderedSite,
    public_host: &str,
    path: &str,
    page_id: &str,
    request: Request,
) -> Response {
    let key = super::forms::client_key(&request);
    if let Err(wait) = state.unlock_rate.allow(&key, Instant::now()) {
        let mut response = challenge(site, path, UnlockNotice::TooManyAttempts, None);
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response.headers_mut().remove(header::WWW_AUTHENTICATE);
        if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return response;
    }
    let Ok(Form(body)) = Form::<UnlockBody>::from_request(request, &()).await else {
        // An unreadable body is not a password: ask again rather than
        // explaining our parser to the internet.
        return challenge(site, path, UnlockNotice::WrongPassword, None);
    };
    match state
        .store
        .verify_page_password(resolved, page_id, &body.password)
        .await
    {
        Ok(Some(version)) => {
            let now = OffsetDateTime::now_utc().unix_timestamp();
            let cookie = state.unlock.cookie(public_host, page_id, &version, now);
            let mut response = (
                StatusCode::SEE_OTHER,
                [
                    (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
                    (header::VARY, HeaderValue::from_static("Cookie")),
                ],
            )
                .into_response();
            let headers = response.headers_mut();
            if let Ok(value) = HeaderValue::from_str(path) {
                headers.insert(header::LOCATION, value);
            }
            if let Ok(value) = HeaderValue::from_str(&cookie) {
                headers.insert(header::SET_COOKIE, value);
            }
            response
        }
        Ok(None) => challenge(site, path, UnlockNotice::WrongPassword, None),
        Err(error) => {
            // The password itself is never in scope for a log line; only that
            // the check could not be made.
            tracing::error!(%error, "page password check failed");
            super::unavailable()
        }
    }
}

/// Signs and checks unlock sessions. Holds only a derived key — never the
/// deployment secret itself, and never anything about a visitor.
pub(super) struct UnlockSessions {
    key: [u8; 32],
}

impl UnlockSessions {
    /// Derives the signing key from the deployment's sites secret.
    pub(super) fn new(secret: impl AsRef<[u8]>) -> Self {
        let key = match HmacSha256::new_from_slice(secret.as_ref()) {
            Ok(mut mac) => {
                mac.update(KEY_LABEL);
                mac.finalize().into_bytes().into()
            }
            // HMAC accepts every key length; stay total at this boundary.
            Err(_) => [0; 32],
        };
        Self { key }
    }

    /// The `Set-Cookie` value that opens `page` on `host` until
    /// [`SESSION_SECONDS`] from `now`.
    ///
    /// `HttpOnly` (no script needs it), `Secure` (public sites are HTTPS-only
    /// behind the proxy), `SameSite=Lax` so following a link into the page
    /// from elsewhere still works, and `Path=/` because a page's path can be
    /// renamed by a later publish while its identity stays.
    pub(super) fn cookie(&self, host: &str, page: &str, version: &str, now: i64) -> String {
        let expires = now + SESSION_SECONDS;
        let signature = self.sign(host, page, version, expires);
        format!(
            "{COOKIE_PREFIX}{page}={expires}.{signature}; Max-Age={SESSION_SECONDS}; \
             Path=/; HttpOnly; Secure; SameSite=Lax"
        )
    }

    /// The `Set-Cookie` value that ends any session for `page` on this host —
    /// used when a cookie arrives for a page whose password has since been
    /// changed or lifted, so the stale value does not travel with every
    /// further request.
    pub(super) fn clearing_cookie(page: &str) -> String {
        format!("{COOKIE_PREFIX}{page}=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax")
    }

    /// Whether `cookie_header` carries a live, untampered session for exactly
    /// this host, page, and password version.
    pub(super) fn opens(
        &self,
        cookie_header: Option<&str>,
        host: &str,
        page: &str,
        version: &str,
        now: i64,
    ) -> bool {
        let Some(value) = cookie_value(cookie_header, page) else {
            return false;
        };
        let Some((expires, signature)) = value.split_once('.') else {
            return false;
        };
        let Ok(expires) = expires.parse::<i64>() else {
            return false;
        };
        if expires <= now {
            return false;
        }
        let expected = self.sign(host, page, version, expires);
        // Both sides are hex of a fixed-width MAC; compare in constant time so
        // a signature cannot be discovered one character at a time.
        constant_time_eq(expected.as_bytes(), signature.as_bytes())
    }

    fn sign(&self, host: &str, page: &str, version: &str, expires: i64) -> String {
        let Ok(mut mac) = HmacSha256::new_from_slice(&self.key) else {
            return String::new();
        };
        // Every field is length-delimited by a byte that cannot occur in any
        // of them, so no two different tuples can produce the same input.
        for field in [host, page, version, &expires.to_string()] {
            mac.update(field.as_bytes());
            mac.update(&[0]);
        }
        let bytes: [u8; 32] = mac.finalize().into_bytes().into();
        bytes.iter().fold(String::with_capacity(64), |mut out, b| {
            use std::fmt::Write;
            let _ = write!(out, "{b:02x}");
            out
        })
    }
}

/// Whether the visitor is carrying any session cookie for `page` — true even
/// when it no longer opens the page, which is exactly when it is worth
/// clearing.
pub(super) fn carries_session(cookie_header: Option<&str>, page: &str) -> bool {
    cookie_value(cookie_header, page).is_some()
}

/// The value of this page's unlock cookie in a `Cookie` header, if present.
fn cookie_value<'a>(header: Option<&'a str>, page: &str) -> Option<&'a str> {
    let name = format!("{COOKIE_PREFIX}{page}");
    header?.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key.trim() == name).then(|| value.trim())
    })
}

/// Length-then-content equality with no early exit on the content.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b)
        .fold(0_u8, |acc, (x, y)| acc | (x ^ y))
        .eq(&0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const HOST: &str = "acme.sites.test";
    const PAGE: &str = "pg_1234567890";
    const VERSION: &str = "v1version";

    fn sessions() -> UnlockSessions {
        UnlockSessions::new(b"a deployment secret of at least 32 bytes")
    }

    /// The `Cookie` header a browser would send back after `Set-Cookie`.
    fn echo(set_cookie: &str) -> String {
        set_cookie
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned()
    }

    #[test]
    fn a_minted_session_opens_exactly_its_own_page_on_its_own_host() {
        let sessions = sessions();
        let cookie = echo(&sessions.cookie(HOST, PAGE, VERSION, 1_000));
        assert!(sessions.opens(Some(&cookie), HOST, PAGE, VERSION, 1_001));
        assert!(
            !sessions.opens(Some(&cookie), "other.sites.test", PAGE, VERSION, 1_001),
            "a session is bound to the host it was opened on"
        );
        assert!(
            !sessions.opens(Some(&cookie), HOST, "pg_other", VERSION, 1_001),
            "unlocking one page does not open another"
        );
        assert!(
            !sessions.opens(Some(&cookie), HOST, PAGE, "rotated", 1_001),
            "changing the password ends the session"
        );
    }

    #[test]
    fn expiry_tampering_and_forgery_all_fail_closed() {
        let sessions = sessions();
        let cookie = echo(&sessions.cookie(HOST, PAGE, VERSION, 1_000));
        assert!(
            !sessions.opens(
                Some(&cookie),
                HOST,
                PAGE,
                VERSION,
                1_000 + SESSION_SECONDS + 1
            ),
            "a session stops working when it runs out"
        );
        let stretched = cookie.replace(
            &(1_000 + SESSION_SECONDS).to_string(),
            &(1_000 + SESSION_SECONDS * 10).to_string(),
        );
        assert!(
            !sessions.opens(Some(&stretched), HOST, PAGE, VERSION, 1_001),
            "the expiry is signed, not merely stated"
        );
        let name = format!("{COOKIE_PREFIX}{PAGE}");
        for forged in [
            format!("{name}=9999999999.deadbeef"),
            format!("{name}=notanumber.deadbeef"),
            format!("{name}=9999999999"),
            format!("{name}="),
            "unrelated=1".to_owned(),
        ] {
            assert!(
                !sessions.opens(Some(&forged), HOST, PAGE, VERSION, 1_001),
                "forged cookie opened the page: {forged}"
            );
        }
        assert!(!sessions.opens(None, HOST, PAGE, VERSION, 1_001));
        let other = UnlockSessions::new(b"a different deployment secret entirely!!");
        assert!(
            !other.opens(Some(&cookie), HOST, PAGE, VERSION, 1_001),
            "another deployment's key cannot mint sessions here"
        );
    }

    #[test]
    fn the_cookie_is_found_among_others_and_scoped_safely() {
        let sessions = sessions();
        let set = sessions.cookie(HOST, PAGE, VERSION, 1_000);
        assert!(set.contains("HttpOnly"), "{set}");
        assert!(set.contains("Secure"), "{set}");
        assert!(set.contains("SameSite=Lax"), "{set}");
        let header = format!("consent=no; {}; theme=dark", echo(&set));
        assert!(sessions.opens(Some(&header), HOST, PAGE, VERSION, 1_001));
        let clearing = UnlockSessions::clearing_cookie(PAGE);
        assert!(clearing.contains("Max-Age=0"), "{clearing}");
        assert!(
            !sessions.opens(Some(&echo(&clearing)), HOST, PAGE, VERSION, 1_001),
            "the clearing cookie carries no session"
        );
    }
}
