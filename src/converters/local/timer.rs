//! Rule-based (qty, unit) extraction from a TIMER span's text.

use super::words::word_tokenize;

const UNITS: &[&str] = &[
    "second", "seconds", "sec", "minute", "minutes", "min", "mins", "hour", "hours", "hr", "hrs",
];

/// Operates on `word_tokenize` output rather than `split_whitespace` so
/// punctuation is its own token: a trailing comma after the unit ("20
/// minutes,") or a leading paren before the quantity ("(10 minutes)") never
/// glues onto the word we're matching against.
pub fn parse_timer(text: &str) -> Option<(String, String)> {
    let words = word_tokenize(text);
    let unit_idx = words
        .iter()
        .position(|w| UNITS.contains(&w.text.to_lowercase().as_str()))?;
    let qty = words[..unit_idx].iter().rev().find(|w| {
        w.text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '½' | '¼' | '¾'))
    })?;
    Some((qty.text.clone(), words[unit_idx].text.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_quantity_and_unit() {
        assert_eq!(
            parse_timer("20 minutes"),
            Some(("20".to_string(), "minutes".to_string()))
        );
    }

    #[test]
    fn range_quantity() {
        assert_eq!(
            parse_timer("4-5 minutes"),
            Some(("4-5".to_string(), "minutes".to_string()))
        );
    }

    #[test]
    fn quantity_not_immediately_before_unit() {
        assert_eq!(
            parse_timer("about 2 hours"),
            Some(("2".to_string(), "hours".to_string()))
        );
    }

    #[test]
    fn no_unit_recognized() {
        assert_eq!(parse_timer("until golden"), None);
    }

    #[test]
    fn trailing_comma_is_not_captured_in_unit() {
        assert_eq!(
            parse_timer("Bake for 20 minutes, then cool"),
            Some(("20".to_string(), "minutes".to_string()))
        );
    }

    #[test]
    fn quantity_after_open_paren_is_found() {
        assert_eq!(
            parse_timer("cook (10 minutes) covered"),
            Some(("10".to_string(), "minutes".to_string()))
        );
    }
}
