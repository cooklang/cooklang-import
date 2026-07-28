//! Word-level splitter — MUST stay in lockstep with Python's
//! recipe_parser/tokenize.py (parity locked by tests/local_parity.rs).

#[derive(Debug, Clone)]
pub struct Word {
    pub text: String,
    pub start: usize, // byte offsets into the source
    pub end: usize,
}

pub fn word_tokenize(text: &str) -> Vec<Word> {
    // Equivalent of r"\w+(?:[-/.]\w+)*|[^\w\s]" without the regex crate.
    let mut out = Vec::new();
    let mut iter = text.char_indices().peekable();
    while let Some(&(start, c)) = iter.peek() {
        if c.is_whitespace() {
            iter.next();
        } else if is_word_char(c) {
            let mut end = start + c.len_utf8();
            iter.next();
            while let Some(&(i, ch)) = iter.peek() {
                if is_word_char(ch) {
                    end = i + ch.len_utf8();
                    iter.next();
                } else if matches!(ch, '-' | '/' | '.') {
                    let mut ahead = iter.clone();
                    ahead.next();
                    match ahead.peek() {
                        Some(&(_, nxt)) if is_word_char(nxt) => {
                            // glue only when a word char follows (4-5, 1.5);
                            // consume the separator here, the next loop
                            // iteration consumes and extends `end` over `nxt`.
                            iter.next();
                            end = i + ch.len_utf8();
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }
            out.push(Word {
                text: text[start..end].to_string(),
                start,
                end,
            });
        } else {
            let end = start + c.len_utf8();
            out.push(Word {
                text: text[start..end].to_string(),
                start,
                end,
            });
            iter.next();
        }
    }
    out
}

fn is_word_char(c: char) -> bool {
    // Python \w matches everything str.isalnum() does, INCLUDING category-No
    // characters like ½ (so "1½" is one token). Rust's is_alphanumeric()
    // covers the same alphabetic + Nd/Nl/No set — no exclusions needed.
    // (An earlier version wrongly excluded vulgar fractions based on a stale
    // comment in the Python source; the parity fixture's "1½ cups" case
    // guards the corrected behavior.)
    c.is_alphanumeric() || c == '_'
}
