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

use alo_store::{DeviceClass, PublicSiteVisit, PublishedSite};

use super::AppState;

/// Bound for the one query-string value that survives this boundary.
const CAMPAIGN_MAX_LEN: usize = 64;

/// Headers an edge proxy may use to report the country it resolved. alo never
/// derives a country from an address itself, and the address the proxy saw is
/// not read here.
const COUNTRY_HEADERS: [&str; 3] = ["cf-ipcountry", "x-country", "x-geo-country"];

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
/// which raw request metadata is discarded: the address, the user agent, the
/// full referrer and the rest of the query string do not leave this function.
pub(super) struct CapturedVisit {
    day: Date,
    visitor_hash: [u8; 32],
    referrer_domain: String,
    campaign: String,
    country: String,
    device: DeviceClass,
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
        campaign: campaign(request.uri()),
        country: country(request),
        device: device_class(request),
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
            &PublicSiteVisit {
                day: visit.day,
                path,
                referrer_domain: &visit.referrer_domain,
                campaign: &visit.campaign,
                country: &visit.country,
                device: visit.device,
                visitor_hash: &visit.visitor_hash,
            },
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

/// Reduces the query string to the one label a campaign report needs. Every
/// other parameter — including anything a link might carry about a person —
/// is dropped here and never reaches storage.
fn campaign(uri: &Uri) -> String {
    let raw = uri
        .query()
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case("utm_campaign"))
        .map(|(_, value)| decode_form_value(value))
        .unwrap_or_default();
    safe_label(&raw)
}

