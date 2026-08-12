//! The conversion half of the beacon payload ([`super::beacon`]): a
//! conversion point of the page was seen, or a visitor began filling it in.
//!
//! Both are facts only a browser can see — the server serves a whole page and
//! cannot know whether the form on it was reached, and it certainly cannot see
//! a first keystroke. The third stage, the submit, is *not* reported from here:
//! it is counted where the submission is actually written
//! (`alo_store::SitePublicStore::record_public_form_conversion`), because a
//! script is easier to lie to than a socket.
//!
//! What the report may carry is deliberately two tokens and nothing else:
//!
//! - `c` — the conversion point's id, which the page's own markup already
//!   published (`<form action="/f/{id}">`). It is the site's id, not the
//!   visitor's: attribution here needs no tracking identity, and the write
//!   door only counts it when it resolves to a form of the site the Host
//!   named.
//! - `s` — one of three fixed stage words.
//!
//! There is no key for a page, a session, a time, or a field value: a
//! conversion count says a form was reached, never who reached it or what
//! they typed.

use alo_store::{CONVERSION_SOURCE_ID_MAX_LEN, ConversionStage};

/// One parsed conversion report, already reduced to what storage accepts.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ConversionReport {
    /// The site-owned id of the conversion point.
    pub(super) source: String,
    pub(super) stage: ConversionStage,
}

/// Reads a conversion report out of the beacon body's key/value pairs, or
/// `None` if the pairs do not form one.
///
/// A submit is refused here even though it is a real stage word: the browser
/// does not get to claim one, because the submission endpoint counts it from
/// the write it actually performed. Accepting it would let anyone inflate the
/// one number an owner is most likely to act on.
pub(super) fn parse(pairs: &[(&str, &str)]) -> Option<ConversionReport> {
    let value = |wanted: &str| {
        pairs
            .iter()
            .find(|(key, _)| *key == wanted)
            .map(|(_, value)| *value)
    };
    let source = safe_source(&super::analytics::decode_form_value(value("c")?))?;
    let stage = match ConversionStage::from_word(&super::analytics::decode_form_value(value("s")?))?
    {
        ConversionStage::Submit => return None,
        stage => stage,
    };
    Some(ConversionReport { source, stage })
}

/// The shape one of our ids has, or `None`. Not repaired into something
/// storable: a value that is not an id is not an id, and the store's door
/// refuses the same shapes for the same reason.
fn safe_source(raw: &str) -> Option<String> {
    let shaped = !raw.is_empty()
        && raw.len() <= CONVERSION_SOURCE_ID_MAX_LEN
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    shaped.then(|| raw.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(body: &str) -> Vec<(&str, &str)> {
        body.split('&')
            .filter_map(|pair| pair.split_once('='))
            .collect()
    }

    #[test]
    fn a_view_and_a_start_are_reports() {
        assert_eq!(
            parse(&pairs("c=Qk9tX3zvS1aQmN2pRt4uYw&s=view")),
            Some(ConversionReport {
                source: "Qk9tX3zvS1aQmN2pRt4uYw".to_owned(),
                stage: ConversionStage::View,
            })
        );
        assert_eq!(
            parse(&pairs("c=Qk9tX3zvS1aQmN2pRt4uYw&s=start")),
            Some(ConversionReport {
                source: "Qk9tX3zvS1aQmN2pRt4uYw".to_owned(),
                stage: ConversionStage::Start,
            })
        );
    }

    #[test]
    fn a_browser_may_not_claim_a_submit() {
        assert_eq!(parse(&pairs("c=Qk9tX3zvS1aQmN2pRt4uYw&s=submit")), None);
    }

    #[test]
    fn an_incomplete_or_hostile_report_is_not_a_report() {
        for hostile in [
            // Half a report.
            "c=Qk9tX3zvS1aQmN2pRt4uYw",
            "s=view",
            "c=&s=view",
            "c=Qk9tX3zvS1aQmN2pRt4uYw&s=",
            // Words that are not stages.
            "c=Qk9tX3zvS1aQmN2pRt4uYw&s=VIEW",
            "c=Qk9tX3zvS1aQmN2pRt4uYw&s=opened",
            // Values that are not ids.
            "c=%3Cscript%3E&s=view",
            "c=..%2F..%2Fetc&s=view",
            "c=one%20two&s=view",
            "c=id%27or%271%3D1&s=view",
            "c=https%3A%2F%2Felsewhere.example&s=view",
        ] {
            assert!(
                parse(&pairs(hostile)).is_none(),
                "{hostile} became a report"
            );
        }
        let long = format!("c={}&s=view", "a".repeat(CONVERSION_SOURCE_ID_MAX_LEN + 1));
        assert!(parse(&pairs(&long)).is_none());
    }
}
