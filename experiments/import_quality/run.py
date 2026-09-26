"""Run pipelines A2, A3, A4, A5, B0, B1 over data/inputs.jsonl -> results/<pipeline>.jsonl.

A2: A0 draft -> autofix -> Haiku judge -> Haiku whole-body repair (prompt v1, the baseline).
A5: A2 with the improved whole-body repair prompt (prompts/repair_prompt.txt).
A4: A0 draft -> autofix -> Haiku judge -> Haiku *edit-based* repair (find/replace tool call)
    -> apply edits -> autofix -> guards (score, paragraph count, ingredient names).
A6: A4 with edit-level rejection: each edit is checked on its own against the text before it
    (no "no recipe", paragraph count unchanged, no ingredient name lost except accepted renames);
    the surviving edits are kept, then autofix and the checker-score guard. Draft shown unnumbered
    (prompts/repair_edits_prompt_v2.txt).
A7: A6, then one more edit round (same prompt v2, same per-edit checks, then autofix and the
    score guard) only when body checker issues remain after A6's final autofix. The problem
    list is the remaining checker issues only — no second judge call.
A8: A7, but the Haiku judge sees the FULL document (frontmatter + body) — A2..A7 showed it only
    the body, so eval_prompt.txt's "missing frontmatter/title" rule fired on ~every recipe. The
    repair still edits the body only, so judge issues about metadata (METADATA_ISSUE keywords)
    are dropped from the problem list, and no repair call is made when nothing fixable remains.
"""
import argparse, asyncio, collections, json, re

from common import (CONVERT_PROMPT, EVAL_PROMPT, HAIKU, REPAIR_EDITS_PROMPT, REPAIR_EDITS_PROMPT_V2,
                    REPAIR_PROMPT,
                    REPAIR_PROMPT_V1, RESULTS, SONNET, call, clean_output, join_frontmatter,
                    lint_many, load_inputs, parse_json, split_frontmatter)

RULES = CONVERT_PROMPT.split("Below are the Cooklang syntax rules you must follow:", 1)[1] \
    .split("Now convert the recipe above", 1)[0].strip()


def issue_lines(lint_issues, judge, judge_issues=None):
    lines = [f"- [{i['kind']}] {i['evidence']}" for i in lint_issues]
    if judge_issues is not None:
        lines += [f"- {s}" for s in judge_issues]
    elif judge:
        lines += [f"- {s}" for s in judge.get("issues", [])]
    return "\n".join(lines)


# Judge issues about the frontmatter/metadata; a body-only edit repair cannot fix them.
# "tags" and "source" count only as metadata keys: the judge also calls @ingredient tokens "tags"
# and says "the source" for the original text.
METADATA_ISSUE = re.compile(r"\b(title|frontmatter|front matter|front-matter|metadata|servings|prep time|"
                            r"cook time|image|description|yield|nutrition)\b|\b(tags|source)\s*(:|url|field|key|link|list)"
                            r"|['\"`](tags|source)['\"`]", re.I)


def body_judge_issues(verdict):
    """(kept, filtered) judge issues: `filtered` mention a metadata keyword."""
    issues = [s for s in (verdict or {}).get("issues", []) if isinstance(s, str)]
    return [s for s in issues if not METADATA_ISSUE.search(s)], [s for s in issues if METADATA_ISSUE.search(s)]


async def judge(original, body):
    r = await call(HAIKU, EVAL_PROMPT.replace("{{ORIGINAL}}", original).replace("{{COOKLANG}}", body),
                   max_tokens=2000, temperature=0)
    return parse_json(r["text"]), r


async def repair(model, original, body, problems, template=REPAIR_PROMPT_V1):
    prompt = (template.replace("{{ORIGINAL}}", original).replace("{{DRAFT}}", body)
              .replace("{{PROBLEMS}}", problems).replace("{{RULES}}", RULES))
    extra = {"temperature": 0} if model == HAIKU else {}
    r = await call(model, prompt, max_tokens=16000, **extra)
    return clean_output(r["text"]), r


async def convert_haiku(original, language):
    prompt = CONVERT_PROMPT.replace("{{RECIPE}}", original) \
        .replace("{{LANGUAGE}}", language or "the original language")
    r = await call(HAIKU, prompt, max_tokens=8000, temperature=0.2)
    return clean_output(r["text"]), r


# ---------------------------------------------------------------- A4: edit-based repair

