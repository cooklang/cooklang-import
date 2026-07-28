//! A structured ingredient-list entry assembled from model tags.

use super::bio::{decode, WordTag};
use super::linker::is_unit_word;
use super::words::Word;

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub qty: Option<String>,
    pub unit: Option<String>,
    pub prep: Option<String>,
}

impl Entry {
    pub fn from_tagged(words: &[Word], tags: &[WordTag]) -> Option<Self> {
        let mut name = Vec::new();
        let mut qty = Vec::new();
        let mut unit = Vec::new();
        let mut prep = Vec::new();
        for span in decode(tags, 0.5) {
            let texts: Vec<&str> = words[span.start..span.end]
                .iter()
                .map(|w| w.text.as_str())
                .collect();
            match span.kind.as_str() {
                "NAME" => name.extend(texts),
                "QTY" => qty.extend(texts),
                "UNIT" => unit.extend(texts),
                "PREP" => prep.extend(texts),
                _ => {}
            }
        }
        if name.is_empty() {
            return None;
        }
        let join = |v: Vec<&str>| {
            if v.is_empty() {
                None
            } else {
                Some(v.join(" "))
            }
        };
        Some(Entry {
            name: name.join(" "),
            qty: join(qty),
            unit: join(unit),
            prep: join(prep),
        })
    }
}

/// Does `t` look like a leading quantity token — an ASCII digit start (also
/// covers a glued mixed number like "1½", since the digit comes first) or a
/// standalone vulgar-fraction glyph? Mirrors `assemble::local_amount`'s
/// definition so list-line repair and step-text local-amount detection agree
/// on what counts as a quantity.
fn is_qty_token(t: &str) -> bool {
    t.chars().next().is_some_and(|c| c.is_ascii_digit())
        || matches!(t, "½" | "¼" | "¾" | "⅓" | "⅔" | "⅛")
}

/// Fill a missing qty/unit from the ingredient line's leading tokens when
/// the model didn't tag them — this is a recall gap (the tagger simply
/// didn't emit a B-QTY/B-UNIT tag for a token it should have), not a case of
/// the token being unrepresentable, so recovering it from the literal text
/// is safe: it never introduces information that wasn't in the source line.
///
/// Only fills fields the model left empty; a tag the model *did* produce is
/// never touched, even if this heuristic would have guessed differently.
pub fn repair_qty_unit(entry: &mut Entry, words: &[Word]) {
    if entry.qty.is_some() {
        return;
    }
    let Some(first) = words.first() else {
        return;
    };
    if !is_qty_token(&first.text) {
        return;
    }
    entry.qty = Some(first.text.clone());
    if entry.unit.is_none() {
        if let Some(second) = words.get(1) {
            if is_unit_word(&second.text) {
                entry.unit = Some(second.text.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build (words, tags) from a list of (text, label) pairs. Byte offsets
    /// are irrelevant to `from_tagged`, so this keeps fixtures readable.
    fn build(pairs: &[(&str, &str)]) -> (Vec<Word>, Vec<WordTag>) {
        let mut words = Vec::new();
        let mut tags = Vec::new();
        let mut pos = 0usize;
        for (text, label) in pairs {
            let end = pos + text.len();
            words.push(Word {
                text: text.to_string(),
                start: pos,
                end,
            });
            tags.push(WordTag {
                label: label.to_string(),
                confidence: 0.9,
            });
            pos = end + 1;
        }
        (words, tags)
    }

    #[test]
    fn assembles_entry_from_tagged_words() {
        let (words, tags) = build(&[
            ("0.25", "B-QTY"),
            ("cup", "B-UNIT"),
            ("unsalted", "B-NAME"),
            ("butter", "I-NAME"),
            (",", "O"),
            ("softened", "B-PREP"),
        ]);
        let entry = Entry::from_tagged(&words, &tags).unwrap();
        assert_eq!(entry.name, "unsalted butter");
        assert_eq!(entry.qty.as_deref(), Some("0.25"));
        assert_eq!(entry.unit.as_deref(), Some("cup"));
        assert_eq!(entry.prep.as_deref(), Some("softened"));
    }

    #[test]
    fn no_name_span_yields_none() {
        let (words, tags) = build(&[("2", "B-QTY"), ("cups", "B-UNIT")]);
        assert!(Entry::from_tagged(&words, &tags).is_none());
    }

    #[test]
    fn repair_fills_missing_qty_and_unit_from_leading_tokens() {
        // Model tagged only NAME (a recall gap on the leading vulgar
        // fraction), qty/unit both missing.
        let (words, tags) = build(&[("½", "O"), ("cup", "O"), ("sugar", "B-NAME")]);
        let mut e = Entry::from_tagged(&words, &tags).unwrap();
        assert_eq!(e.qty, None);
        assert_eq!(e.unit, None);
        repair_qty_unit(&mut e, &words);
        assert_eq!(e.qty.as_deref(), Some("½"));
        assert_eq!(e.unit.as_deref(), Some("cup"));
    }

    #[test]
    fn repair_handles_glued_mixed_number() {
        let (words, tags) = build(&[("1½", "O"), ("cups", "O"), ("flour", "B-NAME")]);
        let mut e = Entry::from_tagged(&words, &tags).unwrap();
        repair_qty_unit(&mut e, &words);
        assert_eq!(e.qty.as_deref(), Some("1½"));
        assert_eq!(e.unit.as_deref(), Some("cups"));
    }

    #[test]
    fn repair_never_overrides_model_tagged_qty_or_unit() {
        let (words, tags) = build(&[("2", "B-QTY"), ("cups", "B-UNIT"), ("flour", "B-NAME")]);
        let mut e = Entry::from_tagged(&words, &tags).unwrap();
        repair_qty_unit(&mut e, &words);
        assert_eq!(e.qty.as_deref(), Some("2"));
        assert_eq!(e.unit.as_deref(), Some("cups"));
    }

    #[test]
    fn repair_fills_qty_only_when_no_unit_word_follows() {
        // "unit" list-line convention: a bare count with no real unit.
        let (words, tags) = build(&[("2", "O"), ("garlic", "B-NAME")]);
        let mut e = Entry::from_tagged(&words, &tags).unwrap();
        repair_qty_unit(&mut e, &words);
        assert_eq!(e.qty.as_deref(), Some("2"));
        assert_eq!(e.unit, None);
    }

    #[test]
    fn repair_does_nothing_when_first_token_is_not_a_quantity() {
        let (words, tags) = build(&[("chopped", "O"), ("scallions", "B-NAME")]);
        let mut e = Entry::from_tagged(&words, &tags).unwrap();
        repair_qty_unit(&mut e, &words);
        assert_eq!(e.qty, None);
        assert_eq!(e.unit, None);
    }
}
