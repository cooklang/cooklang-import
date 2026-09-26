"""Grade pipelines' final outputs with Sonnet 5 (eval_prompt.txt) -> results/grades.jsonl.

    python3 grade.py --pipelines A1,A6,A7 --estimate 10   # 10 direct (non-batch) calls, project cost
    python3 grade.py --pipelines A1,A6,A7 --submit        # create batch(es), ids -> results/grade_batches.json
    python3 grade.py --pipelines A1 --sample 300 --submit # a fixed random subset of recipes
    python3 grade.py --collect                            # poll once; append finished results

Each graded item is keyed "<pipeline>:<udid>" (custom_id "<pipeline>__<udid>", which matches
^[a-zA-Z0-9_-]{1,64}$). No temperature is passed (Sonnet 5 rejects it); only text blocks are read.
Resumable: already-graded ids are skipped.
"""
import argparse, json, random, re

import anthropic

from common import EVAL_PROMPT, RESULTS, SONNET, cost, load_inputs, parse_json, split_frontmatter

client = anthropic.Anthropic()
GRADES = RESULTS / "grades.jsonl"
BATCHES = RESULTS / "grade_batches.json"
CUSTOM_ID = re.compile(r"^[a-zA-Z0-9_-]{1,64}$")


def outputs(pipelines):
    inputs = {r["udid"]: r["cook"] for r in load_inputs()}
    for p in pipelines:
        if p in ("A0", "A1"):
            for l in open(RESULTS / "a0_a1_lint.jsonl"):
                r = json.loads(l)
                yield p, r["udid"], inputs[r["udid"]] if p == "A0" else r["fixed"]
        else:
            for l in open(RESULTS / f"{p}.jsonl"):
                r = json.loads(l)
                yield p, r["udid"], r["cook"]


def params(original, cook):
    _, body = split_frontmatter(cook)
    prompt = EVAL_PROMPT.replace("{{ORIGINAL}}", original).replace("{{COOKLANG}}", body)
    return {"model": SONNET, "max_tokens": 16000, "messages": [{"role": "user", "content": prompt}]}


def record(cid, msg, batch):
    text = "".join(b.text for b in msg.content if b.type == "text")
    usage = {"input_tokens": msg.usage.input_tokens, "output_tokens": msg.usage.output_tokens}
    return {"id": cid, "grade": parse_json(text), "usage": usage, "stop_reason": msg.stop_reason,
            "cost": cost(SONNET, usage, batch=batch), "batch": batch}


def sample_udids(n):
    """A fixed random subset of n recipes (seeded), for grading a pipeline on a sample."""
    udids = sorted(r["udid"] for r in load_inputs())
    return set(random.Random(20260926).sample(udids, n)) if n else set(udids)


def pending(pipelines, sample=0):
    originals = {r["udid"]: r["original_text"] for r in load_inputs()}
    done = {json.loads(l)["id"] for l in open(GRADES)} if GRADES.exists() else set()
    keep = sample_udids(sample)
    for p, udid, cook in outputs(pipelines):
        if udid not in keep:
            continue
        cid = f"{p}:{udid}"
        if cid not in done:
            yield cid, params(originals[udid], cook)


def estimate(pipelines, n):
    todo = list(pending(pipelines))
    random.seed(11)
    sample = random.sample(todo, min(n, len(todo)))
    recs = []
    with open(GRADES, "a") as f:
        for cid, prm in sample:
            rec = record(cid, client.messages.create(**prm), batch=False)
            f.write(json.dumps(rec) + "\n")
            recs.append(rec)
    per = sum(r["cost"] for r in recs) / len(recs)
    rest = len(todo) - len(recs)
    print(f"{len(recs)} graded directly: ${sum(r['cost'] for r in recs):.3f} "
          f"(${per:.4f}/item, output tokens p50 {sorted(r['usage']['output_tokens'] for r in recs)[len(recs) // 2]})")
    print(f"{rest} remaining -> projected batch cost ${per / 2 * rest:.2f} (non-batch ${per * rest:.2f})")
    print("verdicts:", [(r["grade"] or {}).get("verdict") for r in recs])


def submit(pipelines, sample=0):
    reqs = []
    for cid, prm in pending(pipelines, sample):
        custom_id = cid.replace(":", "__")
        assert CUSTOM_ID.match(custom_id), custom_id
        reqs.append({"custom_id": custom_id, "params": prm})
    print(f"{len(reqs)} to grade")
    if not reqs:
        return
    ids = json.loads(BATCHES.read_text()) if BATCHES.exists() else []
    for i in range(0, len(reqs), 5000):
        ids.append(client.messages.batches.create(requests=reqs[i:i + 5000]).id)
    BATCHES.write_text(json.dumps(ids))
    print("batches:", ids)


def collect():
    ids = json.loads(BATCHES.read_text()) if BATCHES.exists() else []
    done = {json.loads(l)["id"] for l in open(GRADES)} if GRADES.exists() else set()
    all_ended = True
    for bid in ids:
        b = client.messages.batches.retrieve(bid)
        print(bid, b.processing_status, b.request_counts)
        if b.processing_status != "ended":
            all_ended = False
            continue
        with open(GRADES, "a") as f:
            for res in client.messages.batches.results(bid):
                cid = res.custom_id.replace("__", ":")
                if cid in done:
                    continue
                if res.result.type != "succeeded":
                    print(f"  {cid}: {res.result.type}")
                    continue
                f.write(json.dumps(record(cid, res.result.message, batch=True)) + "\n")
                done.add(cid)
    print("all ended" if all_ended else "pending")


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--pipelines", default="A1,A6,A7")
    ap.add_argument("--estimate", type=int, default=0)
    ap.add_argument("--submit", action="store_true")
    ap.add_argument("--collect", action="store_true")
    ap.add_argument("--sample", type=int, default=0, help="grade only a fixed random subset of N recipes")
    a = ap.parse_args()
    pipes = a.pipelines.split(",")
    if a.estimate:
        estimate(pipes, a.estimate)
    elif a.submit:
        submit(pipes, a.sample)
    elif a.collect:
        collect()