EDITS_TOOL = {
    "name": "apply_edits",
    "description": "Apply exact find/replace edits to the Cooklang draft body, in order. Each "
                   "'find' must be copied verbatim from the draft and occur exactly once.",
    "input_schema": {
        "type": "object",
        "properties": {"edits": {"type": "array", "items": {
            "type": "object",
            "properties": {"find": {"type": "string"}, "replace": {"type": "string"}},
            "required": ["find", "replace"], "additionalProperties": False}}},
        "required": ["edits"], "additionalProperties": False},
    "strict": True,
}
EDITS_TOOL_LOOSE = {k: v for k, v in EDITS_TOOL.items() if k != "strict"}
# Checker kinds that live in the frontmatter; edits only touch the body.
FRONTMATTER_KINDS = {"html_entity", "empty_metadata", "metadata_key_drift", "invalid_frontmatter"}
BLANK_LINE = re.compile(r"\n[ \t]*\n")
# Braced ingredient `@name{...}` (multi-word) and bare single-word `@name`.
ING_BRACED = re.compile(r"@[?&+\-]*([^\s@#~{}][^@#~{}\n]*?)\{[^}\n]*\}")
ING_BARE = re.compile(r"@[?&+\-]*([\w][\w\-']*)")


def paragraphs(body):
    return [p for p in BLANK_LINE.split(body.strip()) if p.strip()]


def ingredient_names(body):
    names = {m.group(1).strip().lower() for m in ING_BRACED.finditer(body)}
    rest = ING_BRACED.sub(" ", body)
    names |= {m.group(1).strip().lower() for m in ING_BARE.finditer(rest)}
    return names


def numbered(body):
    return "\n\n".join(f"[P{i}] {p}" for i, p in enumerate(paragraphs(body), 1))


def apply_edits(body, edits):
    """Applies edits in order. Returns (new_body, applied, rejected[{reason, find}])."""
    applied, rejected = [], []
    for e in edits:
        find, repl = e.get("find", ""), e.get("replace", "")
        if not find:
            reason = "empty_find"
        elif not repl.strip() and BLANK_LINE.search(find):
            reason = "removes_paragraph"
        elif find == repl:
            reason = "no_op"
        else:
            n = body.count(find)
            reason = None if n == 1 else ("not_found" if n == 0 else "not_unique")
        if reason:
            rejected.append({"reason": reason, "find": find[:80]})
            continue
        body = body.replace(find, repl, 1)
        applied.append(e)
    return body, applied, rejected


def renamed(edits, issues):
    """Old names an edit explicitly renamed (@old{ -> @new{) that SpellingVariant or
    LanguageMismatch evidence mentions. Conservative: one @old{ and one @new{ per edit."""
    evidence = " ".join(i["evidence"].lower() for i in issues
                        if i["kind"] in ("spelling_variant", "language_mismatch"))
    out = set()
    for e in edits:
        old = [m.group(1).strip().lower() for m in ING_BRACED.finditer(e["find"])]
        new = [m.group(1).strip().lower() for m in ING_BRACED.finditer(e["replace"])]
        if len(old) == 1 and len(new) == 1 and old[0] != new[0] and old[0] in evidence:
            out.add(old[0])
    return out


async def repair_edits(original, body, problems, template=REPAIR_EDITS_PROMPT, number=True):
    prompt = (template.replace("{{ORIGINAL}}", original).replace("{{DRAFT}}", numbered(body) if number else body)
              .replace("{{PROBLEMS}}", problems).replace("{{RULES}}", RULES))
    choice = {"type": "tool", "name": "apply_edits"}
    r = await call(HAIKU, prompt, max_tokens=4000, temperature=0, tools=[EDITS_TOOL], tool_choice=choice)
    fallback = None
    if r.get("stop_reason") == "max_tokens" or "edits" not in (r.get("tool_input") or {}):
        # Strict (grammar-constrained) decoding occasionally degenerates and burns the
        # whole budget with an empty input; retry once unconstrained. Both calls are billed.
        fallback = r
        r = await call(HAIKU, prompt, max_tokens=4000, temperature=0, tools=[EDITS_TOOL_LOOSE],
                       tool_choice=choice)
    edits = (r.get("tool_input") or {}).get("edits") or []
    return edits, r, fallback


