//! Safe, mechanical rewrites of Cooklang produced by an LLM.
//!
//! Every fix here mirrors a condition in `tokens::check` / `metadata::check`
//! exactly: `autofix` only ever touches text that `lint` would mark
//! `autofixable: true`, and it clears every such issue. It never changes a
//! digit, drops a step, or adds an ingredient.

use super::metadata::{self, ENTITY, KEY_RENAMES, PLACEHOLDERS, TOP_LEVEL};
use super::split_frontmatter;
use super::tokens::{self, fixed_quantity, fixed_unit_as_note, INGREDIENT};
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// A bare stray `{}` left after a unit moved out of the brace content — the
/// only StrayBraces shape `check` marks autofixable (a `{2}` might hold a
/// real, if misplaced, value and is left alone).
static STRAY_EMPTY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\}\{\}").unwrap());
/// An unescaped approximation tilde directly before a digit ("~2 cm"), as
/// opposed to a real timer ("~{2%minutes}"), which never has a digit
/// immediately after the `~`.
static TILDE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"~\s*(\d)").unwrap());
/// Any `@`/`#`/`~` token appearing on a `>` note line — Cooklang notes may
/// not contain syntax at all, so every match here is rewritten to plain
/// words: an ingredient/cookware token keeps just its name (group 1); a
/// timer keeps its quantity and unit as plain text (group 2), with `%`
/// turned into a space.
static NOTE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[@#]\??([^@#~{}\n]+?)\{[^}\n]*\}(?:\s*\([^)\n]*\))?|~\??[^@#~{}\n]*\{([^}\n]*)\}(?:\([^)\n]*\))?",
    )
    .unwrap()
});

fn fix_note_line(line: &str) -> String {
    NOTE_TOKEN
        .replace_all(line, |c: &Captures| {
            if let Some(name) = c.get(1) {
                name.as_str().trim().to_string()
            } else if let Some(inner) = c.get(2) {
                inner.as_str().replace('%', " ").split_whitespace().collect::<Vec<_>>().join(" ")
            } else {
                String::new()
            }
        })
        .into_owned()
}

/// Rewrites one `INGREDIENT` match, applying the UnitAsNote and
/// MissingPercent fixes exactly when `tokens::check` marks them
/// autofixable.
///
/// `skip` mirrors `check`'s comment/note-line exemption for MissingPercent
/// (UnitAsNote is never skipped there, so it isn't here either).
///
/// Group 3 (the whitespace between `}` and an optional note) is captured by
/// `INGREDIENT` even when there's no note — a greedy `\s*` with nothing to
/// its right still consumes trailing whitespace into the match. Any branch
/// that doesn't rewrite the note must splice it (and group 4) back
/// unchanged, or that whitespace is silently dropped: `@x{2 cups} and`
/// would become `@x{2%cups}and`.
fn fix_ingredient_token(cap: &Captures, skip: bool) -> String {
    let whole = &cap[0];
    let name = &cap[1];
    let content = &cap[2];
    let ws = &cap[3];
    let note = cap.get(4).map(|m| m.as_str());
    let prefix = if whole.starts_with("@?") { "@?" } else { "@" };

    if let Some(note_full) = note {
        let note_inner = note_full.trim_start_matches('(').trim_end_matches(')');
        // `check` only treats a space-preceded parenthetical as a note when
        // the *whole* thing is a single unit word — mirrored here by only
        // accepting the fix when nothing is left over (`remaining.is_none()`
        // is equivalent to a full-string UNIT_WORD_EXACT match, since a
        // partial UNIT_WORD prefix match with no leftover text has, by
        // construction, matched the entire trimmed note).
        let has_space = !ws.is_empty();
        if let Some((new_content, remaining)) = fixed_unit_as_note(content, note_inner) {
            if !has_space || remaining.is_none() {
                return match remaining {
                    Some(rest) => format!("{prefix}{name}{{{new_content}}}({rest})"),
                    None => format!("{prefix}{name}{{{new_content}}}"),
                };
            }
        }
    }

    if !skip {
        if let Some(new_content) = fixed_quantity(content) {
            let note = note.unwrap_or("");
            return format!("{prefix}{name}{{{new_content}}}{ws}{note}");
        }
    }

    whole.to_string()
}

fn fix_body(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    for (i, line) in body.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if line.trim_start().starts_with('>') {
            // Notes may not contain Cooklang syntax at all: strip every
            // token down to plain words.
            out.push_str(&fix_note_line(line));
            continue;
        }
        let skip = tokens::is_comment_or_note_line(line);
        let line = line.replace("@?:", "@?");
        let line = STRAY_EMPTY.replace_all(&line, "}");
        let line = INGREDIENT.replace_all(&line, |c: &Captures| fix_ingredient_token(c, skip));
        let line =
            if skip { line.into_owned() } else { TILDE.replace_all(&line, "about $1").into_owned() };
        out.push_str(&line);
    }
    out
}

fn fix_frontmatter(fm: &str) -> String {
    let lines: Vec<&str> = fm.lines().collect();
    let mut kept = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(cap) = TOP_LEVEL.captures(line) {
            let value = &cap[2];
            if PLACEHOLDERS.contains(&value) && !metadata::has_children(&lines, i) {
                continue;
            }
            if let Some((_, to)) = KEY_RENAMES.iter().find(|(from, _)| *from == &cap[1]) {
                kept.push(format!("{to}: {value}"));
                continue;
            }
        }
        kept.push(line.to_string());
    }
    let joined = kept.join("\n");
    ENTITY
        .replace_all(&joined, |c: &Captures| html_escape::decode_html_entities(&c[0]).into_owned())
        .into_owned()
}

/// Applies the safe mechanical fixes. Never changes a number, drops a step,
/// or adds an ingredient.
pub fn autofix(cooklang: &str) -> String {
    match split_frontmatter(cooklang) {
        (Some(fm), body) => format!("---\n{}\n---\n\n{}", fix_frontmatter(fm), fix_body(body)),
        (None, body) => fix_body(body),
    }
}
