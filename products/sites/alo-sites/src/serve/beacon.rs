//! `POST /_alo/collect` — the collect endpoint of the published page's
//! analytics beacon ([`crate::render::script::BEACON_SCRIPT`]).
//!
//! Some facts exist only in a browser: how long a page stayed readable, which
//! outside domain a visitor followed a link to, where the page was clicked,
//! how far down it was read, and whether a conversion point on it was reached
//! or begun ([`super::conversion`]). Everything else in the traffic report is
//! derived from the request at the door ([`super::analytics`]) and stays there
//! — this endpoint exists precisely because those cannot be.
//!
//! That makes it the one public write with no page load behind it, so its
//! whole design is about what it *cannot* be used for:
//!
//! - **Tenant scope is the `Host`, never the payload.** The body has no site,
//!   tenant, page or session field to put one in; an unresolvable Host is the
//!   same terse `404` a page request gets.
//! - **No identity, in either direction.** The endpoint sets no cookie, reads
//!   none, and derives no visitor token — not even the day-scoped HMAC page
//!   views are counted with. Two beacons from one browser are unlinkable by
//!   construction, which is why these aggregates carry a hit count and no
//!   unique count.
//! - **Tiny and bounded.** The body may be [`BEACON_BODY_MAX_BYTES`]; the
//!   keys are `t` (seconds, bucketed server-side), `o` (a DNS host, folded to
//!   a bounded lowercase label), and the heatmap set — `p` (the page), `w`
//!   (a viewport width, reduced to one of three classes and discarded), `x`
//!   and `y` (a click, in permille of the page, reduced to one grid cell),
//!   `d` (a scroll depth, reduced to one tenth), and the conversion pair — `c`
//!   (a conversion point of the site, whose id the page's own markup already
//!   published) with `s` (one of two stage words). Anything else is a `400`.
//! - **Rate-limited per client**, on its own budget, so beacon traffic can
//!   never spend the budget that stands between a guesser and a protected
//!   page or a flood and a form.
//!
//! The wire contract is deliberately mute: `204` on success, and every
//! refusal is a bare status with no body. `navigator.sendBeacon` cannot read
//! a response anyway, and a chatty endpoint here would only help someone
//! probing which hosts exist.

