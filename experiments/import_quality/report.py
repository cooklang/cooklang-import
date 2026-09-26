"""Join checker lint + Sonnet grades + run records, on the fixed grade sample.

    python3 report.py --version v2 --pipelines A1,A8   # grader saw the full document
    python3 report.py --version v3 --pipelines A1,A9   # ... plus the page metadata block (default)

Prints per-pipeline grader and CLEAN rates, the Haiku judge (full document; with the metadata
block for v3, i.e. exactly the A8/A9 judge call; cached) vs grader agreement on the A1 drafts,
and writes (gitignored, contain recipe text), <last> being the last pipeline:
  results/grader_issues_sample_<version>.jsonl  40 random <last> grader issues, for theme clustering
  results/calibration_<version>.jsonl           30 random <last> recipes where checker and grader disagree
  results/summary.md                            the aggregate tables only

The v1 grades (results/grades.jsonl, body-only grader input) are invalid and not read.
"""
import argparse, asyncio, json, random

from common import RESULTS, lint_many, load_inputs, split_frontmatter
from compare import drift
from run import judge

ap = argparse.ArgumentParser()
ap.add_argument("--version", default="v3", choices=["v2", "v3"])
ap.add_argument("--pipelines", default="A1,A9")
ARGS = ap.parse_args()
VERSION, PIPES = ARGS.version, ARGS.pipelines.split(",")
LAST = PIPES[-1]
GRADER_INPUT = {"v2": "full_doc", "v3": "full_doc+metadata"}[VERSION]
inputs = {r["udid"]: r for r in load_inputs()}
grades = {}
for l in open(RESULTS / f"grades_{VERSION}.jsonl"):
    g = json.loads(l)
    assert g.get("grader_input") == GRADER_INPUT, g["id"]
    grades[g["id"]] = g["grade"]
sample = json.loads((RESULTS / "grade_sample.json").read_text())
a1 = {json.loads(l)["udid"]: json.loads(l) for l in open(RESULTS / "a0_a1_lint.jsonl")}
finals = {"A1": {u: r["fixed"] for u, r in a1.items()}}
for p in PIPES[1:]:
    finals[p] = {json.loads(l)["udid"]: json.loads(l)["cook"] for l in open(RESULTS / f"{p}.jsonl")}


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "–"


def good5(g):
    return bool(g) and g.get("verdict") == "good" and g.get("score") == 5


def good4(g):
    return bool(g) and g.get("verdict") == "good" and isinstance(g.get("score"), int) and g["score"] >= 4


def bad(g):
    return not good5(g)


def stats(p, udids):
    lint = lint_many([{**inputs[u], "cook": finals[p][u]} for u in udids])
    a1b = {u: split_frontmatter(a1[u]["fixed"])[1] for u in udids}
    row = {"n": len(udids), "chk": 0, "g5": 0, "good": 0, "clean": 0, "clean_nd": 0, "chk_g4": 0,
           "scores": [0] * 6}
    for u in udids:
        g = grades[f"{p}:{u}"]
        d = drift(a1b[u], split_frontmatter(finals[p][u])[1], a1[u]["fixed_issues"])
        chk = not lint[u]["issues"]
        row["chk"] += chk
        row["g5"] += good5(g)
        row["good"] += bool(g) and g.get("verdict") == "good"
        row["clean"] += chk and good5(g)
        row["clean_nd"] += chk and good5(g) and not d[0] and not d[3]
        row["chk_g4"] += chk and good4(g)
        s = (g or {}).get("score")
        row["scores"][s if isinstance(s, int) and 1 <= s <= 5 else 0] += 1
    return row, lint


