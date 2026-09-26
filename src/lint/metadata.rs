use super::{Issue, Kind, Severity};
use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

pub(super) static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&(?:[a-zA-Z]{2,8}|#\d{2,5});").unwrap());
/// Top-level `key: value` line.
pub(super) static TOP_LEVEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_ ]*):\s*(.*?)\s*$").unwrap());
/// Indented `key: value` line inside a block mapping (space indentation
/// only — YAML forbids tabs there).
static NESTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^( +)([A-Za-z][A-Za-z0-9_ ]*):\s*(.*?)\s*$").unwrap());
/// A bare `key:` line (no value, no comment) that opens a block mapping.
static PARENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^( *)([A-Za-z][A-Za-z0-9_ ]*):\s*$").unwrap());
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
/// quote characters, and the comment marker. This is only a cheap first
/// filter — `is_bare_safe` empirically verifies the actual choice by parsing
/// the candidate, which is what catches a value that merely *looks* plain
/// but would parse back as a bool/int/float/null ("true", "4", "~").
const YAML_SPECIAL: &[char] =
    &[':', '#', '\'', '"', '[', ']', '{', '}', ',', '&', '*', '!', '|', '>', '%', '@'];
/// YAML 1.1 core-schema boolean spellings. `serde_yaml` itself only treats
/// `true`/`false` (YAML 1.2-ish) as booleans, so `is_bare_safe`'s own
/// round-trip check wouldn't catch these — but cook.md's Rails app reads
/// this frontmatter with Ruby's Psych, which *is* YAML 1.1 and turns a bare
/// `yes`/`no`/`on`/`off`/`y`/`n` (any case) into a real boolean there. Always
/// quote a decoded value that matches one of these, regardless of what our
/// own parser would do with it.
const YAML11_BOOL_WORDS: &[&str] = &["yes", "no", "on", "off", "y", "n"];
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
/// Exposed as `pub(super)` because `apply_structural_fixes` and
/// `entity_fix_for_line` reuse this: a key with children is either a nested
/// mapping/list (structural — the placeholder-deletion case) or a plain
/// scalar folded onto the next line(s) (the entity-fix case, where a
/// single-line value extraction would silently drop the continuation).
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
        || YAML11_BOOL_WORDS.iter().any(|w| s.eq_ignore_ascii_case(w))
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

/// Whether emitting `decoded` bare (unquoted) after `key: ` would parse back
/// to exactly the string `decoded` — checked empirically (by actually
/// parsing the candidate) rather than by trying to enumerate every YAML
/// plain-scalar edge case ourselves. This is what catches a decoded value
/// that merely *looks* safe by character content alone but would silently
/// change type: "true"/"false" (bool), "4"/"3.5" (number), "null"/"~"
/// (null) all parse back as something other than that same string.
fn is_bare_safe(key: &str, decoded: &str) -> bool {
    if needs_quoting(decoded) {
        return false;
    }
    let candidate = format!("{key}: {decoded}");
    let key_v = serde_yaml::Value::String(key.to_string());
    let expected = serde_yaml::Value::String(decoded.to_string());
    matches!(
        serde_yaml::from_str::<serde_yaml::Value>(&candidate),
        Ok(v) if v.as_mapping().and_then(|m| m.get(&key_v)) == Some(&expected)
    )
}

/// The YAML scalar to emit for `decoded`: bare when that's verified safe,
/// double-quoted (with full escaping) otherwise. Double-quoting is always
/// type-safe in YAML — a double-quoted scalar is never reinterpreted as
/// null/bool/int/float — so this never needs a fallback beyond it.
fn safe_scalar(key: &str, decoded: &str) -> String {
    if is_bare_safe(key, decoded) {
        decoded.to_string()
    } else {
        yaml_double_quote(decoded)
    }
}

/// Best-effort: a YAML comment starts at a `#` preceded by whitespace (or at
/// the very start of the scalar). Only used as a fallback for a bare value
/// that doesn't parse in isolation (see `extract_string_value`) — where a
/// real YAML parse can't strip the comment for us either, since the parse
/// itself is what's failing.
fn strip_trailing_comment(raw: &str) -> String {
    static COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|\s)#.*$").unwrap());
    match COMMENT.find(raw) {
        Some(m) => raw[..m.start()].trim_end().to_string(),
        None => raw.to_string(),
    }
}

