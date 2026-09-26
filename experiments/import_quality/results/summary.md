# Import quality — graded results (v2: grader sees the full document)

Sample: 200 recipes (results/grade_sample.json, random.seed(2026)).

### Graded sample per pipeline

| pipeline | n graded | checker-clean | grader good+5 | grader good (any) | **CLEAN** (checker-clean & good+5) | CLEAN & no drift | checker-clean & good (score ≥4) | scores 1/2/3/4/5 (unparsed) |
|---|---|---|---|---|---|---|---|---|
| A1 | 200 | 70.0% | 3.0% | 43.5% | **3.0%** | 3.0% | 38.5% | 3/89/22/80/6 (0) |
| A8 | 200 | 92.5% | 5.0% | 55.5% | **5.0%** | 5.0% | 53.0% | 1/68/22/99/10 (0) |

- judge alone (not good+5) vs grader not good+5 on A1 (n=200): TP 164, FP 2, FN 30, TN 4; precision 98.8%, recall 84.5%
- judge alone (not good+5) vs grader not good≥4 on A1 (n=200): TP 103, FP 63, FN 11, TN 23; precision 62.0%, recall 90.4%
- A8 verdict trigger (judge not good+5 OR checker issue) vs grader not good+5 on A1 (n=200): TP 171, FP 2, FN 23, TN 4; precision 98.8%, recall 88.1%
- A8 verdict trigger (judge not good+5 OR checker issue) vs grader not good≥4 on A1 (n=200): TP 108, FP 65, FN 6, TN 21; precision 62.4%, recall 94.7%

v2 grader spend: $7.27
A8 checker/grader (good+5) disagreements: 175 of 200; 30 sampled to results/calibration_v2.jsonl