def table(title, rows):
    out = [f"### {title}", "", "| pipeline | n graded | checker-clean | grader good+5 | grader good (any) | "
           "**CLEAN** (checker-clean & good+5) | CLEAN & no drift | checker-clean & good (score ≥4) | "
           "scores 1/2/3/4/5 (unparsed) |", "|---|---|---|---|---|---|---|---|---|"]
    for p, r in rows.items():
        n = r["n"]
        out.append(f"| {p} | {n} | {pct(r['chk'], n)} | {pct(r['g5'], n)} | {pct(r['good'], n)} | "
                   f"**{pct(r['clean'], n)}** | {pct(r['clean_nd'], n)} | {pct(r['chk_g4'], n)} | "
                   f"{'/'.join(str(x) for x in r['scores'][1:])} ({r['scores'][0]}) |")
    return out


def main():
    graded = {p: [u for u in sample if f"{p}:{u}" in grades] for p in PIPES}
    lines = [f"# Import quality — graded results ({VERSION}: grader input {GRADER_INPUT})", "",
             f"Sample: {len(sample)} recipes (results/grade_sample.json, random.seed(2026)).", ""]
    rows, lints = {}, {}
    for p in PIPES:
        if graded[p]:
            rows[p], lints[p] = stats(p, graded[p])
    lines += table("Graded sample per pipeline", rows) + [""]

    # Haiku judge on the full A1 document (exactly the A8 / A9 judge call; cached) vs grader on A1.
    async def verdicts(us):
        return await asyncio.gather(*(judge(inputs[u]["original_text"], a1[u]["fixed"],
                                            inputs[u].get("metadata", {}) if VERSION == "v3" else None)
                                      for u in us))
    us = graded["A1"]
    js = asyncio.run(verdicts(us))
    for name, flag in (("judge alone (not good+5)", lambda v, u: bad(v)),
                       (f"{LAST} verdict trigger (judge not good+5 OR checker issue)",
                        lambda v, u: bad(v) or bool(a1[u]["fixed_issues"]))):
        for gname, gbad in (("grader not good+5", bad), ("grader not good≥4", lambda g: not good4(g))):
            c = {"tp": 0, "fp": 0, "fn": 0, "tn": 0}
            for u, (v, _) in zip(us, js):
                f, gb = flag(v, u), gbad(grades[f"A1:{u}"])
                c[("t" if f == gb else "f") + ("p" if f else "n")] += 1
            lines += [f"- {name} vs {gname} on A1 (n={len(us)}): TP {c['tp']}, FP {c['fp']}, FN {c['fn']}, "
                      f"TN {c['tn']}; precision {pct(c['tp'], c['tp'] + c['fp'])}, "
                      f"recall {pct(c['tp'], c['tp'] + c['fn'])}"]
    spend = sum(json.loads(l).get("cost", 0) for l in open(RESULTS / f"grades_{VERSION}.jsonl"))
    lines += ["", f"{VERSION} grader spend: ${spend:.2f}"]

    rng = random.Random(7)
    issues = [(u, s) for u in graded[LAST] for s in (grades[f"{LAST}:{u}"] or {}).get("issues", [])]
    with open(RESULTS / f"grader_issues_sample_{VERSION}.jsonl", "w") as f:
        for u, s in rng.sample(issues, min(40, len(issues))):
            f.write(json.dumps({"pipeline": LAST, "udid": u, "issue": s}, ensure_ascii=False) + "\n")
    dis = [u for u in graded[LAST] if (not lints[LAST][u]["issues"]) != good5(grades[f"{LAST}:{u}"])]
    with open(RESULTS / f"calibration_{VERSION}.jsonl", "w") as f:
        for u in rng.sample(dis, min(30, len(dis))):
            f.write(json.dumps({"udid": u, "checker_clean": not lints[LAST][u]["issues"],
                                "checker_issues": lints[LAST][u]["issues"], "grader": grades[f"{LAST}:{u}"],
                                "original": inputs[u]["original_text"], "cook": finals[LAST][u]},
                               ensure_ascii=False) + "\n")
    lines += [f"{LAST} checker/grader (good+5) disagreements: {len(dis)} of {len(graded[LAST])}; "
              f"{min(30, len(dis))} sampled to results/calibration_{VERSION}.jsonl"]
    (RESULTS / "summary.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