/// Percent- and plus-decoding for one query value. Invalid escapes are kept
/// literally; [`safe_label`] removes whatever they turn out to be. Shared
/// with the beacon's collect endpoint ([`super::beacon`]), whose payload is
/// encoded exactly like a query value and must be read exactly as strictly.
pub(super) fn decode_form_value(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            // Byte-wise on purpose: slicing the string here could land inside
            // a multi-byte character and panic on a hostile query.
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
                {
                    decoded.push(high * 16 + low);
                    index += 3;
                    continue;
                }
                decoded.push(b'%');
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// One hexadecimal digit, or `None` for anything else.
fn hex_digit(byte: u8) -> Option<u8> {
    let value = char::from(byte).to_digit(16)?;
    u8::try_from(value).ok()
}

/// Lowercases a campaign name into the bounded ASCII label storage accepts.
/// Anything else — accents, punctuation, an entire injected document — is
/// folded to a hyphen, so a hostile link cannot invent a dimension shape.
fn safe_label(raw: &str) -> String {
    let mut label = String::new();
    let mut previous_filler = false;
    for character in raw.trim().to_lowercase().chars() {
        if label.len() >= CAMPAIGN_MAX_LEN {
            break;
        }
        let allowed = matches!(character, 'a'..='z' | '0'..='9' | '-' | '_' | '.' | ' ');
        if allowed {
            label.push(character);
            previous_filler = character == '-';
        } else if !previous_filler {
            label.push('-');
            previous_filler = true;
        }
    }
    label
        .trim_matches(|character| character == '-' || character == ' ')
        .to_owned()
}

/// The country an edge proxy resolved, as a two-letter code. Unknown markers
/// and anything that is not two ASCII letters become "not reported".
fn country(request: &Request) -> String {
    COUNTRY_HEADERS
        .iter()
        .find_map(|name| request.headers().get(*name))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .map(str::to_ascii_uppercase)
        .filter(|code| {
            code.len() == 2
                && code.bytes().all(|byte| byte.is_ascii_uppercase())
                // Cloudflare's markers for "could not resolve" and Tor exits
                // are not countries.
                && !matches!(code.as_str(), "XX" | "T1")
        })
        .unwrap_or_default()
}

/// Classifies the user agent into one of five words and then forgets it. The
/// ordering matters: automated traffic often claims to be a phone.
fn device_class(request: &Request) -> DeviceClass {
    let Some(agent) = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_ascii_lowercase)
        .filter(|agent| !agent.trim().is_empty())
    else {
        return DeviceClass::Unknown;
    };
    let contains = |needles: &[&str]| needles.iter().any(|needle| agent.contains(needle));
    if contains(&[
        "bot",
        "crawl",
        "spider",
        "slurp",
        "headlesschrome",
        "curl/",
        "wget/",
        "python-requests",
        "facebookexternalhit",
        "monitoring",
    ]) {
        return DeviceClass::Bot;
    }
    if contains(&["ipad", "tablet"]) || (agent.contains("android") && !agent.contains("mobile")) {
        return DeviceClass::Tablet;
    }
    if contains(&["mobi", "iphone", "ipod", "windows phone"]) {
        return DeviceClass::Phone;
    }
    DeviceClass::Desktop
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

    fn get(uri: &str, headers: &[(&str, &str)]) -> Request {
        let mut builder = Request::builder().uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn only_the_campaign_survives_the_query_string() {
        let uri: Uri =
            "/pricing?email=someone%40example.test&UTM_Campaign=Spring+Sale%202026&utm_source=news"
                .parse()
                .unwrap();
        assert_eq!(campaign(&uri), "spring sale 2026");
        assert_eq!(campaign(&"/".parse::<Uri>().unwrap()), "");
        assert_eq!(campaign(&"/?utm_campaign=".parse::<Uri>().unwrap()), "");
    }

    #[test]
    fn a_hostile_campaign_cannot_invent_a_dimension() {
        let long = format!("/?utm_campaign={}", "a".repeat(200));
        assert_eq!(
            campaign(&long.parse::<Uri>().unwrap()).len(),
            CAMPAIGN_MAX_LEN
        );
        // Script, markup, quotes and accents all fold to the same filler, and
        // a truncated escape is not a decoding accident either.
        assert_eq!(
            campaign(&"/?utm_campaign=%3Cscript%3E".parse::<Uri>().unwrap()),
            "script"
        );
        assert_eq!(
            campaign(&"/?utm_campaign=%C3%A9t%C3%A9".parse::<Uri>().unwrap()),
            "t"
        );
        assert_eq!(
            campaign(&"/?utm_campaign=a%2".parse::<Uri>().unwrap()),
            "a-2"
        );
    }

    #[test]
    fn country_comes_from_the_edge_or_not_at_all() {
        assert_eq!(country(&get("/", &[("cf-ipcountry", "nl")])), "NL");
        assert_eq!(country(&get("/", &[("x-geo-country", " BE ")])), "BE");
        assert_eq!(country(&get("/", &[("cf-ipcountry", "XX")])), "");
        assert_eq!(country(&get("/", &[("cf-ipcountry", "T1")])), "");
        assert_eq!(country(&get("/", &[("cf-ipcountry", "Netherlands")])), "");
        assert_eq!(
            country(&get("/", &[(header::USER_AGENT.as_str(), "Mozilla/5.0")])),
            "",
            "no address is ever turned into a country here"
        );
    }

    #[test]
    fn device_class_is_five_words_and_bots_are_named() {
        let class = |agent: &str| device_class(&get("/", &[(header::USER_AGENT.as_str(), agent)]));
        assert_eq!(
            class("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) AppleWebKit/605.1.15 Mobile/15E148"),
            DeviceClass::Phone
        );
        assert_eq!(
            class("Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Mobile Safari/537.36"),
            DeviceClass::Phone
        );
        assert_eq!(
            class("Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15"),
            DeviceClass::Tablet
        );
        assert_eq!(
            class("Mozilla/5.0 (Linux; Android 14; SM-X200) AppleWebKit/537.36 Safari/537.36"),
            DeviceClass::Tablet
        );
        assert_eq!(
            class("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari"),
            DeviceClass::Desktop
        );
        // A crawler that claims to be a phone is still a crawler.
        assert_eq!(
            class(
                "Mozilla/5.0 (Linux; Android 6.0.1; Nexus 5X) Mobile Safari/537.36 (compatible; Googlebot/2.1)"
            ),
            DeviceClass::Bot
        );
        assert_eq!(class("curl/8.4.0"), DeviceClass::Bot);
        assert_eq!(class("   "), DeviceClass::Unknown);
        assert_eq!(device_class(&get("/", &[])), DeviceClass::Unknown);
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
