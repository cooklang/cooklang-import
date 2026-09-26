"""Adds the scraped page metadata (the JSON-LD fields the frontmatter was generated from) to
data/inputs.jsonl as a `metadata` object per row. In production the same information travels in
RecipeComponents.metadata.

    python3 add_metadata.py /path/to/cookifications-YYYYMMDD-YYYYMMDD.jsonl

Empty strings are dropped; rows whose udid isn't in the export get `metadata: {}`. Idempotent.
"""
import json, sys

from common import ROOT

FIELDS = ["name", "servings", "prep_time", "cook_time", "total_time", "author", "tags", "source_url",
          "description", "image_url"]


def main(export):
    meta = {}
    for line in open(export):
        r = json.loads(line)
        meta[r["udid"]] = {f: r[f].strip() for f in FIELDS if isinstance(r.get(f), str) and r[f].strip()}
    path = ROOT / "data/inputs.jsonl"
    rows = [json.loads(l) for l in open(path)]
    for r in rows:
        r["metadata"] = meta.get(r["udid"], {})
    tmp = path.with_suffix(".jsonl.tmp")
    tmp.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows))
    tmp.replace(path)
    print(f"{sum(bool(r['metadata']) for r in rows)} of {len(rows)} rows have metadata")


if __name__ == "__main__":
    main(sys.argv[1])
