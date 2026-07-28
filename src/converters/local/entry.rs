//! A structured ingredient-list entry assembled from model tags.

use super::bio::{decode, WordTag};
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
}
