"""Grade pipelines' final outputs with Sonnet 5 (eval_prompt.txt) -> results/grades_v2.jsonl.

v2 (2026-09-26): the grader sees the FULL document (frontmatter + body). v1 passed the body only,
so eval_prompt.txt's "missing frontmatter/title" rule made ~every grade a 2; those v1 grades in
results/grades.jsonl are INVALID and are no longer read or written. v2 records carry
"grader_input": "full_doc".

v3 (2026-09-26): full document plus the page metadata block (prompts/page_metadata_wrapper.txt,
from data/inputs.jsonl `metadata`) -> results/grades_v3.jsonl, "grader_input": "full_doc+metadata".
--version picks v2 or v3 (default v3); each has its own grades and batch-id files.

    python3 grade.py --pipelines A1,A9 --estimate 5            # 5 direct (non-batch) calls, project cost
    python3 grade.py --pipelines A1,A9 --submit                # batch(es), ids -> results/grade_batches_v3.json
    python3 grade.py --collect                                 # poll once; append finished results
    python3 poll_batches.py [--version v3]                     # poll up to ~8 min, then collect

The recipe sample (default 200) is fixed in results/grade_sample.json: random.seed(2026), then
random.sample over the udids of data/inputs.jsonl in file order. --sample 0 grades everything.

Each graded item is keyed "<pipeline>:<udid>" (custom_id "<pipeline>__<udid>", which matches
^[a-zA-Z0-9_-]{1,64}$). No temperature is passed (Sonnet 5 rejects it); only text blocks are read.
Resumable: already-graded ids are skipped.
"""
import argparse, json, random, re

import anthropic

from common import RESULTS, SONNET, cost, eval_prompt, load_inputs, parse_json

client = anthropic.Anthropic()
SAMPLE = RESULTS / "grade_sample.json"
VERSIONS = {"v2": ("full_doc", False), "v3": ("full_doc+metadata", True)}
GRADES = BATCHES = GRADER_INPUT = WITH_METADATA = None


def use(version):
    """Selects the grader input version: its grades file, batch-id file and prompt."""
    global GRADES, BATCHES, GRADER_INPUT, WITH_METADATA
    GRADER_INPUT, WITH_METADATA = VERSIONS[version]
    GRADES = RESULTS / f"grades_{version}.jsonl"
    BATCHES = RESULTS / f"grade_batches_{version}.json"


use("v3")
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


def params(row, cook):
    prompt = eval_prompt(row["original_text"], cook, row.get("metadata", {}) if WITH_METADATA else None)
    return {"model": SONNET, "max_tokens": 16000, "messages": [{"role": "user", "content": prompt}]}


def record(cid, msg, batch):
    text = "".join(b.text for b in msg.content if b.type == "text")
    if msg.stop_reason != "end_turn":
        print(f"  {cid}: stop_reason {msg.stop_reason}")
    usage = {"input_tokens": msg.usage.input_tokens, "output_tokens": msg.usage.output_tokens}
    return {"id": cid, "grade": parse_json(text), "usage": usage, "stop_reason": msg.stop_reason,
            "cost": cost(SONNET, usage, batch=batch), "batch": batch, "grader_input": GRADER_INPUT}


def sample_udids(n):
    """The fixed recipe sample (results/grade_sample.json, created on first use); n=0 -> all."""
    udids = [r["udid"] for r in load_inputs()]
    if not n:
        return set(udids)
    if not SAMPLE.exists():
        random.seed(2026)
        SAMPLE.write_text(json.dumps(random.sample(udids, n)))
    sample = json.loads(SAMPLE.read_text())
    assert len(sample) == n, f"{SAMPLE} holds {len(sample)} udids, not {n}"
    return set(sample)


def pending(pipelines, sample=0):
    originals = {r["udid"]: r for r in load_inputs()}
    done = {json.loads(l)["id"] for l in open(GRADES)} if GRADES.exists() else set()
    keep = sample_udids(sample)
    for p, udid, cook in outputs(pipelines):
        if udid not in keep:
            continue
        cid = f"{p}:{udid}"
        if cid not in done:
            yield cid, params(originals[udid], cook)


def estimate(pipelines, n, sample=0):
    todo = list(pending(pipelines, sample))
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
    ap.add_argument("--pipelines", default="A1,A9")
    ap.add_argument("--version", default="v3", choices=sorted(VERSIONS))
    ap.add_argument("--estimate", type=int, default=0)
    ap.add_argument("--submit", action="store_true")
    ap.add_argument("--collect", action="store_true")
    ap.add_argument("--sample", type=int, default=200, help="grade only the fixed sample of N recipes (0 = all)")
    a = ap.parse_args()
    pipes = a.pipelines.split(",")
    use(a.version)
    if a.estimate:
        estimate(pipes, a.estimate, a.sample)
    elif a.submit:
        submit(pipes, a.sample)
    elif a.collect:
        collect()
