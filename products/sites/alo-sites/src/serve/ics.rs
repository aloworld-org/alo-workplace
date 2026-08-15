//! RFC 5545 primitives shared by every page that hands a visitor a calendar
//! document (the booking confirmation, the ticket): the UTC `DATE-TIME`
//! form, §3.3.11 TEXT escaping and §3.1 line folding. Each page composes its
//! own document from these — what a VEVENT says is the page's business; that
//! it is a valid iCalendar line is this module's.

use time::OffsetDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

/// How an instant is written into an iCalendar document: UTC, second
/// precision, the RFC 5545 `DATE-TIME` form with the `Z` designator.
const ICS_TIME: &[BorrowedFormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");

/// An instant as an RFC 5545 UTC `DATE-TIME`.
pub(super) fn ics_time(instant: OffsetDateTime) -> String {
    instant
        .to_offset(time::UtcOffset::UTC)
        .format(ICS_TIME)
        .unwrap_or_default()
}

/// RFC 5545 §3.3.11 TEXT escaping: backslash first, then the reserved
/// separators; a newline becomes the literal `\n`.
pub(super) fn ics_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\r', "")
        .replace('\n', "\\n")
}

/// RFC 5545 §3.1 line folding: content lines longer than 75 octets are split
/// with CRLF + one space, always on a character boundary so no UTF-8
/// sequence is ever cut.
pub(super) fn ics_fold(line: &str) -> String {
    const LIMIT: usize = 74;
    if line.len() <= LIMIT {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len() + line.len() / LIMIT * 3);
    let mut budget = LIMIT;
    let mut used = 0;
    for c in line.chars() {
        let width = c.len_utf8();
        if used + width > budget {
            out.push_str("\r\n ");
            // A continuation line starts with the fold space, which counts
            // against its 75 octets.
            budget = LIMIT - 1;
            used = 0;
        }
        out.push(c);
        used += width;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_values_are_escaped_per_rfc_5545() {
        assert_eq!(
            ics_escape("a;b,c\\d\ne\r\nf"),
            "a\\;b\\,c\\\\d\\ne\\nf".to_owned()
        );
    }

    #[test]
    fn folding_never_splits_a_multibyte_character() {
        let line = format!("SUMMARY:{}", "é".repeat(200));
        let folded = ics_fold(&line);
        for part in folded.split("\r\n") {
            assert!(part.len() <= 75);
            assert!(std::str::from_utf8(part.as_bytes()).is_ok());
        }
        // Unfolding gives the original back.
        assert_eq!(folded.replace("\r\n ", ""), line);
    }

    #[test]
    fn an_instant_is_the_utc_date_time_form() {
        assert_eq!(
            ics_time(time::macros::datetime!(2026-09-16 07:00 +2)),
            "20260916T050000Z"
        );
    }
}
