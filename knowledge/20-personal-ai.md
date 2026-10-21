# 20 — Verdure is your personal AI (build for models, not just people)

**Status:** 🟢 North-star evolution (2026-06-20). Refines `16` (utility/conductor) and `18` (one interface over models).

## The reframe
Verdure is not a personal *assistant*. It is your **personal AI**: the place where you relate to *all* your AI models, and where they relate to the rest of the world — **with you at the core of their context.** We are building **apps for AI models and agents**, not only for people. The human stays at the center; the models act on the world on the human's behalf, through Verdure, drawing on the human's context.

This deepens the mission (`18`: cheapest/safest/easiest way to use AI in daily life): the *easiest* way to use AI in daily life is one place that holds your context and lets any model act for you — locally, privately, owned by you.

## Three implications
1. **Capabilities are tools a model invokes, not just buttons a human clicks.** This is "apps for AI." Tool-calling (the chat front door, `18`) is **step one**: transcription/chat become callable tools. Everything we wrap should be exposed as a tool the model can call, not only a panel.
2. **A personal context core.** A local store of the user's stuff — dictations, preferences, history, what they're working on — that every model/agent reads and writes, keeping the user at the center. On-device, owned by the user, no telemetry (`06`). This is the "you at the core of their context" piece. New component; roadmap (`[OQ-56]`).
3. **AI-first dictation (the Whispr Flow disruptor, done right).** Not speech→text-in-a-box. Speech → **a command Verdure acts on**, and the dictation **accrues to the personal context**. A system-wide, on-screen dictation/accessibility button (like Wispr Flow's) that *also sends commands directly to Verdure* — voice as a command channel into the personal AI, not just an input field (`[OQ-57]`, supersedes the framing in `19`'s dictation note).

## Build order (foundation → vision)
- **Now: tool-calling** — the command channel. The model picks a tool (grammar-constrained), classifier cross-checks, user confirms, Verdure acts (`18`, `[OQ-55]`). This is the substrate for everything below.
- **Next: the personal context store** — local memory all models read/write (`[OQ-56]`).
- **Then: AI-first dictation** — global voice button → commands + context, built on tool-calling + context (`[OQ-57]`).

## Honest scoping
This is a large, multi-quarter vision. We earn it one CI-verified increment at a time, never overpromising — same discipline as the rest of the build. The reframe sets direction; it does not authorize building it all at once. Autonomous/agentic action still respects the "trust + reliability first" line (`17`) — grammar-locked tools + a confirm step keep the human in the loop.

## Open questions
- [OQ-56] The personal context store: what it holds, on-disk shape, how models read/write it, privacy model (`06`), and how it stays user-owned and portable. **→ `25-personal-context.md` (2026-10-21): the store exists; messages are source one.**
- [OQ-57] AI-first dictation: a system-wide dictation/command button (accessibility surface) that feeds both transcription→context and commands→Verdure. Builds on tool-calling (`[OQ-55]`) + context (`[OQ-56]`).
