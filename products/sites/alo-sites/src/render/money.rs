//! Writing a catalog price the way the page's language writes it.
//!
//! Prices arrive as integer minor units plus an ISO 4217 code — never a float,
//! never a pre-formatted string — so the only decisions left here are how many
//! decimals the currency has (the store's
//! [`currency_exponent`](alo_store::currency_exponent), the single source for
//! that) and how the visitor's language spells a number: which separators, and
//! whether the symbol leads or trails. Both spellings live in
//! [`UiStrings`](super::UiStrings) beside every other visitor-facing string, so
//! a new locale is a new const rather than a code hunt.

use alo_store::currency_exponent;

use super::UiStrings;

/// The symbol a currency is written with, or its ISO code when we do not know
/// one. An unknown code renders as itself — honest, and never a wrong symbol.
fn symbol(currency: &str) -> &str {
    match currency {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        "JPY" => "¥",
        other => other,
    }
}

/// Formats minor units of `currency` for one locale, e.g. `€12.50` in English
/// and `12,50 €` in French. The result is plain text; the caller escapes it
/// like every other value.
pub(crate) fn format_price(minor_units: i64, currency: &str, strings: &UiStrings) -> String {
    let currency = currency.trim().to_ascii_uppercase();
    let exponent = currency_exponent(&currency);
    let scale = 10_i64.pow(exponent);
    let (whole, fraction) = (minor_units / scale, (minor_units % scale).abs());
    let mut amount = group(whole, strings.group_separator);
    if exponent > 0 {
        amount.push_str(strings.decimal_separator);
        amount.push_str(&format!(
            "{fraction:0>width$}",
            width = usize::try_from(exponent).unwrap_or(2)
        ));
    }
    let symbol = symbol(&currency);
    // A non-breaking space either way: a price must never wrap between its
    // number and its currency, and an unknown code ("SEK 120") needs the gap
    // as much as a symbol reads well with one.
    if strings.price_symbol_leads {
        format!("{symbol}\u{a0}{amount}")
    } else {
        format!("{amount}\u{a0}{symbol}")
    }
}

/// Thousands grouping, written out rather than reached for a formatting crate:
/// this is the whole of what a price needs, and a dependency would be a third
/// spelling of the same three lines.
fn group(whole: i64, separator: &str) -> String {
    let digits = whole.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    if whole < 0 {
        out.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push_str(separator);
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{EN, FR, NL};
    use super::format_price;

    #[test]
    fn english_leads_with_the_symbol_and_a_point() {
        assert_eq!(format_price(1_250, "EUR", &EN), "€\u{a0}12.50");
        assert_eq!(format_price(1_234_500, "EUR", &EN), "€\u{a0}12,345.00");
        assert_eq!(format_price(0, "EUR", &EN), "€\u{a0}0.00");
    }

    #[test]
    fn french_and_dutch_write_the_comma() {
        assert_eq!(format_price(1_250, "EUR", &FR), "12,50\u{a0}€");
        assert_eq!(format_price(1_250, "EUR", &NL), "€\u{a0}12,50");
        assert_eq!(format_price(123_450, "EUR", &NL), "€\u{a0}1.234,50");
    }

    #[test]
    fn a_currency_without_minor_units_shows_no_decimals() {
        assert_eq!(format_price(1_200, "JPY", &EN), "¥\u{a0}1,200");
        assert_eq!(format_price(12_500, "KWD", &EN), "KWD\u{a0}12.500");
    }
}
