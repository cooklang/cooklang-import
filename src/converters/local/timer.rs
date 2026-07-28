//! Rule-based (qty, unit) extraction from a TIMER span's text.

const UNITS: &[&str] = &[
    "second", "seconds", "sec", "minute", "minutes", "min", "mins", "hour", "hours", "hr", "hrs",
];

pub fn parse_timer(text: &str) -> Option<(String, String)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let unit_idx = words.iter().position(|w| {
        UNITS.contains(
            &w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
                .as_str(),
        )
    })?;
    let qty = words[..unit_idx].iter().rev().find(|w| {
        w.chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '½' | '¼' | '¾'))
    })?;
    Some((
        qty.to_string(),
        words[unit_idx].trim_end_matches('.').to_lowercase(),
    ))
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
}
