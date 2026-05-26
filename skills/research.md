# Research

Honest inquiry under uncertainty. The discipline that keeps Verdure's claims true.

## The premise

Verdure makes claims about the world: tokens per second on specific hardware, model behavior on specific tasks, license terms, runtime backend capabilities. Every claim is either grounded in evidence or marked as provisional. There is no third category.

## When you encounter a [GAP]

Knowledge files contain `**[GAP — needs research]**` markers where information is missing. These are honest about uncertainty. **Do not invent content to close a gap.** Either:

1. **Do the research.** Find an authoritative source, validate the finding, update the file, mark the originating `[OQ-XX]` resolved.
2. **Leave the gap visible.** If the research can't be done now, the marker stays. A future Claude knows it's still open.

What you must not do: paper over the gap with plausible-sounding text. That is exactly how a knowledge base becomes useless — when the markers stop signaling real uncertainty, no one trusts them.

## Source quality hierarchy

When evidence is needed:

1. **Primary sources first** — official documentation, the actual paper, the model card from the author, the license text, the benchmark from the team that ran it.
2. **Authoritative secondary sources** — recognized maintainers, well-known engineers, established publications with editorial review.
3. **Community sources with care** — blog posts, forums, Stack Overflow. Useful for direction; verify before citing.
4. **Last resort** — extrapolation from adjacent data, clearly marked as such.

If a source is one you wouldn't cite to a skeptical reviewer, it isn't good enough to ground a Verdure claim.

## When real-world validation is required

Some claims need physical measurement, not literature review. Verdure's tokens-per-second numbers for Tier P1 / P2 are this category — see `[OQ-03]` and `[OQ-05]`. Until a real Unisoc T606 device runs Gemma 3 1B at Q4_K_M and is measured, the number in `../knowledge/03-model-matrix.md` is an extrapolation. The file must say so (Status: 🟡 Provisional).

The line: provisional content is fine to ship, but it must be marked. Unmarked uncertainty is dishonesty.

## Don't extrapolate silently

Extrapolation is a legitimate research tool. "Llama 3.2 3B on Snapdragon 8 Gen 1 gets ~14 tok/s, so on the lower-clocked 7 Gen 1 we'd expect ~10–12" is reasonable. But the extrapolation must be visible in the file, not hidden behind a confident number. "~10–12 tok/s (extrapolated from Snapdragon 8 Gen 1 benchmarks, not directly measured)" is honest.

## When sources conflict

Two sources disagree. Common cases:

- One is current, the other outdated → prefer current, note when each was published.
- One is the primary source, the other is reporting on it → prefer primary.
- Both are credible and current → the disagreement itself is the finding. Document it. Add an open question if it matters.

Don't pick the source that confirms what you wanted to find. That is bias, not research.

## Don't search what is already known

Before doing research, check the knowledge base. If `../knowledge/02-hardware-tiers.md` already defines tiers, don't go research tiers — load that file. If something is settled (✅), trust it unless you have new evidence that warrants reopening it.

Research is for filling gaps, not for re-deriving what is already documented.

## Honest about your own limits

When research returns inconclusive results, say so. "I couldn't find authoritative numbers for X" is a valid finding. Add it to `open-questions.md`. The user can decide whether to invest in deeper investigation (e.g., buying a physical device for `[OQ-03]`).

Confidence that exceeds evidence is the failure mode to avoid. Verdure's privacy principles include "honest performance expectations" — that principle starts here, in the research that produces those numbers.

## Self-check before declaring research done

- Are my sources primary where possible?
- Is anything I'm putting into the knowledge base actually validated, or am I extrapolating?
- If extrapolating, is that visible in the text?
- Are remaining unknowns logged as open questions?
- Did I update the originating file's status marker if appropriate?
