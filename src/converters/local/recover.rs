//! Recovery pass: catch ingredient-list entries the tagger's ING tagging
//! missed in the step text, but ONLY when the entry's name literally (modulo
//! case/plural folding) appears somewhere in the words — this never invents
//! a mention. Entries whose names genuinely never occur in any step are
//! left uncovered; that's a real content gap between the recipe's
//! ingredient list and its instructions (or a silver-gold divergence, when
//! comparing against LLM-generated reference data), not a bug to paper
//! over.

use super::bio::Span;
use super::entry::Entry;
use super::linker::norm;
use super::words::{word_tokenize, Word};

/// For each entry not yet marked `covered`, look for the first contiguous,
/// normalized match of its (word-tokenized) name in `words` and, if the hit
/// doesn't overlap a span the tagger already produced, append a synthetic
/// `"ING"` span covering it and mark the entry covered.
///
/// This only supplies the missing span — downstream assembly re-runs the
/// same deterministic linker/amount rules against it exactly as it would
/// for a tagger-produced span, so no amount or markup decision is made
/// here.
pub fn recover_unmentioned(
    words: &[Word],
    spans: &mut Vec<Span>,
    entries: &[Entry],
    covered: &mut [bool],
) {
    for (i, entry) in entries.iter().enumerate() {
        if covered[i] {
            continue;
        }
        let Some((start, end)) = find_name_run(words, &entry.name) else {
            continue;
        };
        if spans
            .iter()
            .any(|s| ranges_overlap(s.start, s.end, start, end))
        {
            continue;
        }
        spans.push(Span {
            kind: "ING".to_string(),
            start,
            end,
        });
        covered[i] = true;
    }
}

/// First word-index run in `words` whose normalized tokens equal `name`'s
/// normalized tokens, in order — e.g. `name` "Ground Beef" matches a
/// "ground beef" or "Ground Beefs" run in `words`.
fn find_name_run(words: &[Word], name: &str) -> Option<(usize, usize)> {
    let needle: Vec<String> = word_tokenize(name)
        .iter()
        .map(|w| norm(&w.text))
        .filter(|t| !t.is_empty())
        .collect();
    if needle.is_empty() || needle.len() > words.len() {
        return None;
    }
    'outer: for start in 0..=(words.len() - needle.len()) {
        for (k, tok) in needle.iter().enumerate() {
            if norm(&words[start + k].text) != *tok {
                continue 'outer;
            }
        }
        return Some((start, start + needle.len()));
    }
    None
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> Entry {
        Entry {
            name: name.to_string(),
            qty: Some("10".into()),
            unit: Some("ounce".into()),
            prep: None,
        }
    }

    #[test]
    fn recovers_a_literal_mention_the_tagger_missed() {
        let text = "Season the ground beef well.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        let mut spans: Vec<Span> = Vec::new(); // tagger produced nothing
        let mut covered = vec![false];

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert!(covered[0]);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, "ING");
        let surface: Vec<&str> = words[spans[0].start..spans[0].end]
            .iter()
            .map(|w| w.text.as_str())
            .collect();
        assert_eq!(surface, vec!["ground", "beef"]);
    }

    #[test]
    fn matches_case_and_plural_insensitively() {
        let text = "Stir the GROUND BEEFS into the sauce.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        let mut spans: Vec<Span> = Vec::new();
        let mut covered = vec![false];

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert!(covered[0]);
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn does_not_recover_already_covered_entries() {
        let text = "Season the ground beef well.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        let mut spans: Vec<Span> = Vec::new();
        let mut covered = vec![true]; // already linked by a real tagged span elsewhere

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert!(spans.is_empty());
    }

    #[test]
    fn does_not_recover_when_name_never_appears() {
        let text = "Season the chicken well.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        let mut spans: Vec<Span> = Vec::new();
        let mut covered = vec![false];

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert!(spans.is_empty());
        assert!(
            !covered[0],
            "genuine silver-gold divergence, stays unmarked"
        );
    }

    #[test]
    fn skips_a_match_that_overlaps_an_existing_span() {
        let text = "Season the ground beef well.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        // Words 2..4 are "ground beef" — pretend the tagger already claimed
        // that range for something else; the existing span wins.
        let mut spans = vec![Span {
            kind: "COOK".to_string(),
            start: 2,
            end: 4,
        }];
        let mut covered = vec![false];

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert_eq!(spans.len(), 1);
        assert!(!covered[0]);
    }

    #[test]
    fn recovers_at_most_one_span_per_entry() {
        let text = "Ground beef, then more ground beef.";
        let words = word_tokenize(text);
        let entries = vec![entry("Ground Beef")];
        let mut spans: Vec<Span> = Vec::new();
        let mut covered = vec![false];

        recover_unmentioned(&words, &mut spans, &entries, &mut covered);

        assert_eq!(spans.len(), 1, "only the first occurrence is recovered");
    }
}
