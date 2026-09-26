"""Compare A0, A1, A2, A4 (and A5 if present): re-lint every final output with the current
binary, plus repair/edit/guard stats and drift vs the A1 draft. Prints a markdown table."""
import collections, json, statistics

from common import RESULTS, lint_many, load_inputs, split_frontmatter
from run import ingredient_names, paragraphs


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "–"


def q(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))] if xs else 0


def load(name):
    path = RESULTS / f"{name}.jsonl"
    return {r["udid"]: r for r in map(json.loads, open(path))} if path.exists() else None


def drift(a1_body, body):
    return (len(paragraphs(body)) != len(paragraphs(a1_body)),
            bool(ingredient_names(a1_body) - ingredient_names(body)),
            len(body.strip()) < 0.8 * len(a1_body.strip()))


def main():
    rows = {r["udid"]: r for r in load_inputs()}
    a1 = lint_many(list(rows.values()), fix=True)  # current binary: A0 issues + A1 fixed
    finals = {"A0": {u: r["cook"] for u, r in rows.items()},
              "A1": {u: r["fixed"] for u, r in a1.items()}}
    recs = {}
    for p in ("A2", "A4", "A5"):
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
        row = {"p": p, "clean": pct(clean, n), "top": top}
        a1_body = {u: split_frontmatter(a1[u]["fixed"])[1] for u in rows}
        if p in recs:
            R = recs[p]
            if p == "A4":
                trig = [u for u, r in R.items() if r["repaired"]]
                kept = [u for u, r in R.items() if r["repair_kept"]]
            else:
                trig = [u for u, r in R.items() if any(c["step"] == "repair" for c in r["calls"])]
                kept = [u for u in trig if R[u]["cook"] != a1[u]["fixed"]]
            d = [drift(a1_body[u], split_frontmatter(docs[u])[1]) for u in rows]
            out_tok = [sum(c["usage"]["output_tokens"] for c in R[u]["calls"] if c["step"].startswith("repair"))
                       for u in trig]
            lat = [r["latency_ms"] / 1000 for r in R.values()]
            row.update(trig=pct(len(trig), n), kept=f"{pct(len(kept), n)} ({pct(len(kept), len(trig))} of triggered)",
                       para=pct(sum(x[0] for x in d), n), lost=pct(sum(x[1] for x in d), n),
                       short=pct(sum(x[2] for x in d), n), tok=int(statistics.median(out_tok)) if out_tok else 0,
                       lat=f"{q(lat, .5):.1f} / {q(lat, .9):.1f} s",
                       cost=f"${sum(r['cost'] for r in R.values()) / n:.4f}")
            if p == "A4":
                app = sum(r["edits_applied"] for r in R.values())
                rej = sum(r["edits_rejected"] for r in R.values())
                reasons = collections.Counter(x["reason"] for r in R.values() for x in r["reject_reasons"])
                guards = collections.Counter(g for r in R.values() for g in r["guard_failed"])
                repair_cost = sum(c["cost"] for r in R.values() for c in r["calls"] if c["step"].startswith("repair"))
                extra += [f"A4 edits: {app} applied, {rej} rejected ({pct(rej, app + rej)}): {dict(reasons)}",
                          f"A4 guard failures (recipes, of {len(trig)} triggered): {dict(guards)}",
                          f"A4 strict fallbacks: {sum(1 for r in R.values() if r.get('strict_fallback'))}",
                          f"A4 repair-only $/recipe: ${repair_cost / n:.4f} (total ${repair_cost:.2f})"]
        table.append(row)
    cols = [("p", "pipeline"), ("clean", "checker-clean"), ("trig", "repair trigger"), ("kept", "repair kept"),
            ("para", "para count changed"), ("lost", "lost ingr. name"), ("short", "<80% length"),
            ("tok", "repair out tok p50"), ("lat", "added latency p50/p90"), ("cost", "$/recipe")]
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
