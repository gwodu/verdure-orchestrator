# 06 — Privacy Principles

**Status:** ✅ Settled — these are foundational product values

These five principles apply across all phases of Verdure. They are non-negotiable product values, not implementation details. They rule out a class of conversion-optimization tactics that other products use; Verdure trades short-term flow optimization for long-term trust.

## 1. Ask, do not sniff

Verdure asks users for the information it needs. It does not fingerprint, sniff, or silently detect. The user always knows exactly what Verdure knows about them because they typed it in.

This rules out:
- Browser-based device detection (`navigator.deviceMemory`, `userAgent` parsing, etc.) on the Phase 0 website
- Silent telemetry that identifies the user or their device
- Any "smart default" that depends on the user not noticing

## 2. Local by default

Inference happens on the user's machine. Models never phone home. Conversations never leave the device. This is the entire reason local AI matters; Verdure protects it.

## 3. No telemetry by default

If Verdure ever collects anything — even anonymized hardware specs to improve the orchestrator — it is opt-in, explained clearly, and refusable without losing functionality.

A user who declines telemetry must get the full product. There is no telemetry-required functionality.

## 4. The user is the source of truth

When Verdure parses something the user typed (device name, intent description), it shows the user what it understood and asks for confirmation. No silent assumptions. No "we figured out what you meant" without verification.

## 5. Honest performance expectations

Recommendations include real numbers: expected tokens per second, expected memory usage, expected impact on other applications. No hype, no marketing language, no padded estimates. The user can make an informed decision because Verdure tells the truth.

## Why these matter

These principles are also a competitive position. The local-AI audience cares about privacy specifically — that is why they are running models locally instead of using cloud APIs. A product in this space that violates these principles loses the audience permanently. Verdure earns trust by aligning with the audience's reasons for being there.