/// Recovers the *true* value of a top-level line's raw (regex-captured)
/// value text — this is the fix for the root cause: a regex capturing
/// "everything after the colon" can't tell a quote from a comment from an
/// escape sequence, so it was corrupting `é`/`\x41` escapes, absorbing
/// a trailing `# comment` into the value, and mishandling nested quotes.
///
/// Parses `raw_value` as its own standalone YAML document — this is what
/// correctly resolves quoting, backslash/unicode/hex escapes, and comment
/// stripping, all via the real YAML grammar instead of hand-rolled regexes.
/// Returns `None` when that parses to anything other than a string (a flow
/// collection, a bool/int/float/null with no entity-driven ambiguity, etc.)
/// — out of scope for a value-level fix.
///
/// A *bare* scalar that starts with a YAML indicator character (`&`, `*`,
/// `!`, `?`, ...) doesn't parse even in isolation — YAML reads the leading
/// character as an anchor/alias/tag/etc. regardless of context. That's not
/// something entity-decoding caused (the raw, undecoded text already has
/// that shape), so falling back to the raw text (with a best-effort trailing
/// comment strip) is safe: nothing about the *value itself* was ambiguous,
/// only how to open it as a self-contained document.
fn extract_string_value(raw_value: &str) -> Option<String> {
    match serde_yaml::from_str::<serde_yaml::Value>(raw_value) {
        Ok(v) => v.as_str().map(|s| s.to_string()),
        Err(_) => Some(strip_trailing_comment(raw_value)),
    }
}

