//! BIO tag sequence -> word-index spans, with a mean-confidence floor.
//! Spec: an unmarked ingredient is a smaller sin than a wrong quantity —
//! low-confidence spans are skipped, not guessed.

#[derive(Debug, Clone)]
pub struct WordTag {
    pub label: String, // "O" | "B-ING" | "I-COOK" | ...
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub kind: String, // "ING" | "COOK" | "TIMER" | "QTY" | ...
    pub start: usize, // word indices, half-open
    pub end: usize,
}

pub fn decode(tags: &[WordTag], min_confidence: f32) -> Vec<Span> {
    let mut spans: Vec<(Span, f32, usize)> = Vec::new();
    for (i, t) in tags.iter().enumerate() {
        let (prefix, kind) = match t.label.split_once('-') {
            Some((p, k)) => (p, k),
            None => continue, // "O"
        };
        let extend = prefix == "I"
            && spans
                .last()
                .is_some_and(|(s, _, _)| s.kind == kind && s.end == i);
        if extend {
            let (s, c, n) = spans.last_mut().unwrap();
            s.end = i + 1;
            *c += t.confidence;
            *n += 1;
        } else {
            spans.push((
                Span {
                    kind: kind.to_string(),
                    start: i,
                    end: i + 1,
                },
                t.confidence,
                1,
            ));
        }
    }
    spans
        .into_iter()
        .filter(|(_, c, n)| c / *n as f32 >= min_confidence)
        .map(|(s, _, _)| s)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(l: &str, c: f32) -> WordTag {
        WordTag {
            label: l.to_string(),
            confidence: c,
        }
    }

    #[test]
    fn decodes_contiguous_spans() {
        let tags = vec![
            tag("O", 0.9),
            tag("B-ING", 0.9),
            tag("I-ING", 0.9),
            tag("O", 0.9),
            tag("B-TIMER", 0.9),
        ];
        let spans = decode(&tags, 0.5);
        assert_eq!(
            spans,
            vec![
                Span {
                    kind: "ING".into(),
                    start: 1,
                    end: 3
                },
                Span {
                    kind: "TIMER".into(),
                    start: 4,
                    end: 5
                },
            ]
        );
    }

    #[test]
    fn low_confidence_spans_are_dropped() {
        let tags = vec![tag("B-ING", 0.3), tag("I-ING", 0.4)];
        assert!(decode(&tags, 0.5).is_empty());
    }

    #[test]
    fn orphan_i_starts_a_span() {
        let tags = vec![tag("I-COOK", 0.9)];
        assert_eq!(
            decode(&tags, 0.5),
            vec![Span {
                kind: "COOK".into(),
                start: 0,
                end: 1
            }]
        );
    }
}
