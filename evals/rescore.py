#!/usr/bin/env python3 -I
"""Re-score every evals/out/*.transcript.txt against the references without
re-running a recogniser — for when the term list or the scorer changes. Keeps
the timing fields (audio/processing seconds, RTF) from the existing
score.json, since those come from the run itself. Facts files are picked up
when present. Usage: python3 -I evals/rescore.py"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import score  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "evals/out"
TERMS = score.load_terms(ROOT / "evals/terms-clinical.txt")


def main() -> int:
    n = 0
    for hyp in sorted(OUT.glob("*.transcript.txt")):
        stem = hyp.name[: -len(".transcript.txt")]
        consultation = stem.split(".")[0]
        old_path = OUT / f"{stem}.score.json"
        old = json.loads(old_path.read_text()) if old_path.exists() else {}
        facts_path = OUT / f"{stem}.facts.json"
        facts = json.loads(facts_path.read_text()).get("facts") if facts_path.exists() else None
        report = score.score(score.reference_text(ROOT / "evals/fixtures/primock57" / consultation), hyp.read_text(), facts, old.get("audio_seconds"), old.get("processing_seconds"), TERMS)
        old_path.write_text(json.dumps(report, indent=2))
        n += 1
    print(f"re-scored {n} transcripts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