async def run_a4(row):
    calls, added_ms = [], 0
    fixed = lint_many([row], fix=True)[row["udid"]]
    draft, draft_issues, draft_score = fixed["fixed"], fixed["fixed_issues"], fixed["fixed_score"]
    fm, draft_body = split_frontmatter(draft)
    verdict, jr = await judge(row["original_text"], draft_body)
    calls.append(("judge", jr))
    added_ms += jr["latency_ms"]
    needs_repair = bool(draft_issues) or not verdict or verdict.get("verdict") != "good" \
        or verdict.get("score", 0) < 5
    rec = {"repaired": needs_repair, "edits": [], "edits_applied": 0, "edits_rejected": 0,
           "reject_reasons": [], "guard_failed": [], "repair_kept": False,
           "draft_score": draft_score, "final_score": draft_score}
    final = draft
    if needs_repair:
        body_issues = [i for i in draft_issues if i["kind"] not in FRONTMATTER_KINDS]
        edits, rr, fallback = await repair_edits(row["original_text"], draft_body,
                                                 issue_lines(body_issues, verdict))
        if fallback:
            calls.append(("repair_strict_failed", fallback))
            added_ms += fallback["latency_ms"]
        calls.append(("repair", rr))
        added_ms += rr["latency_ms"]
        rec["strict_fallback"] = bool(fallback)
        new_body, applied, rejected = apply_edits(draft_body, edits)
        rec.update(edits=edits, edits_applied=len(applied), edits_rejected=len(rejected),
                   reject_reasons=rejected, stop_reason=rr.get("stop_reason"))
        if not applied:
            rec["guard_failed"] = ["no_edits_applied"]
        else:
            cand = lint_many([{**row, "cook": join_frontmatter(fm, new_body)}], fix=True)[row["udid"]]
            cand_body = split_frontmatter(cand["fixed"])[1]
            failed = []
            if cand["fixed_score"] > draft_score:
                failed.append("score")
            if len(paragraphs(cand_body)) != len(paragraphs(draft_body)):
                failed.append("paragraphs")
            lost = ingredient_names(draft_body) - ingredient_names(cand_body) - renamed(applied, draft_issues)
            if lost:
                failed.append("ingredients")
                rec["lost_ingredients"] = sorted(lost)
            rec["guard_failed"] = failed
            rec["candidate_score"] = cand["fixed_score"]
            if not failed:
                final = cand["fixed"]
                rec.update(repair_kept=True, final_score=cand["fixed_score"])
    return {
        "udid": row["udid"], "pipeline": "A4", "cook": final, **rec,
        "calls": [{"step": s, "latency_ms": r["latency_ms"], "cost": r["cost"], "usage": r["usage"],
                   "cached": r.get("cached", False)} for s, r in calls],
        "latency_ms": added_ms, "cost": sum(r["cost"] for _, r in calls),
    }


# ---------------------------------------------------------------- A6: edit-level rejection

# A braced ingredient plus its optional (note).
ING_TOKEN = re.compile(r"@[?&+\-]*([^\s@#~{}][^@#~{}\n]*?)\{[^}\n]*\}(?:\s*\(([^)\n]*)\))?")


def subset_rename(old, text):
    """`old` was renamed to a proper word-subset of itself and the removed words sit in that
    token's note in `text`: @shredded mozzarella{} -> @mozzarella{}(shredded)."""
    ow = collections.Counter(old.lower().split())
    for m in ING_TOKEN.finditer(text):
        nw = collections.Counter(m.group(1).strip().lower().split())
        if not nw or nw - ow or sum(nw.values()) >= sum(ow.values()):
            continue
        note = (m.group(2) or "").lower()
        if all(w in note for w in (ow - nw).elements()):
            return True
    return False


def check_edit(text, edit, issues):
    """Returns (new_text, None) if the edit is acceptable on its own, else (text, reason)."""
    find, repl = edit.get("find", ""), edit.get("replace", "")
    if not find:
        return text, "empty_find"
    if find == repl:
        return text, "no_op"
    if find.split() == repl.split():
        return text, "whitespace_only"
    n = text.count(find)
    if n != 1:
        return text, "not_found" if n == 0 else "not_unique"
    if "no recipe" in repl.lower():
        return text, "no_recipe"
    if not repl.strip() and BLANK_LINE.search(find):
        return text, "removes_paragraph"
    new = text.replace(find, repl, 1)
    if len(paragraphs(new)) != len(paragraphs(text)):
        return text, "paragraphs"
    ok_renames = renamed([edit], issues)
    lost = {x for x in ingredient_names(text) - ingredient_names(new)
            if x not in ok_renames and not subset_rename(x, repl)}
    if lost:
        return text, "ingredients"
    return new, None


