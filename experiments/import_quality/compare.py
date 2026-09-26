"""Compare A0, A1 and every finished LLM pipeline (A2, A4, A5, A6, A7, A8): re-lint every final output
with the current binary, plus repair/edit/guard stats and drift vs the A1 draft. Prints a
markdown table.

"clean & no drift": checker-clean, same paragraph count as A1, and no A1 ingredient name lost
except accepted renames (named in A1's spelling_variant/language_mismatch evidence, or a
word-subset rename with the removed words in the note)."""
import collections, json, statistics

from common import RESULTS, lint_many, load_inputs, split_frontmatter
from run import ingredient_names, paragraphs, subset_rename


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "–"


def q(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))] if xs else 0


def load(name):
    path = RESULTS / f"{name}.jsonl"
    return {r["udid"]: r for r in map(json.loads, open(path))} if path.exists() else None


def drift(a1_body, body, a1_issues):
    lost = ingredient_names(a1_body) - ingredient_names(body)
    evidence = " ".join(i["evidence"].lower() for i in a1_issues
                        if i["kind"] in ("spelling_variant", "language_mismatch"))
    lost_unaccepted = {x for x in lost if x not in evidence and not subset_rename(x, body)}
    return (len(paragraphs(body)) != len(paragraphs(a1_body)), bool(lost),
            len(body.strip()) < 0.8 * len(a1_body.strip()), bool(lost_unaccepted))


def new_spend(p, rec):
    """$ actually spent by this pipeline: uncached calls. Older records lack the flag: A2 ran
    first (all calls new); A4 reused A2's judge calls (only repairs new)."""
    calls = rec["calls"]
    if calls and "cached" in calls[0]:
        return sum(c["cost"] for c in calls if not c["cached"])
    return sum(c["cost"] for c in calls if p == "A2" or c["step"].startswith("repair"))


