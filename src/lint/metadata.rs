use super::{Issue, Kind, Severity};
use regex::{Captures, Regex};
use std::collections::HashSet;
use std::sync::LazyLock;

pub(super) static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&(?:[a-zA-Z]{2,8}|#\d{2,5});").unwrap());
/// Top-level `key: value` line.
pub(super) static TOP_LEVEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_ ]*):\s*(.*?)\s*$").unwrap());
pub(super) const PLACEHOLDERS: &[&str] = &["", "''", "\"\"", "null", "'null'", "\"null\"", "~"];
pub(super) const KEY_RENAMES: &[(&str, &str)] = &[
    ("source_url", "source"),
    ("prep_time", "prep time"),
    ("cook_time", "cook time"),
    ("total_time", "time required"),
    ("image_url", "image"),
];
/// Characters that make a decoded value unsafe as a bare (unquoted) YAML
/// scalar: mapping/flow-collection/anchor/alias/tag/directive indicators,
/// quote characters, and the comment marker. Whenever a decoded entity value
/// contains one of these, it must be re-emitted as a double-quoted scalar or
/// it can silently change what the frontmatter means (`Foo&#58; bar` -> a
/// second mapping value; `Pie &#35;2` -> a truncated YAML comment; `A&#10;
/// author: evil` -> an injected key).
const YAML_SPECIAL: &[char] =
    &[':', '#', '\'', '"', '[', ']', '{', '}', ',', '&', '*', '!', '|', '>', '%', '@'];
/// A cap on the entity-decoding fixed-point loop (see `decode_fixed_point`):
/// real values decode in 1-2 passes; this only matters for adversarial,
/// deeply nested encodings ("&amp;amp;amp;...;") and exists purely so a
/// pathological input can't loop forever. A value that doesn't converge
/// within this many passes is left alone entirely.
const MAX_ENTITY_PASSES: usize = 20;

/// A key has "children" when the next line that isn't blank *or a comment*
/// is indented (space or tab) or is a YAML sequence item (`- `). Comments
/// are skipped when searching so that
/// ```yaml
/// tags:
/// # a comment
///   - a
/// ```
/// still recognizes `tags` as having children instead of reading the
/// comment line as the answer and concluding there are none.
///
/// Exposed as `pub(super)` because `apply_structural_fixes` reuses this to
/// decide whether an empty-looking line is safe to delete.
pub(super) fn has_children(lines: &[&str], i: usize) -> bool {
    lines[i + 1..]
        .iter()
        .find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .is_some_and(|next| next.starts_with(' ') || next.starts_with('\t') || next.starts_with('-'))
}

/// The set of top-level keys that survive placeholder deletion, exactly as
/// originally spelled (before any rename). Used both to delete
/// empty/placeholder lines and — for `KEY_RENAMES` — to detect when a rename
/// would collide with a key that's already there (see `apply_structural_fixes`).
fn surviving_keys<'a>(lines: &[&'a str]) -> HashSet<&'a str> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let cap = TOP_LEVEL.captures(line)?;
            let deleted = PLACEHOLDERS.contains(&&cap[2]) && !has_children(lines, i);
            if deleted {
                None
            } else {
                Some(cap.get(1).unwrap().as_str())
            }
        })
        .collect()
}

/// Applies the two structural, line-shape fixes — deleting an
/// empty/placeholder line with no children, and renaming a drifted metadata
/// key — to `lines`, returning the kept lines paired with their original
/// index (so a caller doing further per-line fixes can still tell which
/// original line a kept line came from).
///
/// A rename is skipped (the line is kept unchanged) when the target key is
/// already used by another surviving line — applying it would create a
/// duplicate YAML key and break the frontmatter (e.g. `source: a` +
/// `source_url: b`, or `prep time: 5 min` + `prep_time: 10 min`). `check`
/// calls `surviving_keys` with the same rule to decide `autofixable`, so the
/// two never disagree about which renames are safe.
pub(super) fn apply_structural_fixes(lines: &[&str]) -> Vec<(usize, String)> {
    let survivors = surviving_keys(lines);
    let mut kept = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(cap) = TOP_LEVEL.captures(line) {
            let value = &cap[2];
            if PLACEHOLDERS.contains(&value) && !has_children(lines, i) {
                continue;
            }
            if let Some((_, to)) = KEY_RENAMES.iter().find(|(from, _)| *from == &cap[1]) {
                if !survivors.contains(to) {
                    kept.push((i, format!("{to}: {value}")));
                    continue;
                }
            }
        }
        kept.push((i, (*line).to_string()));
    }
    kept
}

/// Repeatedly decodes HTML entities in `value` until nothing changes.
/// Real exports sometimes double- (or deeper-) encode text
/// ("&amp;amp;" for a literal "&"); a single pass only partially decodes
/// that. Returns `None` if it doesn't converge within `MAX_ENTITY_PASSES` —
/// the caller then leaves the value untouched rather than risk an unbounded
/// loop on adversarial input. Returns `Some((value, false))` unchanged when
/// every entity present is one `html_escape` itself declines to decode
/// (unknown name, invalid numeric reference, etc.) — those are never worth
/// touching.
fn decode_fixed_point(value: &str) -> Option<(String, bool)> {
    let mut cur = value.to_string();
    let mut changed = false;
    for _ in 0..MAX_ENTITY_PASSES {
        let next = ENTITY.replace_all(&cur, |c: &Captures| {
            html_escape::decode_html_entities(&c[0]).into_owned()
        });
        if next == cur {
            return Some((cur, changed));
        }
        changed = true;
        cur = next.into_owned();
    }
    None
}

