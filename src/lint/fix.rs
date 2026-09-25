//! Safe, mechanical rewrites of Cooklang produced by an LLM.
//!
//! Every fix here mirrors a condition in `tokens::check` / `metadata::check`
//! exactly: `autofix` only ever touches text that `lint` would mark
//! `autofixable: true`, and it clears every such issue. It never changes a
//! digit, drops a step, or adds an ingredient.
//!
//! Follow-up (out of scope here, see the coordinator's I5): the right long
//! term shape for this module is `lint` emitting span-level edits alongside
//! each issue, with `autofix` just applying them — that would make "check
//! and fix agree" true by construction instead of by convention (shared
//! predicates like `tokens::unit_as_note_fix`/`tokens::fixed_quantity`, plus
//! the property test in `tests/test_lint_property.rs`, are the guard for
//! now).

use super::metadata;
use super::split_frontmatter;
use super::tokens::{self, fixed_quantity, unit_as_note_fix, INGREDIENT};
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// A stray `{}` (one or more, back to back) left after a unit moved out of
/// the brace content — the only StrayBraces shape `check` marks
/// autofixable (a `{2}` might hold a real, if misplaced, value and is left
/// alone). Matches the *whole* run in one shot (`(?:\{\})+`) so
/// `@x{2%g}{}{}{}` collapses to `@x{2%g}` in a single pass instead of
/// needing one pass per stray pair.
static STRAY_EMPTY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\}(?:\{\})+").unwrap());
/// `@?:` with one or more colons — `@\?:+` rather than a literal `"@?:"`
/// replace, so `@?::salt` (two colons) is fully cleared in one pass instead
/// of leaving one behind.
static OPTIONAL_COLON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@\?:+").unwrap());
/// An unescaped approximation tilde directly before a digit ("~2 cm"), as
/// opposed to a real timer ("~{2%minutes}"), which never has a digit
/// immediately after the `~`. Applied only outside token spans (see
/// `fix_tilde`) — the same scope `tokens::check` uses.
static TILDE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"~\s*(\d)").unwrap());
/// Any `@`/`#`/`~` token on a `>` note line — Cooklang notes may not contain
/// syntax at all. Mirrors `INGREDIENT`'s shape (name / content / whitespace
/// / optional note) so the same glued-vs-spaced note rule applies, plus a
/// leading sigil capture so one regex handles ingredients, cookware, and
/// timers. The name group allows an empty match (`@{}`, `#{2}`), matching
/// `ANY_TOKEN`'s own shape.
static NOTE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([@#~])\??([^@#~{}\n]*?)\{([^}\n]*)\}(\s*)(\([^)\n]*\))?").unwrap()
});

/// Rewrites one `NOTE_TOKEN` match to plain text that keeps its quantity —
/// notes may not contain Cooklang syntax, but the words in the token still
/// belong in the recipe. `@cheese{50%g}(grated)` -> `50 g cheese (grated)`;
/// `@salt{}` -> `salt`; `~{10%minutes}` -> `10 minutes`; `~rest{5%min}` ->
/// `rest 5 min` (a named timer reads name-then-duration, the reverse of an
/// ingredient's quantity-then-name). A parenthetical counts as the token's
/// note only when it's glued to `}` (no whitespace before it) — the same
/// rule `INGREDIENT` uses; a space-preceded parenthetical is ordinary prose
/// and is reproduced unchanged.
fn fix_note_line(line: &str) -> String {
    NOTE_TOKEN
        .replace_all(line, |c: &Captures| {
            let sigil = &c[1];
            let name = c[2].trim();
            let quantity_owned = c[3].replace('%', " ");
            let quantity = quantity_owned.trim();
            let ws = &c[4];
            let note = c.get(5).map(|m| m.as_str());

            let mut combined = String::new();
            let push_part = |part: &str, combined: &mut String| {
                if part.is_empty() {
                    return;
                }
                if !combined.is_empty() {
                    combined.push(' ');
                }
                combined.push_str(part);
            };
            if sigil == "~" {
                push_part(name, &mut combined);
                push_part(quantity, &mut combined);
            } else {
                push_part(quantity, &mut combined);
                push_part(name, &mut combined);
            }

            // `ws` (the whitespace between `}` and an optional note) is part
            // of the overall match even when there's no note at all — same
            // greedy-`\s*`-with-nothing-to-its-right situation as
            // `INGREDIENT`. Reproduce it unconditionally, or e.g. `#{2}
            // here` loses its space and becomes `2here`.
            let has_space = !ws.is_empty();
            combined.push_str(ws);
            if let Some(note_full) = note {
                if has_space {
                    // Ordinary prose, not a note: reproduce it unchanged
                    // (the separating whitespace was already appended above).
                    combined.push_str(note_full);
                } else {
                    let note_inner = note_full.trim_start_matches('(').trim_end_matches(')');
                    if !combined.is_empty() {
                        combined.push(' ');
                    }
                    combined.push('(');
                    combined.push_str(note_inner);
                    combined.push(')');
                }
            }
            combined
        })
        .into_owned()
}

