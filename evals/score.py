#!/usr/bin/env python3
"""Score a transcript (and optional facts) against a PriMock57 reference.

    python3 evals/score.py --ref evals/fixtures/primock57/day1_consultation01 \
        --hyp out/transcript.txt [--facts out/facts.json] [--seconds 12.3]

`--ref` is the consultation prefix; both `_doctor.TextGrid` and `_patient.TextGrid`
are read and merged by time. Reports: word error rate, the share of reference
numbers that survived, numbers in the facts that the reference never said
(the "invented number" count — the CHW gate), and real-time factor if the
processing time is given. Standard library only. See knowledge/26 §gates.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

TAG = re.compile(r"<UNSURE>|</UNSURE>|<UNIN/>")
# Whole numbers and decimals only, not digits inside words (HbA1c) and not
# compounds: "150/95" yields 150 and 95, each checked on its own.
NUM = re.compile(r"(?<![A-Za-z0-9.])\d+(?:\.\d+)?(?![A-Za-z0-9])")


def parse_textgrid(text: str) -> list[tuple[float, float, str]]:
    """Intervals (xmin, xmax, text) with non-empty text, in file order."""
    out = []
    xmin = xmax = None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("xmin = "):
            xmin = float(line[7:])
        elif line.startswith("xmax = "):
            xmax = float(line[7:])
        elif line.startswith("text = "):
            t = line[7:].strip().strip('"')
            if t and xmin is not None and xmax is not None:
                out.append((xmin, xmax, t))
    return out


def reference_text(prefix: Path) -> str:
    """Both speakers' utterances merged by start time, tags removed."""
    utts = []
    for side in ("doctor", "patient"):
        p = Path(f"{prefix}_{side}.TextGrid")
        if p.exists():
            utts.extend(parse_textgrid(p.read_text()))
    utts.sort(key=lambda u: u[0])
    return " ".join(TAG.sub(" ", t) for _, _, t in utts)


def normalise(text: str) -> list[str]:
    text = text.lower().replace("’", "'")
    text = re.sub(r"[^a-z0-9'/.,% ]+", " ", text)
    # Spoken fillers the transcribers kept; a recogniser is not wrong to drop them.
    words = [w.strip(".,") for w in text.split()]
    return [w for w in words if w and w not in {"um", "uh", "mm", "hmm", "mhm", "erm", "ah"}]


def wer(ref: list[str], hyp: list[str]) -> float:
    """Levenshtein over words, divided by the reference length."""
    if not ref:
        return 0.0 if not hyp else 1.0
    prev = list(range(len(hyp) + 1))
    for i, r in enumerate(ref, 1):
        cur = [i] + [0] * len(hyp)
        for j, h in enumerate(hyp, 1):
            cur[j] = min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (r != h))
        prev = cur
    return prev[-1] / len(ref)


def numbers(text: str) -> set[str]:
    return set(NUM.findall(text.replace(",", "")))


def load_terms(path: Path) -> list[list[list[str]]]:
    """Each term is a list of accepted spellings, each spelling a token list."""
    terms = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        terms.append([normalise(alt) for alt in line.split("|") if normalise(alt)])
    return terms


def count_seq(tokens: list[str], seq: list[str]) -> int:
    n = len(seq)
    return sum(1 for i in range(len(tokens) - n + 1) if tokens[i : i + n] == seq)


def term_recall(ref: list[str], hyp: list[str], terms: list[list[list[str]]]) -> dict:
    """Of every clinical term the reference says, the share the hypothesis
    also says (any accepted spelling), plus the misses — the clinician's
    vocabulary, which plain WER buries under the small words."""
    in_ref = found = 0
    missed: list[str] = []
    for alts in terms:
        r = sum(count_seq(ref, a) for a in alts)
        if r == 0:
            continue
        h = sum(count_seq(hyp, a) for a in alts)
        in_ref += r
        found += min(r, h)
        if h < r:
            missed.append(f"{' '.join(alts[0])} ×{r - min(r, h)}")
    return {"terms_in_ref": in_ref, "terms_found": found, "term_error_rate": round(1 - found / in_ref, 4) if in_ref else None, "terms_missed": missed}


def score(ref_text: str, hyp_text: str, facts: list[dict] | None, audio_seconds: float | None, processing_seconds: float | None, terms: list[list[list[str]]] | None = None) -> dict:
    ref, hyp = normalise(ref_text), normalise(hyp_text)
    ref_nums = numbers(ref_text)
    hyp_nums = numbers(hyp_text)
    report = {
        "reference_words": len(ref),
        "hypothesis_words": len(hyp),
        "wer": round(wer(ref, hyp), 4),
        "reference_numbers": len(ref_nums),
        "numbers_kept": len(ref_nums & hyp_nums),
    }
    if terms:
        report.update(term_recall(ref, hyp, terms))
    if facts is not None:
        fact_nums = set()
        for f in facts:
            fact_nums |= numbers(str(f.get("text", "")))
        report["facts"] = len(facts)
        report["fact_groups"] = sorted({f.get("group", "?") for f in facts})
        report["invented_numbers"] = sorted(fact_nums - ref_nums - hyp_nums)
    if audio_seconds and processing_seconds:
        report["audio_seconds"] = round(audio_seconds, 1)
        report["processing_seconds"] = round(processing_seconds, 1)
        report["rtf"] = round(processing_seconds / audio_seconds, 3)
    return report


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--ref", required=True, help="consultation prefix (without _doctor/_patient.TextGrid)")
    ap.add_argument("--hyp", required=True, help="hypothesis transcript, plain text")
    ap.add_argument("--facts", help="JSON with a top-level 'facts' list")
    ap.add_argument("--audio-seconds", type=float)
    ap.add_argument("--seconds", type=float, help="processing wall time")
    ap.add_argument("--terms", help="clinical term list (one per line, | for spellings)")
    a = ap.parse_args()
    facts = json.loads(Path(a.facts).read_text()).get("facts") if a.facts else None
    terms = load_terms(Path(a.terms)) if a.terms else None
    r = score(reference_text(Path(a.ref)), Path(a.hyp).read_text(), facts, a.audio_seconds, a.seconds, terms)
    print(json.dumps(r, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
