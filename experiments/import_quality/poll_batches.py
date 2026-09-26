"""Poll the v2 grading batches (results/grade_batches_v2.json) every 30 s for at most ~8 minutes,
printing status; when all have ended, collect their results into results/grades_v2.jsonl.
Exit code 0 = all ended and collected, 3 = still pending (run again)."""
import json, sys, time

from grade import BATCHES, client, collect

DEADLINE = time.monotonic() + 8 * 60

while True:
    ids = json.loads(BATCHES.read_text()) if BATCHES.exists() else []
    states = [client.messages.batches.retrieve(b) for b in ids]
    for b in states:
        c = b.request_counts
        print(time.strftime("%H:%M:%S"), b.id, b.processing_status,
              f"processing={c.processing} succeeded={c.succeeded} errored={c.errored} expired={c.expired}", flush=True)
    if ids and all(b.processing_status == "ended" for b in states):
        collect()
        sys.exit(0)
    if time.monotonic() + 30 > DEADLINE:
        print("still pending; run poll_batches.py again")
        sys.exit(3)
    time.sleep(30)
