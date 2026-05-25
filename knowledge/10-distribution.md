# 10 — Model Distribution

**Status:** 🔴 Open — many decisions deferred until Phase 1 begins

This file documents how Verdure delivers model files to user devices. Phase 0 does not distribute models — the website refers users to existing tools (see `05-website-spec.md`). This buys time to resolve the questions below before Phase 1.

## Source of truth

**Hugging Face Hub** is the primary source. Pre-quantized GGUFs (see `04-quantization.md`) are published by official model authors and by community accounts (`bartowski`, `unsloth`, historical `TheBloke`).

## Concerns to resolve before Phase 1

**Bandwidth and rate limits.** Hugging Face may not be sustainable as a source if Verdure has 100K+ users each pulling 2 GB models. **[GAP — needs research]** Current HF TOS for high-volume programmatic downloads and any rate limits at scale. Tracked as `[OQ-19]`.

**Geographic latency.** A user in Lagos pulling 2 GB from a US-hosted CDN over a 3G connection is a multi-hour download with a high failure rate. Regional mirrors (CloudFlare R2, BunnyCDN, others) may be necessary. **[GAP — needs research]** CDN economics at scale for the top 20 models globally. Tracked as `[OQ-20]`.

**Resume and integrity.** Flaky networks are the norm in emerging markets. Verdure needs chunked downloads, SHA256 verification, automatic resume on failure. Standard but must be specified.

## Catalog approach

Two extremes:

- **Curated catalog** — Verdure ships an approved list of models, tested, ranked, and known to work at each hardware tier. Safer for users, smaller surface area for breakage, license-clean.
- **Open marketplace** — any GGUF on Hugging Face works. More flexible, more breakage risk, harder to support.

**Likely answer**: curated by default, open as advanced option. Most users get the safe path; power users can paste a Hugging Face URL.

**[GAP — needs decision]** Governance for the curated catalog — who decides what is in, what evaluation criteria, what version-pinning policy. Tracked as `[OQ-21]`.

## Model metadata schema

Verdure needs canonical descriptors for each model:

- Model name (display + canonical ID)
- Parameter count
- Quantization level (e.g., Q4_K_M)
- Footprint in memory and on disk
- Expected throughput per hardware tier
- License (see `09-licensing.md`)
- Source URL
- SHA256 checksum
- Intent affinities (creative, code, multilingual, etc.) — for the intent-shift logic in `03-model-matrix.md`

**[GAP — needs decision]** Whether to extend Hugging Face's existing metadata format or build a Verdure-specific schema. Tracked as `[OQ-22]`.

## Open questions

- [OQ-19] Hugging Face TOS at scale — rate limits, bandwidth costs, any usage-tier requirements.
- [OQ-20] CDN economics for mirroring top 20 models globally — providers, costs, latency comparisons.
- [OQ-21] Curated catalog governance — selection criteria, version pinning, update cadence.
- [OQ-22] Model metadata schema — extend HF format or roll our own.
