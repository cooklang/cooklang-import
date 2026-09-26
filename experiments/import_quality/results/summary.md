# Import quality — graded results (v3: grader input full_doc+metadata)

Sample: 200 recipes (results/grade_sample.json, random.seed(2026)).

### Graded sample per pipeline

| pipeline | n graded | checker-clean | grader good+5 | grader good (any) | **CLEAN** (checker-clean & good+5) | CLEAN & no drift | checker-clean & good (score ≥4) | scores 1/2/3/4/5 (unparsed) |
|---|---|---|---|---|---|---|---|---|
| A1 | 200 | 54.0% | 5.5% | 49.5% | **4.0%** | 4.0% | 33.0% | 2/76/24/87/11 (0) |
| A9 | 200 | 89.0% | 7.0% | 65.0% | **7.0%** | 7.0% | 61.5% | 0/53/17/116/14 (0) |

- judge alone (not good+5) vs grader not good+5 on A1 (n=200): TP 164, FP 8, FN 25, TN 3; precision 95.3%, recall 86.8%
- judge alone (not good+5) vs grader not good≥4 on A1 (n=200): TP 97, FP 75, FN 5, TN 23; precision 56.4%, recall 95.1%
- A9 verdict trigger (judge not good+5 OR checker issue) vs grader not good+5 on A1 (n=200): TP 170, FP 8, FN 19, TN 3; precision 95.5%, recall 89.9%
- A9 verdict trigger (judge not good+5 OR checker issue) vs grader not good≥4 on A1 (n=200): TP 98, FP 80, FN 4, TN 18; precision 55.1%, recall 96.1%

v3 grader spend: $7.33
A9 checker/grader (good+5) disagreements: 164 of 200; 30 sampled to results/calibration_v3.jsonl
