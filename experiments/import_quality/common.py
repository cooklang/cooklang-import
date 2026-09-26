"""Shared helpers: cached Anthropic calls, lint binary, pricing."""
import asyncio, hashlib, json, os, pathlib, re, subprocess, time

import anthropic

ROOT = pathlib.Path(__file__).parent
REPO = ROOT.parent.parent
CACHE = ROOT / "cache"
RESULTS = ROOT / "results"
LINT_BIN = REPO / "target/release/cooklang-lint"
CONVERT_PROMPT = (REPO / "src/converters/prompt.txt").read_text()
EVAL_PROMPT = (REPO / "src/converters/eval_prompt.txt").read_text()
REPAIR_PROMPT = (ROOT / "prompts/repair_prompt.txt").read_text()
REPAIR_PROMPT_V1 = (ROOT / "prompts/repair_prompt_v1.txt").read_text()
REPAIR_EDITS_PROMPT = (ROOT / "prompts/repair_edits_prompt.txt").read_text()
REPAIR_EDITS_PROMPT_V2 = (ROOT / "prompts/repair_edits_prompt_v2.txt").read_text()

CACHE.mkdir(exist_ok=True)
RESULTS.mkdir(exist_ok=True)

HAIKU, SONNET = "claude-haiku-4-5", "claude-sonnet-5"
# $ per million tokens (input, output); batch is half price.
PRICES = {HAIKU: (1.0, 5.0), SONNET: (2.0, 10.0)}

client = anthropic.AsyncAnthropic(timeout=120, max_retries=2)
sem = asyncio.Semaphore(8)


def load_inputs():
    return [json.loads(l) for l in open(ROOT / "data/inputs.jsonl")]


def split_frontmatter(doc):
    if doc.startswith("---\n"):
        end = doc.find("\n---", 4)
        if end != -1:
            return doc[4:end], doc[end + 4:].lstrip("\n")
    return None, doc


def join_frontmatter(fm, body):
    return f"---\n{fm}\n---\n\n{body.strip()}\n" if fm else body.strip() + "\n"


def cost(model, usage, batch=False):
    pin, pout = PRICES[model]
    c = (usage["input_tokens"] * pin + usage["output_tokens"] * pout) / 1e6
    return c / 2 if batch else c


async def call(model, prompt, max_tokens=8000, **extra):
    """Cached single-turn call. Returns {text, usage, latency_ms, cost}.

    With `tools=[...]` and a forced `tool_choice`, the result also carries
    `tool_input`: the input of the first tool_use block (or None).
    """
    key = hashlib.sha256(json.dumps([model, prompt, max_tokens, extra], sort_keys=True).encode()).hexdigest()
    path = CACHE / f"{key}.json"
    if path.exists():
        return {**json.loads(path.read_text()), "cached": True}
    async with sem:
        for attempt in range(5):
            try:
                t0 = time.monotonic()
                msg = await client.messages.create(
                    model=model, max_tokens=max_tokens,
                    messages=[{"role": "user", "content": prompt}], **extra)
                latency = int((time.monotonic() - t0) * 1000)
                break
            except (anthropic.RateLimitError, anthropic.APIConnectionError, anthropic.InternalServerError):
                await asyncio.sleep(2 ** attempt * 5)
        else:
            raise RuntimeError(f"{model} call failed after retries")
    if msg.stop_reason == "refusal":
        raise RuntimeError(f"refusal on {key}")
    text = "".join(b.text for b in msg.content if b.type == "text")
    usage = {"input_tokens": msg.usage.input_tokens, "output_tokens": msg.usage.output_tokens}
    result = {"text": text, "usage": usage, "latency_ms": latency, "cost": cost(model, usage),
              "stop_reason": msg.stop_reason}
    if "tools" in extra:
        result["tool_input"] = next((b.input for b in msg.content if b.type == "tool_use"), None)
    path.write_text(json.dumps(result))
    return result


def parse_json(text):
    m = re.search(r"\{.*\}", text, re.S)
    try:
        return json.loads(m.group(0)) if m else None
    except json.JSONDecodeError:
        return None


def lint_many(rows, fix=False):
    """rows: [{udid, original_text, cook}] -> {udid: lint record}."""
    args = [str(LINT_BIN)] + (["--fix"] if fix else [])
    inp = "".join(json.dumps(r) + "\n" for r in rows)
    out = subprocess.run(args, input=inp, capture_output=True, text=True, check=True).stdout
    return {rec["udid"]: rec for rec in map(json.loads, out.splitlines())}


def clean_output(text):
    """Strip code fences a model may wrap around Cooklang."""
    text = text.strip()
    m = re.match(r"^```[a-z]*\n(.*)\n```$", text, re.S)
    return m.group(1) if m else text
