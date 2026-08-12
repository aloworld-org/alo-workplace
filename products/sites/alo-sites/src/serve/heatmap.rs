//! The heatmap half of the beacon payload ([`super::beacon`]): where a page
//! was clicked, and how far down it was read.
//!
//! Unlike the read-time and outbound reports, a heatmap event has to name the
//! page it happened on — an overlay is drawn over one page. That is the whole
//! of the extra privacy surface, and it is the same path the page view is
//! already counted under, so this module's job is to make sure nothing *else*
//! comes with it:
//!
//! - The click position arrives as permille of the page's own width and
//!   height and becomes one grid cell here; no pixel coordinate is passed on.
//! - The scroll depth arrives the same way and becomes one of ten tenths.
//! - The viewport arrives as a CSS pixel width — a fingerprinting signal — and
//!   is reduced to one of three classes in this function. The number is never
//!   returned, logged, or stored.
//! - The path is canonicalized exactly as the page path is at the door
//!   (trailing slash trimmed) and refused unless it is an absolute page path.
//!   A query string or fragment is refused rather than trimmed: a browser
//!   sends `location.pathname`, so anything else did not come from one.
//!
//! There is deliberately no key for an identity, a session, or a time.

use alo_store::{HeatmapCell, HeatmapSignal, ScrollDepth, ViewportClass};

/// Bound for the page path, matching the store's own.
const PATH_MAX_LEN: usize = 2048;

/// The most digits any numeric key may carry. A permille is at most four and
/// a viewport width at most five; anything longer is not a measurement.
const NUMBER_MAX_DIGITS: usize = 5;

/// One parsed heatmap report, already reduced to what storage accepts.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct HeatmapReport {
    pub(super) path: String,
    pub(super) viewport: ViewportClass,
    pub(super) signal: HeatmapSignal,
}

/// Reads a heatmap report out of the beacon body's key/value pairs, or `None`
/// if the pairs do not form one. Every heatmap report needs a page (`p`) and a
/// viewport width (`w`); a click adds `x` and `y` permille, a scroll adds `d`.
pub(super) fn parse(pairs: &[(&str, &str)]) -> Option<HeatmapReport> {
    let value = |wanted: &str| {
        pairs
            .iter()
            .find(|(key, _)| *key == wanted)
            .map(|(_, value)| *value)
    };
    let path = safe_path(&super::analytics::decode_form_value(value("p")?))?;
    let viewport = ViewportClass::from_width(number(value("w")?)?);
    let signal = match (value("x"), value("y"), value("d")) {
        (Some(x), Some(y), _) => {
            HeatmapSignal::Click(HeatmapCell::from_permille(permille(x)?, permille(y)?))
        }
        (_, _, Some(depth)) => HeatmapSignal::Scroll(ScrollDepth::from_permille(permille(depth)?)),
        _ => return None,
    };
    Some(HeatmapReport {
        path,
        viewport,
        signal,
    })
}

/// A bounded decimal number, or `None`. Signs, decimals, and absurd lengths
/// are refused rather than repaired — a browser sends integers.
fn number(raw: &str) -> Option<u32> {
    (!raw.is_empty() && raw.len() <= NUMBER_MAX_DIGITS)
        .then(|| raw.parse::<u32>().ok())
        .flatten()
}

/// A permille, clamped to the scale rather than refused past it: browsers
/// round differently at the far edge, and a click one permille past the end
/// of the page is still a click at the end of the page.
fn permille(raw: &str) -> Option<u16> {
    number(raw).map(|value| u16::try_from(value.min(1000)).unwrap_or(1000))
}

/// Canonicalizes a browser-reported `location.pathname` into the exact shape
/// a page view is counted under, or `None` if it is not one.
fn safe_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim_end_matches('/');
    let path = if trimmed.is_empty() { "/" } else { trimmed };
    let shaped = path.starts_with('/')
        && path.len() <= PATH_MAX_LEN
        && !path.contains(['?', '#', '\\'])
        && !path.chars().any(|character| {
            character.is_control() || character.is_whitespace() || character == '"'
        });
    shaped.then(|| path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(body: &str) -> Vec<(&str, &str)> {
        body.split('&')
            .filter_map(|pair| pair.split_once('='))
            .collect()
    }

    fn report(path: &str, viewport: ViewportClass, signal: HeatmapSignal) -> Option<HeatmapReport> {
        Some(HeatmapReport {
            path: path.to_owned(),
            viewport,
            signal,
        })
    }

    #[test]
    fn a_click_becomes_a_cell_on_a_named_page() {
        assert_eq!(
            parse(&pairs("x=500&y=250&p=%2Fprices&w=1440")),
            report(
                "/prices",
                ViewportClass::Desktop,
                HeatmapSignal::Click(HeatmapCell::from_permille(500, 250))
            )
        );
        // The narrow screen is a class, not the width it reported.
        assert_eq!(
            parse(&pairs("x=0&y=0&p=%2F&w=390")),
            report(
                "/",
                ViewportClass::Phone,
                HeatmapSignal::Click(HeatmapCell::from_permille(0, 0))
            )
        );
    }

    #[test]
    fn a_scroll_becomes_a_tenth_of_the_page() {
        assert_eq!(
            parse(&pairs("d=880&p=%2Fabout&w=768")),
            report(
                "/about",
                ViewportClass::Tablet,
                HeatmapSignal::Scroll(ScrollDepth::from_permille(880))
            )
        );
        // Past the end of the page is still the end of the page.
        assert_eq!(
            parse(&pairs("d=32000&p=%2F&w=768")),
            report(
                "/",
                ViewportClass::Tablet,
                HeatmapSignal::Scroll(ScrollDepth::from_permille(1000))
            )
        );
    }

    #[test]
    fn an_incomplete_or_hostile_report_is_not_a_report() {
        for hostile in [
            // No page, no viewport, no measurement.
            "x=500&y=250&w=1440",
            "x=500&y=250&p=%2Fprices",
            "p=%2Fprices&w=1440",
            "x=500&p=%2Fprices&w=1440",
            // Numbers that are not measurements.
            "x=-5&y=250&p=%2Fprices&w=1440",
            "x=5.5&y=250&p=%2Fprices&w=1440",
            "x=999999999&y=250&p=%2Fprices&w=1440",
            "d=abc&p=%2Fprices&w=1440",
            "d=880&p=%2Fprices&w=",
            // Paths that are not page paths.
            "d=880&p=prices&w=1440",
            "d=880&p=https%3A%2F%2Felsewhere.example%2Fx&w=1440",
            "d=880&p=%2Fprices%3Futm_campaign%3Dspring&w=1440",
            "d=880&p=%2Fprices%23table&w=1440",
            "d=880&p=%2Fpri%20ces&w=1440",
            "d=880&p=%2Fprices%0Aset-cookie%3A%20x&w=1440",
        ] {
            assert!(
                parse(&pairs(hostile)).is_none(),
                "{hostile} became a report"
            );
        }
        let long = format!("d=880&w=1440&p=%2F{}", "a".repeat(PATH_MAX_LEN));
        assert!(parse(&pairs(&long)).is_none());
    }

    #[test]
    fn a_trailing_slash_is_the_same_page_as_without_one() {
        assert_eq!(
            parse(&pairs("d=500&p=%2Fabout%2F&w=1440")),
            parse(&pairs("d=500&p=%2Fabout&w=1440")),
        );
        assert_eq!(
            parse(&pairs("d=500&p=%2Fabout%2F&w=1440")),
            report(
                "/about",
                ViewportClass::Desktop,
                HeatmapSignal::Scroll(ScrollDepth::from_permille(500))
            )
        );
    }
}