/// Applies the tilde-approximation fix, but only outside any `@`/`#`/`~`
/// token span — the same scope `tokens::check` uses (see
/// `tokens::token_spans`). Without this, `@x{~2%g}` (a tilde that's really
/// part of the quantity, sitting inside braces) would get rewritten into
/// `@x{about 2%g}`, a token nothing ever flagged.
fn fix_tilde(line: &str) -> String {
    let spans = tokens::token_spans(line);
    TILDE
        .replace_all(line, |c: &Captures| {
            let m = c.get(0).unwrap();
            if spans.iter().any(|&(s, e)| m.start() >= s && m.end() <= e) {
                m.as_str().to_string()
            } else {
                format!("about {}", &c[1])
            }
        })
        .into_owned()
}

/// Rewrites one `INGREDIENT` match, applying the UnitAsNote and
/// MissingPercent fixes exactly when `tokens::check` marks them
/// autofixable (via the same shared predicates it uses).
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
        let has_space = !ws.is_empty();
        if let Some((new_content, remaining)) = unit_as_note_fix(content, note_inner, has_space) {
            return match remaining {
                Some(rest) => format!("{prefix}{name}{{{new_content}}}({rest})"),
                None => format!("{prefix}{name}{{{new_content}}}"),
            };
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
        // Both of these are flagged everywhere in the body (StrayBraces and
        // OptionalColon aren't exempted on comment/note lines by `check`),
        // so they're applied everywhere too, before the note/non-note split
        // below — otherwise a bare "@?:" or "}{}" inside a `>` note line
        // (which has no enclosing token for `fix_note_line` to rewrite)
        // would stay flagged forever.
        let line = OPTIONAL_COLON.replace_all(line, "@?");
        let line = STRAY_EMPTY.replace_all(&line, "}");

        if line.trim_start().starts_with('>') {
            out.push_str(&fix_note_line(&line));
            continue;
        }
        let skip = tokens::is_comment_or_note_line(&line);
        let line = INGREDIENT.replace_all(&line, |c: &Captures| fix_ingredient_token(c, skip));
        let line = if skip { line.into_owned() } else { fix_tilde(&line) };
        out.push_str(&line);
    }
    out
}

fn fix_frontmatter(fm: &str) -> String {
    let lines: Vec<&str> = fm.lines().collect();
    let mut working: Vec<String> =
        metadata::apply_structural_fixes(&lines).into_iter().map(|(_, l)| l).collect();
    for i in 0..working.len() {
        if let Some(new_line) = metadata::entity_fix_for_line(&working, i) {
            working[i] = new_line;
        }
    }
    working.join("\n")
}

/// Whether `cooklang` has any issue `lint` would mark `autofixable: true`.
/// Deliberately cheap and narrow: only the mechanical checks that don't need
/// the original text (frontmatter + token-syntax) can ever be autofixable —
/// `parse` diagnostics and `consistency` issues never are.
fn has_autofixable_issue(cooklang: &str) -> bool {
    let (fm, body) = split_frontmatter(cooklang);
    if fm.is_some_and(|fm| metadata::check(fm).iter().any(|i| i.autofixable)) {
        return true;
    }
    tokens::check(body).iter().any(|i| i.autofixable)
}

/// Converts every bare `\n` to `\r\n`, without doubling a `\r` that's
/// already there. `fix_body`'s own split-on-`\n`-then-rejoin-with-`\n`
/// naturally preserves whatever line ending the body used (a `\r` left
/// dangling at the end of a split piece just gets a fresh `\n` appended
/// after it), but the frontmatter template and `fix_frontmatter`'s internal
/// `join("\n")` always produce bare `\n` — this brings the whole document
/// back in line with whatever ending the original used.
fn to_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => out.push_str("\r\n"),
            '\r' => {
                out.push('\r');
                if chars.peek() == Some(&'\n') {
                    out.push('\n');
                    chars.next();
                } else {
                    out.push('\n');
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Applies the safe mechanical fixes. Never changes a number, drops a step,
/// or adds an ingredient.
///
/// When `cooklang` has no autofixable issue at all, this returns it
/// completely unchanged — byte for byte, including a leading BOM, `\r\n`
/// line endings, and blank-line layout the reconstruction template below
/// wouldn't otherwise preserve. That's not just tidiness: it's the only way
/// to guarantee `autofix(x) == x` for arbitrary already-clean input, which
/// the property test in `tests/test_lint_property.rs` checks directly.
pub fn autofix(cooklang: &str) -> String {
    if !has_autofixable_issue(cooklang) {
        return cooklang.to_string();
    }
    let bom = cooklang.starts_with('\u{FEFF}');
    let crlf = cooklang.contains("\r\n");
    let rebuilt = match split_frontmatter(cooklang) {
        (Some(fm), body) => format!("---\n{}\n---\n\n{}", fix_frontmatter(fm), fix_body(body)),
        (None, body) => fix_body(body),
    };
    let rebuilt = if crlf { to_crlf(&rebuilt) } else { rebuilt };
    if bom && !rebuilt.starts_with('\u{FEFF}') {
        format!("\u{FEFF}{rebuilt}")
    } else {
        rebuilt
    }
}
