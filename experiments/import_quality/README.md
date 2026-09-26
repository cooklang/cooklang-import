# Import quality experiment (2026-09-25)

Spec: growth repo `docs/2026-09-25-import-quality-experiment-design.md`.

    cargo build --release --features lint --bin cooklang-lint
    set -a; source /Users/alexeydubovskoy/Cooklang/growth/.env; set +a
    python3 experiments/import_quality/run.py --pipelines A2,A3,B0,B1
    python3 experiments/import_quality/run.py --pipelines A4   # edit-based repair + guards
    python3 experiments/import_quality/run.py --pipelines A5,A6  # improved whole-body / edit-level rejection
    (cd experiments/import_quality && python3 compare.py)      # A0/A1/A2/A4 table, re-linted
    python3 experiments/import_quality/run.py --pipelines A8     # A7 with the full-document judge
    (cd experiments/import_quality && python3 grade.py --pipelines A1,A8 --estimate 5)
    (cd experiments/import_quality && python3 grade.py --pipelines A1,A8 --submit)   # 200-recipe sample
    (cd experiments/import_quality && python3 poll_batches.py)   # <= ~8 min per call; rerun until it exits 0
    (cd experiments/import_quality && python3 report.py)

A2 uses `prompts/repair_prompt_v1.txt` (the baseline, frozen). `prompts/repair_prompt.txt` is the
improved whole-body prompt (pipeline A5). A4 uses `prompts/repair_edits_prompt.txt` (numbered
paragraphs); A6 uses `prompts/repair_edits_prompt_v2.txt` (unnumbered draft, notes in the recipe's
language) and rejects offending edits one by one instead of the whole repair.

**Grader/judge input (fixed 2026-09-26).** Up to A7 the Haiku judge and the Sonnet grader were given
the recipe body only, so eval_prompt.txt's "missing frontmatter" rule fired on nearly every recipe:
`results/grades.jsonl` (v1) is invalid. A8 = A7 with the judge on the full document; metadata judge
issues (`run.METADATA_ISSUE`) are dropped from the body-only repair's problem list. `grade.py` now
grades full documents into `results/grades_v2.jsonl` (`"grader_input": "full_doc"`) on the fixed
200-recipe sample `results/grade_sample.json` (random.seed(2026)). Both models still flag
frontmatter servings/times/nutrition/image as "fabricated", because that metadata comes from the
page's structured data, which is not part of `original_text`.

API keys (e.g. `ANTHROPIC_API_KEY`) come from `/Users/alexeydubovskoy/Cooklang/growth/.env` —
load them into the shell with `set -a; source /Users/alexeydubovskoy/Cooklang/growth/.env; set +a`
before running `run.py`. Never print or commit the contents of that file, and never commit a key.

`data/`, `cache/` and `results/*.jsonl` hold user recipe text and are gitignored.
Only `results/summary.md` is committed.