def main():
    rows = {r["udid"]: r for r in load_inputs()}
    a1 = lint_many(list(rows.values()), fix=True)  # current binary: A0 issues + A1 fixed
    finals = {"A0": {u: r["cook"] for u, r in rows.items()},
              "A1": {u: r["fixed"] for u, r in a1.items()}}
    recs = {}
    for p in ("A2", "A4", "A5", "A6", "A7", "A8", "A9"):
        d = load(p)
        if d and len(d) == len(rows):
            finals[p], recs[p] = {u: r["cook"] for u, r in d.items()}, d
    n = len(rows)
    table, extra = [], []
    for p, docs in finals.items():
        lint = lint_many([{**rows[u], "cook": c} for u, c in docs.items()])
        clean = sum(1 for r in lint.values() if not r["issues"])
        kinds = collections.Counter(k for r in lint.values() for k in {i["kind"] for i in r["issues"]})
        top = ", ".join(f"{k} {pct(v, n)}" for k, v in kinds.most_common(6))
        row = {"p": p, "clean": pct(clean, n), "top": top, "uc": pct(kinds["under_count"], n)}
        a1_body = {u: split_frontmatter(a1[u]["fixed"])[1] for u in rows}
        d = {u: drift(a1_body[u], split_frontmatter(docs[u])[1], a1[u]["fixed_issues"]) for u in rows}
        row["cnd"] = pct(sum(1 for u in rows if not lint[u]["issues"] and not d[u][0] and not d[u][3]), n)
        if p in recs:
            R = recs[p]
            if p in ("A4", "A6", "A7", "A8", "A9"):
                trig = [u for u, r in R.items() if r["repaired"]]
                kept = [u for u, r in R.items() if r["repair_kept"]]
            else:
                trig = [u for u, r in R.items() if any(c["step"] == "repair" for c in r["calls"])]
                kept = [u for u in trig if R[u]["cook"] != a1[u]["fixed"]]
            d = list(d.values())
            out_tok = [sum(c["usage"]["output_tokens"] for c in R[u]["calls"] if c["step"].startswith("repair"))
                       for u in trig]
            lat = [r["latency_ms"] / 1000 for r in R.values()]
            row.update(trig=pct(len(trig), n), kept=f"{pct(len(kept), n)} ({pct(len(kept), len(trig))} of triggered)",
                       para=pct(sum(x[0] for x in d), n), lost=pct(sum(x[1] for x in d), n),
                       short=pct(sum(x[2] for x in d), n), tok=int(statistics.median(out_tok)) if out_tok else 0,
                       lat=f"{q(lat, .5):.1f} / {q(lat, .9):.1f} s",
                       cost=f"${sum(r['cost'] for r in R.values()) / n:.4f}",
                       new=f"${sum(new_spend(p, r) for r in R.values()):.2f}")
            if p in ("A7", "A8", "A9"):
                r2 = [r for r in R.values() if r["round2"]]
                lat2 = [r["round2_latency_ms"] / 1000 for r in r2]
                spend2 = sum(c["cost"] for r in r2 for c in r["calls"]
                             if c["step"].startswith("repair2") and not c["cached"])
                extra += [f"{p} round 2: {len(r2)} recipes ({pct(len(r2), n)}), kept {sum(r['round2_kept'] for r in r2)}, "
                          f"edits applied {sum(r['round2_applied'] for r in r2)}, rejected "
                          f"{dict(collections.Counter(x['reason'] for r in r2 for x in r['round2_rejected']))}, "
                          f"added latency p50/p90 {q(lat2, .5):.1f} / {q(lat2, .9):.1f} s, round-2 new $ {spend2:.2f}"]
            if p in ("A8", "A9"):
                vt = sum(r["needs_repair_verdict"] for r in R.values())
                filt = sum(len(r["judge_issues_filtered"]) for r in R.values())
                kept_j = sum(r["judge_issues_kept"] for r in R.values())
                only_md = sum(r["needs_repair_verdict"] and not r["repaired"] for r in R.values())
                j5 = sum(bool(r["verdict"]) and r["verdict"].get("verdict") == "good" and r["verdict"].get("score") == 5
                         for r in R.values())
                if p == "A9":
                    extra += [f"A9 no_method (ingredient-list-only originals, not judged or repaired): "
                              f"{sum(r['no_method'] for r in R.values())}"]
                extra += [f"{p} judge (full doc{' + metadata' if p == 'A9' else ''}): good+5 on {pct(j5, n)}; verdict-or-checker trigger {pct(vt, n)}; "
                          f"repair skipped (only metadata issues left) on {only_md}; judge issues filtered as "
                          f"metadata {filt} of {filt + kept_j} ({pct(filt, filt + kept_j)})"]
            if p in ("A4", "A6", "A7", "A8", "A9"):
                app = sum(r["edits_applied"] for r in R.values())
                rej = sum(r["edits_rejected"] for r in R.values())
                reasons = collections.Counter(x["reason"] for r in R.values() for x in r["reject_reasons"])
                guards = collections.Counter(g for r in R.values() for g in r["guard_failed"])
                repair_cost = sum(c["cost"] for r in R.values() for c in r["calls"] if c["step"].startswith("repair"))
                extra += [f"{p} edits: {app} applied, {rej} rejected ({pct(rej, app + rej)}): {dict(reasons)}",
                          f"{p} guard failures (recipes, of {len(trig)} triggered): {dict(guards)}",
                          f"{p} strict fallbacks: {sum(1 for r in R.values() if r.get('strict_fallback'))}",
                          f"{p} repair-only $/recipe: ${repair_cost / n:.4f} (total ${repair_cost:.2f})"]
        table.append(row)
    cols = [("p", "pipeline"), ("clean", "checker-clean"), ("uc", "under_count"), ("cnd", "clean & no drift"), ("trig", "repair trigger"), ("kept", "repair kept"),
            ("para", "para count changed"), ("lost", "lost ingr. name"), ("short", "<80% length"),
            ("tok", "repair out tok p50"), ("lat", "added latency p50/p90"), ("cost", "$/recipe (all calls)"), ("new", "new $ spent")]
    print("| " + " | ".join(h for _, h in cols) + " |")
    print("|" + "---|" * len(cols))
    for r in table:
        print("| " + " | ".join(str(r.get(k, "–")) for k, _ in cols) + " |")
    print()
    for r in table:
        print(f"- {r['p']} top residual kinds (per recipe): {r['top']}")
    for e in extra:
        print("- " + e)


if __name__ == "__main__":
    main()