async def run_a6(row, full_doc=False):
    """full_doc=False is A6/A7 as run (judge sees the body only). full_doc=True is A8: the judge
    sees the whole document, metadata judge issues are filtered out of the problem list, and the
    repair is skipped when no body problem remains."""
    calls, added_ms = [], 0
    fixed = lint_many([row], fix=True)[row["udid"]]
    draft, draft_issues, draft_score = fixed["fixed"], fixed["fixed_issues"], fixed["fixed_score"]
    fm, draft_body = split_frontmatter(draft)
    verdict, jr = await judge(row["original_text"], draft if full_doc else draft_body)
    calls.append(("judge", jr))
    added_ms += jr["latency_ms"]
    needs_repair = bool(draft_issues) or not verdict or verdict.get("verdict") != "good" \
        or verdict.get("score", 0) < 5
    body_issues = [i for i in draft_issues if i["kind"] not in FRONTMATTER_KINDS]
    rec = {}
    judge_kept = None
    if full_doc:
        judge_kept, judge_filtered = body_judge_issues(verdict)
        rec = {"verdict": verdict, "needs_repair_verdict": needs_repair,
               "judge_issues_kept": len(judge_kept), "judge_issues_filtered": judge_filtered}
        # Nothing a body edit can fix: skip the repair call.
        needs_repair = needs_repair and bool(body_issues or judge_kept)
    rec.update({"repaired": needs_repair, "edits": [], "edits_applied": 0, "edits_rejected": 0,
                "reject_reasons": [], "guard_failed": [], "repair_kept": False,
                "draft_score": draft_score, "final_score": draft_score})
    final = draft
    if needs_repair:
        edits, rr, fallback = await repair_edits(row["original_text"], draft_body,
                                                 issue_lines(body_issues, verdict, judge_kept),
                                                 template=REPAIR_EDITS_PROMPT_V2, number=False)
        if fallback:
            calls.append(("repair_strict_failed", fallback))
            added_ms += fallback["latency_ms"]
        calls.append(("repair", rr))
        added_ms += rr["latency_ms"]
        body, applied, rejected = draft_body, [], []
        for e in edits:
            body, reason = check_edit(body, e, draft_issues)
            if reason:
                rejected.append({"reason": reason, "find": e.get("find", "")[:80]})
            else:
                applied.append(e)
        rec.update(edits=edits, applied=applied, edits_applied=len(applied), edits_rejected=len(rejected),
                   reject_reasons=rejected, strict_fallback=bool(fallback))
        if not applied:
            rec["guard_failed"] = ["no_edits_applied"]
        else:
            cand = lint_many([{**row, "cook": join_frontmatter(fm, body)}], fix=True)[row["udid"]]
            rec["candidate_score"] = cand["fixed_score"]
            if cand["fixed_score"] > draft_score:
                rec["guard_failed"] = ["score"]
            else:
                final = cand["fixed"]
                rec.update(repair_kept=True, final_score=cand["fixed_score"])
    return {
        "udid": row["udid"], "pipeline": "A6", "cook": final, **rec,
        "calls": [{"step": s, "latency_ms": r["latency_ms"], "cost": r["cost"], "usage": r["usage"],
                   "cached": r.get("cached", False)} for s, r in calls],
        "latency_ms": added_ms, "cost": sum(r["cost"] for _, r in calls),
    }


async def run_a7(row, full_doc=False):
    rec = await run_a6(row, full_doc)
    cur = lint_many([{**row, "cook": rec["cook"]}], fix=True)[row["udid"]]
    fm, body = split_frontmatter(cur["fixed"])
    issues = [i for i in cur["fixed_issues"] if i["kind"] not in FRONTMATTER_KINDS]
    r2 = {"round2": bool(issues), "round2_edits": [], "round2_applied": 0, "round2_rejected": [],
          "round2_kept": False, "round2_latency_ms": 0, "round2_score_before": cur["fixed_score"]}
    final, calls = cur["fixed"], list(rec["calls"])
    if issues:
        edits, rr, fallback = await repair_edits(row["original_text"], body, issue_lines(issues, None),
                                                 template=REPAIR_EDITS_PROMPT_V2, number=False)
        for step, r in ((("repair2_strict_failed", fallback),) if fallback else ()) + (("repair2", rr),):
            calls.append({"step": step, "latency_ms": r["latency_ms"], "cost": r["cost"],
                          "usage": r["usage"], "cached": r.get("cached", False)})
            r2["round2_latency_ms"] += r["latency_ms"]
        new_body, applied, rejected = body, [], []
        for e in edits:
            new_body, reason = check_edit(new_body, e, cur["fixed_issues"])
            if reason:
                rejected.append({"reason": reason, "find": e.get("find", "")[:80]})
            else:
                applied.append(e)
        r2.update(round2_edits=edits, round2_applied=len(applied), round2_rejected=rejected)
        if applied:
            cand = lint_many([{**row, "cook": join_frontmatter(fm, new_body)}], fix=True)[row["udid"]]
            r2["round2_candidate_score"] = cand["fixed_score"]
            if cand["fixed_score"] <= cur["fixed_score"]:
                final = cand["fixed"]
                r2["round2_kept"] = True
    return {**rec, **r2, "pipeline": "A8" if full_doc else "A7", "cook": final, "calls": calls,
            "latency_ms": rec["latency_ms"] + r2["round2_latency_ms"],
            "cost": sum(c["cost"] for c in calls)}


