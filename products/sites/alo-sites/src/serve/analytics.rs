//! Privacy boundary for public traffic analytics. Request metadata exists in
//! this module only long enough to reduce it to safe dimensions and a
//! day-scoped HMAC. Storage never sees an IP address, user agent, query
//! string, full referrer, or unsalted identifier.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::http::{Method, StatusCode, Uri, header};
use axum::response::Response;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use time::{Date, OffsetDateTime};

use alo_store::PublishedSite;

use super::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Creates daily-separated visitor tokens from transient client keys. The
/// secret itself never enters analytics storage.
pub(super) struct VisitorHasher {
    secret: Vec<u8>,
}

impl VisitorHasher {
    pub(super) fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    fn hash(&self, day: Date, client: &str) -> [u8; 32] {
        // HMAC accepts keys of every length; retain a total function at this
        // boundary even if a future implementation changes that invariant.
        let Ok(mut mac) = HmacSha256::new_from_slice(&self.secret) else {
            return [0; 32];
        };
        mac.update(day.to_string().as_bytes());
        mac.update(&[0]);
        mac.update(client.as_bytes());
        mac.finalize().into_bytes().into()
    }
}

/// Safe, owned request derivatives. Constructing this is the exact point at
/// which raw request metadata is discarded.
pub(super) struct CapturedVisit {
    day: Date,
    visitor_hash: [u8; 32],
    referrer_domain: String,
}

/// Captures only the safe derivatives of a GET before any async work begins.
pub(super) fn capture(state: &Arc<AppState>, request: &Request) -> Option<CapturedVisit> {
    if request.method() != Method::GET {
        return None;
    }
    let day = OffsetDateTime::now_utc().date();
    Some(CapturedVisit {
        day,
        visitor_hash: state.analytics.hash(day, &client_key(request)),
        referrer_domain: referrer_domain(request),
    })
}

/// Best-effort collection for a successful HTML GET. The response is always
/// returned unchanged: a metrics outage must never become a site outage.
pub(super) async fn record_html_view(
    state: &Arc<AppState>,
    site: &PublishedSite,
    path: &str,
    visit: Option<CapturedVisit>,
    response: Response,
) -> Response {
    let Some(visit) = visit else {
        return response;
    };
    if !matches!(response.status(), StatusCode::OK | StatusCode::NOT_MODIFIED)
        || !response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"))
    {
        return response;
    }

    if let Err(error) = state
        .store
        .record_public_site_view(
            site,
            visit.day,
            path,
            &visit.referrer_domain,
            &visit.visitor_hash,
        )
        .await
    {
        tracing::warn!(site = %site.site, %error, "site analytics write failed");
    }
    response
}

/// The proxy-appended address, or direct peer address. The returned string
/// is consumed immediately by the HMAC and never retained or logged.
fn client_key(request: &Request) -> String {
    if let Some(client) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next_back())
        .map(str::trim)
        .filter(|client| !client.is_empty())
    {
        return client.to_owned();
    }
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| "direct".to_owned(), |info| info.0.ip().to_string())
}

/// Reduces `Referer` to a lowercase DNS host. Paths, query strings,
/// credentials, fragments, invalid values, and unknown traffic are dropped.
fn referrer_domain(request: &Request) -> String {
    request
        .headers()
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<Uri>().ok())
        .and_then(|uri| uri.host().map(str::to_ascii_lowercase))
        .filter(|host| host.len() <= 253)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use axum::body::Body;

    use super::*;

    #[test]
    fn visitor_tokens_are_stable_within_a_day_and_separated_between_days() {
        let hasher = VisitorHasher::new(b"fixture secret with enough entropy");
        let day = Date::from_calendar_date(2026, time::Month::August, 9).unwrap();
        let same = hasher.hash(day, "198.51.100.42");
        assert_eq!(same, hasher.hash(day, "198.51.100.42"));
        assert_ne!(same, hasher.hash(day.next_day().unwrap(), "198.51.100.42"));
        assert_ne!(same, hasher.hash(day, "198.51.100.43"));
    }

    #[test]
    fn referrer_is_reduced_to_its_domain() {
        let request = Request::builder()
            .uri("/")
            .header(
                header::REFERER,
                "https://User:secret@NEWS.Example/private/path?token=raw#fragment",
            )
            .body(Body::empty())
            .unwrap();
        assert_eq!(referrer_domain(&request), "news.example");
    }
}