fn needs_quoting(s: &str) -> bool {
    s.is_empty()
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.contains('\n')
        || s.chars().any(|c| YAML_SPECIAL.contains(&c))
}

fn yaml_double_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Best-effort unescape of a YAML double-quoted scalar's body (just the
/// common escapes real recipe metadata could plausibly contain) — enough to
/// recover the intended text before re-decoding entities inside it, not a
/// full YAML-spec implementation.
fn unescape_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Whether replacing `original_lines[i]` with `candidate_lines[i]` (all
/// other lines unchanged) is a safe rewrite: the candidate must parse as
/// YAML, and — when the original also parsed — must keep exactly the same
/// top-level key set. When the original didn't parse at all, any candidate
/// that now parses is accepted (an improvement, never a regression).
fn round_trip_safe(original_lines: &[String], candidate_lines: &[String]) -> bool {
    let candidate_text = candidate_lines.join("\n");
    let Ok(cand_val) = serde_yaml::from_str::<serde_yaml::Value>(&candidate_text) else {
        return false;
    };
    let original_text = original_lines.join("\n");
    match serde_yaml::from_str::<serde_yaml::Value>(&original_text) {
        Ok(orig_val) => match (orig_val.as_mapping(), cand_val.as_mapping()) {
            (Some(om), Some(cm)) => {
                let mut ok: Vec<String> = om.keys().map(|k| format!("{k:?}")).collect();
                let mut ck: Vec<String> = cm.keys().map(|k| format!("{k:?}")).collect();
                ok.sort();
                ck.sort();
                ok == ck
            }
            (None, None) => true,
            _ => false,
        },
        Err(_) => true,
    }
}

/// Attempts a safe HTML-entity fix for `lines[i]`, returning the replacement
/// line when one exists. Scope is deliberately narrow — only a top-level
/// `key: value` line's value is touched, and only when doing so is
/// YAML-safe:
///
/// - A flow collection (`[...]`, `{...}`) is left alone entirely: decoding
///   an entity inside one element without corrupting the collection's shape
///   needs real YAML-aware editing, not a line-based text fix.
/// - A single- or double-quoted value has its surface quoting stripped
///   (unescaped) before decoding, and is always re-quoted as a double-quoted
///   scalar afterwards — regardless of the original style — so the result
///   only ever needs one, well-understood escaping scheme.
/// - The final candidate line is required to round-trip (`round_trip_safe`):
///   if decoding would break the YAML or add/remove a top-level key (e.g. an
///   injected `&#10;newkey: evil`), the line is left completely untouched.
///
/// Entities outside a top-level value (inside a key, a nested/indented line,
/// a list item, or a block-scalar continuation line) are out of scope and
/// never touched — see `check`'s HtmlEntity loop, which marks those
/// non-autofixable for the same reason.
pub(super) fn entity_fix_for_line(lines: &[String], i: usize) -> Option<String> {
    let line = lines.get(i)?;
    let cap = TOP_LEVEL.captures(line)?;
    let key = &cap[1];
    let value = &cap[2];
    if !ENTITY.is_match(value) {
        return None;
    }
    if value.starts_with('[') || value.starts_with('{') {
        return None;
    }
    let inner = if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        value[1..value.len() - 1].replace("''", "'")
    } else if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        unescape_double(&value[1..value.len() - 1])
    } else {
        value.to_string()
    };
    let (decoded, changed) = decode_fixed_point(&inner)?;
    if !changed {
        return None;
    }
    let quoted = if needs_quoting(&decoded) { yaml_double_quote(&decoded) } else { decoded };
    let candidate_line = format!("{key}: {quoted}");

    let mut candidate_lines = lines.to_vec();
    candidate_lines[i] = candidate_line.clone();
    round_trip_safe(lines, &candidate_lines).then_some(candidate_line)
}

pub(super) fn check(frontmatter: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(frontmatter) {
        issues.push(Issue::new(Kind::InvalidFrontmatter, Severity::Error, e.to_string(), false));
    }
    let lines: Vec<&str> = frontmatter.lines().collect();
    let survivors = surviving_keys(&lines);
    for (i, line) in lines.iter().enumerate() {
        let Some(cap) = TOP_LEVEL.captures(line) else { continue };
        let (key, value) = (&cap[1], &cap[2]);
        if PLACEHOLDERS.contains(&value) && !has_children(&lines, i) {
            issues.push(Issue::new(Kind::EmptyMetadata, Severity::Warning, *line, true));
        }
        if let Some((_, to)) = KEY_RENAMES.iter().find(|(from, _)| *from == key) {
            // A rename that would collide with an existing key is left
            // alone by `apply_structural_fixes` — mirror that here so
            // `autofixable` matches what `autofix` actually does.
            let collides = survivors.contains(to);
            issues.push(Issue::new(Kind::MetadataKeyDrift, Severity::Warning, *line, !collides));
        }
    }

    // HtmlEntity: see `entity_fix_for_line` for the exact scope. Every
    // occurrence on a line shares that line's fixability, since a fix (when
    // one applies) rewrites the whole value in one shot.
    let owned_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    for (i, line) in lines.iter().enumerate() {
        let value_start = TOP_LEVEL.captures(line).map(|cap| cap.get(2).unwrap().start());
        for m in ENTITY.find_iter(line) {
            let in_scope = value_start.is_some_and(|start| m.start() >= start);
            let autofixable = in_scope && entity_fix_for_line(&owned_lines, i).is_some();
            issues.push(Issue::new(Kind::HtmlEntity, Severity::Warning, m.as_str(), autofixable));
        }
    }
    issues
}