async def run_one(row, pipeline, baseline_lint):
    """Returns the per-recipe record for one pipeline."""
    if pipeline == "A4":
        return await run_a4(row)
    if pipeline == "A6":
        return await run_a6(row)
    if pipeline == "A7":
        return await run_a7(row)
    if pipeline == "A8":
        return await run_a7(row, full_doc=True)
    fm, a0_body = split_frontmatter(row["cook"])
    calls, added_ms = [], 0
    if pipeline in ("B0", "B1"):
        body, r = await convert_haiku(row["original_text"], baseline_lint["language"])
        calls.append(("convert", r))
        draft = join_frontmatter(fm, body)
    else:
        draft = row["cook"]
    if pipeline == "B0":
        final = draft
    else:
        # A2/A3/B1: autofix (code) ∥ judge (Haiku), then optional repair.
        fixed = lint_many([{**row, "cook": draft}], fix=True)[row["udid"]]
        draft, draft_issues, draft_score = fixed["fixed"], fixed["fixed_issues"], fixed["fixed_score"]
        _, draft_body = split_frontmatter(draft)
        verdict, jr = await judge(row["original_text"], draft_body)
        calls.append(("judge", jr))
        added_ms += jr["latency_ms"]
        needs_repair = draft_issues or not verdict or verdict.get("verdict") != "good" or verdict.get("score", 0) < 5
        final = draft
        if needs_repair:
            model = SONNET if pipeline == "A3" else HAIKU
            template = REPAIR_PROMPT if pipeline == "A5" else REPAIR_PROMPT_V1
            body, rr = await repair(model, row["original_text"], draft_body,
                                    issue_lines(draft_issues, verdict), template)
            calls.append(("repair", rr))
            added_ms += rr["latency_ms"]
            candidate = join_frontmatter(split_frontmatter(draft)[0], body)
            cand = lint_many([{**row, "cook": candidate}])[row["udid"]]
            # Keep the repair unless the checker says it made things worse.
            if cand["score"] <= draft_score:
                final = candidate
    if pipeline in ("B0", "B1"):
        added_ms += calls[0][1]["latency_ms"]  # converter latency; compared to ft latency in report
    return {
        "udid": row["udid"], "pipeline": pipeline, "cook": final,
        "calls": [{"step": s, "latency_ms": r["latency_ms"], "cost": r["cost"], "usage": r["usage"],
                   "cached": r.get("cached", False)} for s, r in calls],
        "latency_ms": added_ms, "cost": sum(r["cost"] for _, r in calls),
    }


async def main(pipelines, limit):
    rows = load_inputs()[:limit] if limit else load_inputs()
    baseline = lint_many(rows)
    for p in pipelines:
        out = RESULTS / f"{p}.jsonl"
        done = {json.loads(l)["udid"] for l in open(out)} if out.exists() else set()
        todo = [r for r in rows if r["udid"] not in done]
        print(f"{p}: {len(todo)} to run ({len(done)} done)")
        with open(out, "a") as f:
            for i in range(0, len(todo), 40):
                chunk = todo[i:i + 40]
                recs = await asyncio.gather(*(run_one(r, p, baseline[r["udid"]]) for r in chunk), return_exceptions=True)
                for r, rec in zip(chunk, recs):
                    if isinstance(rec, Exception):
                        print(f"  {r['udid']}: {rec}")
                        continue
                    f.write(json.dumps(rec) + "\n")
                f.flush()
                print(f"  {min(i + 40, len(todo))}/{len(todo)}")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--pipelines", default="A2,A3,B0,B1")
    ap.add_argument("--limit", type=int, default=0)
    a = ap.parse_args()
    asyncio.run(main(a.pipelines.split(","), a.limit))
