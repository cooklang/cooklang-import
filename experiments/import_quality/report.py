"""Join checker lint + Sonnet grades + run records.

Prints per-pipeline grader and CLEAN rates (also on the common graded subset), Haiku-judge vs
grader agreement on the A1 drafts, and writes (gitignored, contain recipe text):
  results/grader_issues_sample.jsonl  30 random grader issues per pipeline, for theme clustering
  results/calibration.jsonl           30 random A7 recipes where checker and grader disagree
  results/summary.md                  the aggregate tables only
"""
import asyncio, json, random

from common import RESULTS, lint_many, load_inputs, split_frontmatter
from compare import drift
from run import judge

PIPES = ["A1", "A6", "A7"]
inputs = {r["udid"]: r for r in load_inputs()}
grades = {}
for l in open(RESULTS / "grades.jsonl"):
    g = json.loads(l)
    grades[g["id"]] = g["grade"]
a1 = {json.loads(l)["udid"]: json.loads(l) for l in open(RESULTS / "a0_a1_lint.jsonl")}
finals = {"A1": {u: r["fixed"] for u, r in a1.items()}}
for p in PIPES[1:]:
    finals[p] = {json.loads(l)["udid"]: json.loads(l)["cook"] for l in open(RESULTS / f"{p}.jsonl")}


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "–"


def good5(g):
    return bool(g) and g.get("verdict") == "good" and g.get("score") == 5


def bad(g):
    return not g or g.get("verdict") != "good" or g.get("score", 0) < 5


def stats(p, udids):
    lint = lint_many([{**inputs[u], "cook": finals[p][u]} for u in udids])
    a1b = {u: split_frontmatter(a1[u]["fixed"])[1] for u in udids}
    row = {"n": len(udids), "chk": 0, "g5": 0, "good": 0, "clean": 0, "clean_nd": 0}
    for u in udids:
        g = grades[f"{p}:{u}"]
        d = drift(a1b[u], split_frontmatter(finals[p][u])[1], a1[u]["fixed_issues"])
        chk = not lint[u]["issues"]
        row["chk"] += chk
        row["g5"] += good5(g)
        row["good"] += bool(g) and g.get("verdict") == "good"
        row["clean"] += chk and good5(g)
        row["clean_nd"] += chk and good5(g) and not d[0] and not d[3]
    return row, lint


def table(title, rows):
    out = [f"### {title}", "", "| pipeline | n graded | checker-clean | grader good+5 | grader good (any) | "
           "**CLEAN** | CLEAN & no drift |", "|---|---|---|---|---|---|---|"]
    for p, r in rows.items():
        n = r["n"]
        out.append(f"| {p} | {n} | {pct(r['chk'], n)} | {pct(r['g5'], n)} | {pct(r['good'], n)} | "
                   f"**{pct(r['clean'], n)}** | {pct(r['clean_nd'], n)} |")
    return out


def main():
    graded = {p: [u for u in finals[p] if f"{p}:{u}" in grades]
              for p in PIPES}
    lines = ["# Import quality — graded results", ""]
    rows, lints = {}, {}
    for p in PIPES:
        if graded[p]:
            rows[p], lints[p] = stats(p, graded[p])
    lines += table("All graded recipes per pipeline", rows) + [""]
    common = set.intersection(*(set(v) for v in graded.values() if v))
    if common:
        lines += table(f"Common subset graded in every pipeline (n={len(common)})",
                       {p: stats(p, sorted(common))[0] for p in PIPES if graded[p]}) + [""]

    # Haiku judge (as run in A6/A7, on the A1 draft body; cached) vs Sonnet grader on A1.
    async def verdicts(us):
        return await asyncio.gather(*(judge(inputs[u]["original_text"], split_frontmatter(a1[u]["fixed"])[1])
                                      for u in us))
    us = graded["A1"]
    js = asyncio.run(verdicts(us))
    cnt = {"tp": 0, "fp": 0, "fn": 0, "tn": 0}
    cnt_t = dict(cnt)
    for u, (v, _) in zip(us, js):
        jb = bad(v)
        trig = jb or bool(a1[u]["fixed_issues"])
        gb = bad(grades[f"A1:{u}"])
        for c, flag in ((cnt, jb), (cnt_t, trig)):
            c[("t" if flag == gb else "f") + ("p" if flag else "n")] += 1
    for name, c in (("judge alone (bad or score<5)", cnt), ("A6 repair trigger (judge OR checker issue)", cnt_t)):
        lines += [f"- {name} vs grader bad-or-<5 on A1 (n={len(us)}): TP {c['tp']}, FP {c['fp']}, FN {c['fn']}, "
                  f"TN {c['tn']}; precision {pct(c['tp'], c['tp'] + c['fp'])}, recall {pct(c['tp'], c['tp'] + c['fn'])}"]
    spend = sum(json.loads(l).get("cost", 0) for l in open(RESULTS / "grades.jsonl"))
    lines += ["", f"Grader spend so far: ${spend:.2f}"]

    rng = random.Random(7)
    with open(RESULTS / "grader_issues_sample.jsonl", "w") as f:
        for p in PIPES:
            issues = [(u, s) for u in graded[p] for s in (grades[f"{p}:{u}"] or {}).get("issues", [])]
            for u, s in rng.sample(issues, min(30, len(issues))):
                f.write(json.dumps({"pipeline": p, "udid": u, "issue": s}, ensure_ascii=False) + "\n")
    dis = [u for u in graded["A7"] if (not lints["A7"][u]["issues"]) != good5(grades[f"A7:{u}"])]
    with open(RESULTS / "calibration.jsonl", "w") as f:
        for u in rng.sample(dis, min(30, len(dis))):
            f.write(json.dumps({"udid": u, "checker_clean": not lints["A7"][u]["issues"],
                                "checker_issues": lints["A7"][u]["issues"], "grader": grades[f"A7:{u}"],
                                "original": inputs[u]["original_text"], "cook": finals["A7"][u]},
                               ensure_ascii=False) + "\n")
    lines += [f"A7 checker/grader disagreements: {len(dis)} of {len(graded['A7'])}; "
              f"{min(30, len(dis))} sampled to results/calibration.jsonl"]
    (RESULTS / "summary.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
