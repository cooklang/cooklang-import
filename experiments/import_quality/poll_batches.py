"""Poll the grading batches (results/grade_batches_<version>.json, default v3) every 30 s for at most ~8 minutes,
printing status; when all have ended, collect their results into results/grades_<version>.jsonl.
Exit code 0 = all ended and collected, 3 = still pending (run again)."""
import argparse, json, sys, time

import grade

ap = argparse.ArgumentParser()
ap.add_argument("--version", default="v3", choices=sorted(grade.VERSIONS))
grade.use(ap.parse_args().version)
BATCHES, client = grade.BATCHES, grade.client

DEADLINE = time.monotonic() + 8 * 60

while True:
    ids = json.loads(BATCHES.read_text()) if BATCHES.exists() else []
    states = [client.messages.batches.retrieve(b) for b in ids]
    for b in states:
        c = b.request_counts
        print(time.strftime("%H:%M:%S"), b.id, b.processing_status,
              f"processing={c.processing} succeeded={c.succeeded} errored={c.errored} expired={c.expired}", flush=True)
    if ids and all(b.processing_status == "ended" for b in states):
        grade.collect()
        sys.exit(0)
    if time.monotonic() + 30 > DEADLINE:
        print("still pending; run poll_batches.py again")
        sys.exit(3)
    time.sleep(30)