/// Whether replacing `lines[i]` with `candidate_lines[i]` (every other line
/// left byte-identical) is a safe rewrite:
///
/// 1. the candidate parses as YAML, and its mapping's `key` entry is
///    *exactly* the decoded string `expected` — a YAML `String`, never
///    null/bool/int/float via an accidental plain-scalar collision; and
/// 2. every other top-level key keeps the same value it parses to standalone
///    (guards against the changed line somehow altering how an unrelated,
///    byte-identical line is interpreted — a multi-document construct, an
///    anchor/alias, etc.).
fn round_trip_safe(lines: &[String], candidate_lines: &[String], key: &str, expected: &str) -> bool {
    let Ok(cand_val) = serde_yaml::from_str::<serde_yaml::Value>(&candidate_lines.join("\n")) else {
        return false;
    };
    let Some(cand_map) = cand_val.as_mapping() else { return false };
    let key_v = serde_yaml::Value::String(key.to_string());
    if cand_map.get(&key_v) != Some(&serde_yaml::Value::String(expected.to_string())) {
        return false;
    }
    for (j, orig_line) in lines.iter().enumerate() {
        if candidate_lines[j] != *orig_line {
            continue; // this is the line we changed; verified above
        }
        let Some(cap) = TOP_LEVEL.captures(orig_line) else { continue };
        let other_key = &cap[1];
        if other_key == key {
            continue;
        }
        let Ok(solo) = serde_yaml::from_str::<serde_yaml::Value>(orig_line) else { continue };
        let Some(solo_map) = solo.as_mapping() else { continue };
        let other_key_v = serde_yaml::Value::String(other_key.to_string());
        let Some(solo_val) = solo_map.get(&other_key_v) else { continue };
        if cand_map.get(&other_key_v) != Some(solo_val) {
            return false;
        }
    }
    true
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Whether the nested line `lines[i]` (indented `indent` spaces) has children
/// or a continuation: the next non-blank, non-comment line is indented deeper
/// (a nested mapping, or a plain scalar folded onto the next line), is a
/// sequence item at the same indent, or uses a tab.
fn nested_has_children(lines: &[&str], i: usize, indent: usize) -> bool {
    lines[i + 1..].iter().find(|l| !is_blank_or_comment(l)).is_some_and(|next| {
        let ind = indent_of(next);
        next[ind..].starts_with('\t') || ind > indent || (ind == indent && next[ind..].starts_with('-'))
    })
}

/// The key path of the nested line `lines[i]`: each ancestor must be a bare
/// `key:` line at a strictly smaller indent, up to column 0. `None` when any
/// ancestor is something else (a sequence item, a key with an inline value
/// or comment, ...) — only plain block mappings are in scope.
fn nested_path(lines: &[&str], i: usize) -> Option<Vec<String>> {
    let cap = NESTED.captures(lines[i])?;
    let mut path = vec![cap[2].to_string()];
    let mut cur = cap[1].len();
    for line in lines[..i].iter().rev() {
        if is_blank_or_comment(line) || indent_of(line) >= cur {
            continue;
        }
        let parent = PARENT.captures(line)?;
        path.push(parent[2].to_string());
        cur = parent[1].len();
        if cur == 0 {
            path.reverse();
            return Some(path);
        }
    }
    None
}

/// Whether `candidate` parses to exactly the tree `lines` parses to, except
/// that the string at `path` — which must have been exactly `original` — is
/// now `decoded`. Validates the *whole* frontmatter, so a rewrite can't
/// disturb any other key, however it's nested.
fn tree_safe(lines: &[String], candidate: &[String], path: &[String], original: &str, decoded: &str) -> bool {
    let (Ok(orig), Ok(cand)) = (
        serde_yaml::from_str::<serde_yaml::Value>(&lines.join("\n")),
        serde_yaml::from_str::<serde_yaml::Value>(&candidate.join("\n")),
    ) else {
        return false;
    };
    let mut want = orig;
    let mut node = &mut want;
    for key in path {
        let Some(child) = node.as_mapping_mut().and_then(|m| m.get_mut(key.as_str())) else {
            return false;
        };
        node = child;
    }
    if node.as_str() != Some(original) {
        return false;
    }
    *node = serde_yaml::Value::String(decoded.to_string());
    cand == want
}

/// The nested-mapping counterpart of the top-level entity fix: the same
/// isolated value parse, fixed-point decode and `safe_scalar` quoting, but
/// validated against the whole frontmatter tree (`tree_safe`). Sequences,
/// flow collections, block scalars and folded continuations stay out of
/// scope.
fn nested_entity_fix(lines: &[String], i: usize) -> Option<String> {
    let str_lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let cap = NESTED.captures(&lines[i])?;
    let (indent, key) = (&cap[1], &cap[2]);
    if nested_has_children(&str_lines, i, indent.len()) {
        return None;
    }
    let path = nested_path(&str_lines, i)?;
    let original_value = extract_string_value(&cap[3])?;
    if !ENTITY.is_match(&original_value) {
        return None;
    }
    let (decoded, changed) = decode_fixed_point(&original_value)?;
    if !changed {
        return None;
    }
    let candidate_line = format!("{indent}{key}: {}", safe_scalar(key, &decoded));
    let mut candidate_lines = lines.to_vec();
    candidate_lines[i] = candidate_line.clone();
    tree_safe(lines, &candidate_lines, &path, &original_value, &decoded).then_some(candidate_line)
}

/// Attempts a safe HTML-entity fix for `lines[i]`, returning the replacement
/// line when one exists. Scope is deliberately narrow — only a top-level
/// `key: value` line's value, or an indented one inside a plain block mapping
/// (see `nested_entity_fix`), is touched, and only when doing so is
/// YAML-safe:
///
/// - A line with children (see `has_children`) is skipped entirely: either a
///   nested mapping/list, or a plain scalar folded onto the next line(s) —
///   extracting "the value" from this line alone wouldn't be the true value.
/// - The true original value comes from `extract_string_value`, not a raw
///   text slice — see its docs for why. A flow collection or a non-string
///   scalar type falls out of scope there (not a `String`), no separate
///   check needed.
/// - The replacement is emitted via `safe_scalar`: bare only when
///   empirically verified safe, double-quoted otherwise.
/// - The final candidate is required to round-trip (`round_trip_safe`): if
///   decoding would break the YAML, change the key's value to the wrong
///   thing or the wrong type, or disturb another key, the line is left
///   completely untouched.
pub(super) fn entity_fix_for_line(lines: &[String], i: usize) -> Option<String> {
    if NESTED.is_match(lines.get(i)?) {
        return nested_entity_fix(lines, i);
    }
    let str_lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    if has_children(&str_lines, i) {
        return None;
    }
    let line = lines.get(i)?;
    let cap = TOP_LEVEL.captures(line)?;
    let key = &cap[1];
    let original_value = extract_string_value(&cap[2])?;
    if !ENTITY.is_match(&original_value) {
        return None;
    }
    let (decoded, changed) = decode_fixed_point(&original_value)?;
    if !changed {
        return None;
    }
    let candidate_line = format!("{key}: {}", safe_scalar(key, &decoded));

    let mut candidate_lines = lines.to_vec();
    candidate_lines[i] = candidate_line.clone();
    round_trip_safe(lines, &candidate_lines, key, &decoded).then_some(candidate_line)
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

    // HtmlEntity: fixability is decided on the *structurally-fixed* lines —
    // the same baseline `fix::fix_frontmatter` actually runs entity decoding
    // against (structural fixes first, entity decoding as a separate final
    // pass) — not the raw original. Otherwise e.g. a placeholder duplicate
    // (`title: ''` / `title: A &amp; B`) would be judged against a
    // still-duplicated document and wrongly called non-autofixable, when
    // autofix's structural pass removes the duplicate first and the entity
    // fix then succeeds cleanly.
    let structural = apply_structural_fixes(&lines);
    let orig_to_working: HashMap<usize, usize> =
        structural.iter().enumerate().map(|(wi, (oi, _))| (*oi, wi)).collect();
    let working: Vec<String> = structural.iter().map(|(_, l)| l.clone()).collect();

    for (i, line) in lines.iter().enumerate() {
        let value_start = TOP_LEVEL
            .captures(line)
            .map(|cap| cap.get(2).unwrap().start())
            .or_else(|| NESTED.captures(line).map(|cap| cap.get(3).unwrap().start()));
        for m in ENTITY.find_iter(line) {
            let in_scope = value_start.is_some_and(|start| m.start() >= start);
            let autofixable = in_scope
                && match orig_to_working.get(&i) {
                    Some(&wi) => entity_fix_for_line(&working, wi).is_some(),
                    // The line was deleted by the structural pass (an
                    // empty/placeholder value) — its entity vanishes with it.
                    None => true,
                };
            issues.push(Issue::new(Kind::HtmlEntity, Severity::Warning, m.as_str(), autofixable));
        }
    }
    issues
}
