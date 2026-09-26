# Import quality experiment (2026-09-25)

Spec: growth repo `docs/2026-09-25-import-quality-experiment-design.md`.

    cargo build --release --features lint --bin cooklang-lint
    set -a; source /Users/alexeydubovskoy/Cooklang/growth/.env; set +a
    python3 experiments/import_quality/run.py --pipelines A2,A3,B0,B1
    python3 experiments/import_quality/run.py --pipelines A4   # edit-based repair + guards
    (cd experiments/import_quality && python3 compare.py)      # A0/A1/A2/A4 table, re-linted
    python3 experiments/import_quality/grade.py
    python3 experiments/import_quality/report.py

A2 uses `prompts/repair_prompt_v1.txt` (the baseline, frozen). `prompts/repair_prompt.txt` is the
improved whole-body prompt (pipeline A5, not yet run). A4 uses `prompts/repair_edits_prompt.txt`.

API keys (e.g. `ANTHROPIC_API_KEY`) come from `/Users/alexeydubovskoy/Cooklang/growth/.env` —
load them into the shell with `set -a; source /Users/alexeydubovskoy/Cooklang/growth/.env; set +a`
before running `run.py`. Never print or commit the contents of that file, and never commit a key.

`data/`, `cache/` and `results/*.jsonl` hold user recipe text and are gitignored.
Only `results/summary.md` is committed.