use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::{FromRequest, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use time::OffsetDateTime;

use alo_store::{PublicSiteHeatmapReport, PublicSiteSignal, ReadTimeBucket};

use super::AppState;
use super::conversion::ConversionReport;
use super::heatmap::HeatmapReport;

/// The most a beacon body may carry. A real one is under fifty bytes (`t=137`,
/// `o=` and a domain, or a click on a short path); this leaves room for a long
/// percent-encoded punycode host or page path and nothing else.
pub(super) const BEACON_BODY_MAX_BYTES: usize = 512;

/// The most key/value pairs a body may hold. The largest report uses four
/// (`x`, `y`, `p`, `w`); the bound keeps a padded body from costing a scan.
const BEACON_MAX_PAIRS: usize = 8;

/// The most a domain label may be, matching the store's referrer bound.
const DOMAIN_MAX_LEN: usize = 253;

/// One thing a beacon may report. Parsed from the body before anything is
/// resolved, so a malformed payload never costs a database read.
#[derive(Debug, PartialEq, Eq)]
enum Report {
    /// Seconds the page stayed readable, still unbucketed.
    ReadSeconds(u64),
    /// The DNS host a visitor left for, already folded to storage shape.
    Outbound(String),
    /// A click cell or a scroll depth on one named page ([`super::heatmap`]).
    Heatmap(HeatmapReport),
    /// A conversion point of this site was seen or begun
    /// ([`super::conversion`]).
    Conversion(ConversionReport),
}

/// Handles one beacon POST: rate limit, parse, resolve the Host, write one
/// bounded aggregate.
pub(super) async fn collect(State(state): State<Arc<AppState>>, request: Request) -> Response {
    // The limiter runs before anything else, including the body read: a flood
    // must cost this service one map lookup, not a parse and a query.
    if let Err(wait) = state
        .beacon_rate
        .allow(&super::forms::client_key(&request), Instant::now())
    {
        let mut response = terse(StatusCode::TOO_MANY_REQUESTS);
        if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return response;
    }

    let Some(scope) = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| super::host::scope(value, &state.sites_domain))
    else {
        return terse(StatusCode::NOT_FOUND);
    };

    let body = match Bytes::from_request(request, &()).await {
        Ok(body) => body,
        // Over the route's size limit, or unreadable: one terse refusal each.
        Err(rejection) => return terse(rejection.status()),
    };
    let Some(report) = std::str::from_utf8(&body).ok().and_then(parse) else {
        return terse(StatusCode::BAD_REQUEST);
    };

    let resolved = match super::resolve_scope(&state, &scope).await {
        Ok(Some(site)) => site,
        // Unknown, unpublished, and not-a-site are one answer, exactly as on
        // the page path: this endpoint reveals no site's existence.
        Ok(None) => return terse(StatusCode::NOT_FOUND),
        Err(error) => {
            tracing::error!(host = scope.host(), %error, "beacon resolver read failed");
            return terse(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    let day = OffsetDateTime::now_utc().date();
    let written = match &report {
        Report::ReadSeconds(seconds) => {
            state
                .store
                .record_public_site_signal(
                    &resolved,
                    day,
                    PublicSiteSignal::ReadTime(ReadTimeBucket::from_seconds(*seconds)),
                )
                .await
        }
        Report::Outbound(domain) => {
            state
                .store
                .record_public_site_signal(&resolved, day, PublicSiteSignal::Outbound(domain))
                .await
        }
        Report::Heatmap(heatmap) => {
            state
                .store
                .record_public_site_heatmap(
                    &resolved,
                    &PublicSiteHeatmapReport {
                        day,
                        path: &heatmap.path,
                        viewport: heatmap.viewport,
                        signal: heatmap.signal,
                    },
                )
                .await
        }
        Report::Conversion(conversion) => {
            // The bool says whether the source resolved to a form of this
            // resolved site. A foreign or invented id simply counts nothing:
            // answering differently would turn the endpoint into an oracle for
            // which ids exist.
            state
                .store
                .record_public_site_conversion(&resolved, day, &conversion.source, conversion.stage)
                .await
                .map(drop)
        }
    };
    if let Err(error) = written {
        // A metrics outage is not a site outage, and the beacon's sender
        // cannot act on the difference — but the operator can.
        tracing::warn!(site = %resolved.site, %error, "site beacon write failed");
        return terse(StatusCode::SERVICE_UNAVAILABLE);
    }
    terse(StatusCode::NO_CONTENT)
}

/// A bare status with no body and no caching. Every answer this endpoint
/// gives has this shape, success included.
fn terse(status: StatusCode) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
    )
        .into_response()
}

/// Reads the one report a body carries. The payload is `key=value` pairs like
/// a form body — `sendBeacon` sends it as text, so it is parsed here rather
/// than by an extractor that would insist on a content type a beacon cannot
/// choose.
///
/// Exactly one report per request: the first recognized, well-formed key
/// wins, and a body with no such key is refused rather than silently ignored.
/// A heatmap report needs several keys together ([`super::heatmap`]), so its
/// marker key hands the whole pair list over rather than reading one value.
fn parse(body: &str) -> Option<Report> {
    let pairs = body
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .take(BEACON_MAX_PAIRS)
        .collect::<Vec<_>>();
    pairs.iter().find_map(|&(key, value)| match key {
        // Bounded before parsing: a thousand-digit number is not a
        // duration, and refusing it here keeps the parse total.
        "t" if !value.is_empty() && value.len() <= 7 => {
            value.parse::<u64>().ok().map(Report::ReadSeconds)
        }
        "o" => safe_domain(&super::analytics::decode_form_value(value)).map(Report::Outbound),
        "x" | "d" => super::heatmap::parse(&pairs).map(Report::Heatmap),
        "c" => super::conversion::parse(&pairs).map(Report::Conversion),
        _ => None,
    })
}

/// Folds a browser-reported hostname into the bounded lowercase DNS host
/// storage accepts, or `None` if it is not one.
///
/// Unlike a campaign label this is not repaired into something storable:
/// a value that is not a hostname is not a hostname, and inventing
/// `-` out of an injected document would create a bucket nobody asked for.
/// Non-ASCII is rejected outright — a browser's `link.hostname` is already
/// punycode for an international domain, so anything else is not from one.
fn safe_domain(raw: &str) -> Option<String> {
    let domain = raw.trim().to_ascii_lowercase();
    let shaped = !domain.is_empty()
        && domain.len() <= DOMAIN_MAX_LEN
        && domain.contains('.')
        && !domain.starts_with(['.', '-'])
        && !domain.ends_with(['.', '-'])
        && !domain.contains("..")
        && domain.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        });
    shaped.then_some(domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_time_is_a_bounded_number_of_seconds() {
        assert_eq!(parse("t=0"), Some(Report::ReadSeconds(0)));
        assert_eq!(parse("t=137"), Some(Report::ReadSeconds(137)));
        assert_eq!(parse("t="), None);
        assert_eq!(parse("t=-3"), None);
        assert_eq!(parse("t=12.5"), None);
        assert_eq!(parse("t=99999999999999"), None, "not a duration");
        assert_eq!(parse(""), None);
        assert_eq!(parse("path=/prices"), None, "no unknown key is accepted");
    }

    #[test]
    fn seconds_only_ever_reach_storage_as_one_of_six_buckets() {
        let bucket = |seconds| ReadTimeBucket::from_seconds(seconds).as_str();
        assert_eq!(bucket(0), "0-10s");
        assert_eq!(bucket(9), "0-10s");
        assert_eq!(bucket(10), "10-30s");
        assert_eq!(bucket(59), "30-60s");
        assert_eq!(bucket(60), "1-3m");
        assert_eq!(bucket(179), "1-3m");
        assert_eq!(bucket(600), "10m+");
        assert_eq!(
            bucket(u64::MAX),
            "10m+",
            "an absurd claim is still a bucket"
        );
    }

    #[test]
    fn an_outbound_domain_is_a_hostname_or_it_is_nothing() {
        assert_eq!(
            parse("o=News.Example"),
            Some(Report::Outbound("news.example".to_owned()))
        );
        assert_eq!(
            parse("o=shop.news.example"),
            Some(Report::Outbound("shop.news.example".to_owned()))
        );
        // A hostile payload is refused, never folded into a storable label:
        // markup, a path, a whole URL, an address, and a bare word are all
        // simply not hostnames.
        for hostile in [
            "o=%3Cscript%3E",
            "o=news.example/private/path",
            "o=https%3A%2F%2Fnews.example",
            "o=user%40news.example",
            "o=localhost",
            "o=.news.example",
            "o=news..example",
            "o=news.example-",
            "o=%C3%A9t%C3%A9.example",
            "o=",
        ] {
            assert_eq!(parse(hostile), None, "{hostile} became a bucket");
        }
        let long = format!("o={}.example", "a".repeat(DOMAIN_MAX_LEN));
        assert_eq!(parse(&long), None);
    }

    #[test]
    fn a_heatmap_report_needs_its_whole_set_of_keys() {
        // The marker key hands the body to the heatmap parser, which decides
        // whether the rest of it forms a report at all.
        assert!(matches!(
            parse("x=500&y=250&p=%2Fprices&w=1440"),
            Some(Report::Heatmap(_))
        ));
        assert!(matches!(
            parse("d=880&p=%2Fprices&w=390"),
            Some(Report::Heatmap(_))
        ));
        assert_eq!(parse("x=500&y=250"), None, "a click with no page");
        assert_eq!(parse("d=880&w=1440"), None, "a scroll with no page");
        assert_eq!(parse("p=%2Fprices&w=1440"), None, "a page with no event");
    }

    #[test]
    fn a_conversion_report_is_read_by_its_own_parser() {
        // The marker key hands the body to the conversion parser, which
        // decides what is a report — including refusing the submit stage,
        // which is counted at the write instead ([`super::super::conversion`]).
        assert!(matches!(
            parse("c=Qk9tX3zvS1aQmN2pRt4uYw&s=view"),
            Some(Report::Conversion(_))
        ));
        assert_eq!(parse("c=Qk9tX3zvS1aQmN2pRt4uYw&s=submit"), None);
        assert_eq!(parse("c=Qk9tX3zvS1aQmN2pRt4uYw"), None, "half a report");
        assert_eq!(parse("s=view"), None, "a stage with no conversion point");
    }

    #[test]
    fn one_body_reports_one_thing() {
        // The first well-formed key wins; a second cannot smuggle a bucket in
        // beside it.
        assert_eq!(
            parse("t=42&o=news.example"),
            Some(Report::ReadSeconds(42)),
            "a body reports exactly one thing"
        );
        assert_eq!(
            parse("t=x&o=news.example"),
            Some(Report::Outbound("news.example".to_owned()))
        );
    }
}
