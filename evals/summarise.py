#!/usr/bin/env python3
"""One table from evals/out/*.score.json: per configuration, weighted WER,
clinical term error rate, real-time factor, and the terms most often missed."""
from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

out = Path(__file__).resolve().parent / "out"
rows: dict[str, list[dict]] = defaultdict(list)
for f in sorted(out.glob("*.score.json")):
    parts = f.name.split(".")
    if len(parts) < 3:  # not a run output
        continue
    tag = parts[1] if len(parts) > 3 else "(untagged)"  # pre-bake-off runs: no --tag
    rows[tag].append(json.loads(f.read_text()))

print(f"{'configuration':22} {'n':>2} {'WER':>6} {'term err':>8} {'RTF':>5}  most missed terms")
for tag, rs in sorted(rows.items()):
    ref_w = sum(r["reference_words"] for r in rs)
    err_w = sum(r["wer"] * r["reference_words"] for r in rs)
    t_in = sum(r.get("terms_in_ref", 0) for r in rs)
    t_found = sum(r.get("terms_found", 0) for r in rs)
    rtfs = [r["rtf"] for r in rs if r.get("rtf")]
    missed = Counter()
    for r in rs:
        for m in r.get("terms_missed", []):
            name, _, n = m.rpartition(" ×")
            missed[name] += int(n or 1)
    term_err = f"{1 - t_found / t_in:6.1%}" if t_in else "   n/a"
    rtf = f"{sum(rtfs) / len(rtfs):5.2f}" if rtfs else "  n/a"
    top = ", ".join(f"{k} {v}" for k, v in missed.most_common(6))
    print(f"{tag:22} {len(rs):>2} {err_w / ref_w:6.1%} {term_err:>8} {rtf:>5}  {top}")
if len(sys.argv) > 1:
    for tag, rs in sorted(rows.items()):
        print(f"\n{tag}:")
        for r in rs:
            print(f"  WER {r['wer']:.3f} terms {r.get('terms_found')}/{r.get('terms_in_ref')} rtf {r.get('rtf')}")
