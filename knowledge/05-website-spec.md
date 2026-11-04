# 05 — Website Spec (Phase 0)

**Status:** 🟡 The four-screen funnel below is live at `/start` but no longer the site's front door. **What the site is now is specified in "The 2026-11-04 revamp" at the end of this file** (`[OQ-87]` resolved); read the sections between as the spec of the recommender only.

This file specifies the Phase 0 recommendation website at verdureai.com. The website is the orchestration brain exposed publicly — the propositional knowledge base operationalized as a recommender. No binary, no installation, no model downloads through Verdure. Just intent-and-device → recommendation → install path.

For privacy posture, see `06-privacy-principles.md`.

## Goal

Produce a recommendation a non-technical user can act on in 60 seconds.

## User flow — four screens

**Screen 1 — Intent capture.** Plain-language input. The user types a description in their own words: "I want to write fantasy novels," "help me debug Rust code at work," "my kid needs Spanish homework help."

> **Done (`[OQ-32]`, 2026-06-01):** Screen 1 is card-based — six intent cards (`INTENT_CARDS` in `packages/decider/src/modality.ts`: Everyday questions/chat, Writing & ideas, Coding & tech help, Turn recordings into text, Create images, Languages & translation), plus a free-text fallback for the long tail. The set is grounded in `13-use-case-demand.md` (recognition over recall, per `../skills/design.md`); **no video-generation card** (weakest, least local-feasible — it stays reachable via free text and gets the honest decline). Each card carries an example phrase the classifier maps to its `(modality, task)`, locked by a unit test.

**Screen 2 — Device picker.** Curated set of 5–6 visual cards mapping to internal tiers (see `02-hardware-tiers.md` for the bucket table). Optional free-text path for typing an exact device model.

**Screen 3 — Confirmation.** Verdure shows what it understood: "We think you have: 2019 MacBook Pro 13-inch, 8 GB RAM, Intel Core i5." User confirms or corrects.

**Screen 4 — Recommendation.** Specific model with honest performance expectations, alternatives, and three install paths.

## Intent classification — LLM-powered

The user's natural-language description is sent to a cheap LLM with a structured-output system prompt. The LLM returns JSON: `intent_type`, `complexity`, `latency_tolerance`, `language`, `context_length`.

Candidate LLM APIs: Claude Haiku 4.5, Gemini Flash, GPT-5 nano, or Llama 3.1 8B via Groq. Cost is trivial — roughly a fraction of a cent per call. Order of magnitude: 100,000 users costs ~$100.

**Fallback**: if the API call fails or returns malformed output, drop to keyword matching for the obvious cases (write / code / translate / summarize). Cache common patterns.

**Future**: in Phase 1+, the intent classifier can move on-device using a tiny model like SmolLM 2 1.7B (see `03-model-matrix.md`).

## Device classification — user-asked, never sniffed

**Path A (default) — curated picker.** 5–6 visual cards covering the common cases. Each card has a brief clarifier ("about how much memory?" with 4 / 8 / 16 / not sure). Mapping table is in `02-hardware-tiers.md`.

**Path B — free text + LLM parse.** For users who click "Type my exact model." Their input is parsed by an LLM into structured specs. Show the parsed result, ask for confirmation.

**"I don't know my specs" flow**: a friendly screenshot-guided walkthrough for checking RAM on each OS. Minimum viable: "Open System Information (Mac) or Settings → System → About (Windows). Look for Memory or RAM."

No browser fingerprinting. No silent detection. The user is the source of truth. See `06-privacy-principles.md`.

## The recommendation page

Not just "install Gemma 3 4B." Honest and human:

> **For writing and brainstorming on your 2019 MacBook Air, we recommend Gemma 3 4B.**
>
> Expect about 15 tokens per second — roughly comfortable reading speed. Uses around 3 GB of memory while running. Your other apps will work fine.
>
> **Why this model?** Punches above its weight on creative tasks. Free. Runs entirely on your laptop. Conversations never leave your machine.
>
> **Alternatives:** Faster but lighter quality? Try Gemma 3 1B. Different style? Llama 3.2 3B is comparable.

## Install paths — three tiers

Every recommendation page surfaces three install options, ranked by simplicity:

1. **Easiest (coming soon)** — "Install Verdure — one-click setup." Currently a "notify me when this launches" button that captures email for the Phase 1 launch list.
2. **Easier than the status quo** — "Use LM Studio or Jan." Verdure deep-links or pre-populates the model name. Two clicks instead of a terminal.
3. **Technical but documented** — "Use Ollama." Two-command setup, exact commands copy-pasteable.

Verdure does not host or distribute models during Phase 0 — we refer to existing tools (see `10-distribution.md` for why this buys us time).

## Relationship to Jan, LM Studio, Ollama

These are **competitors for the Phase 1 binary** but **referral targets for Phase 0**. Phase 0 positions Verdure as the trusted decider; Jan / LM Studio / Ollama are the runners. The strategic objective is to ship Phase 1 fast enough that competitors cannot copy the intent-first recommender before Verdure closes the loop from recommendation to one-click install.

## Tech stack

Whatever web framework ships fastest. Next.js, SvelteKit, Astro, or plain HTML all work. Two screens, a decision tree, one LLM API call, a lookup table. Do not agonize over the choice.

## Analytics

Privacy-respecting only (Plausible or Fathom). Measure: did the user complete the flow? Did they click an install path? Did they sign up for the Phase 1 launch list? Skip vanity metrics like raw visitor count.

## Discipline guardrails

Phase 0 is easier to ship and easier to iterate on than Phase 1. The temptation will be to keep polishing the website indefinitely. **Phase 0 ships in Weeks 1–2. Phase 1 binary work starts Week 3 regardless.** The website is a means, not the end.

## Open questions

- [OQ-07] Which LLM API for intent classification? (latency, structured-output reliability, vendor stability)
- [OQ-08] Resolved (2026-05-26) — the Phase 0 domain is verdureai.com.
- [OQ-09] Tech stack pick — any reasonable choice is fine, but pick one and commit.
- [OQ-10] Resolved (2026-05-29) — launch-list capture is a Google Sheet: the `/api/waitlist` route appends one row per signup via a service account. Implemented in `website/lib/sheets.ts` (setup in `website/SETUP_SHEETS.md`).
- [OQ-11] Visual design language — needs to be defined before launch.
- [OQ-12] Spec database for the free-text device parsing path — is LLM-only sufficient for v1, or do we need to integrate with EveryMac, Notebookcheck, or similar?

## The 2026-11-04 revamp — the site is the SDK's front door

The founder's brief (2026-11-04): "we're reaching out to people now… this is the first thing they're gonna see… simple, direct, what we're doing, try out the SDK, have a sample example." Three pieces of outside feedback on the old page were folded in (a Notion thread, Sept–Oct 2026): too plain for the promise, more pages, a permanent email form, some graphics, and the beta links on the site; keep the intro and the founder visible; add real user statements once there are any (not yet — none invented).

**One job per page** (`skills/copywriting.md`):

| Page | The reader | The one action |
|---|---|---|
| `/` | A company deciding whether Verdure goes in its app | See the hello world (`/sdk#hello`) |
| `/sdk` | Their developer, with an afternoon | Email for the test kit |
| `/try` | Anyone who wants to feel it on their own device | Download for macOS; ask for an iOS/Android test build |
| `/privacy` | App-store reviewers and the careful user | Read |
| `/start/*` | The Phase 0 recommender, kept and reachable from `/try` | Pick → device → recommendation |

**What the home page says, in order:** the tagline ("Run your local models with a few lines of code."), one paragraph of what the runtime does and where, two buttons (hello world, how it works), the three things an app gets, the one picture (speech in, transcript and JSON out, all inside the device's dotted line), three measured numbers with their caveats, who it is for, the app, keep-me-posted, the founder. Nothing on it is a promise we have not measured; the "not done yet" list is one click away and linked from the numbers.

**The SDK page is the white paper**, navigable like Stripe's docs: a sticky list of nine sections, each readable in a minute, the hello world in Kotlin, Rust and Python, the full API table, the tier table, the bake-off and phone tables, licences, and what is not done. Its facts are data (`website/lib/sdk-content.ts`) with a test that every method a sample calls is in the API table, so the sample cannot drift from the SDK.

**Beta access, the third option.** iOS (TestFlight) and Android (Play closed testing) both need the tester added by hand, and a `mailto:` fails on most phones. So `/try` has one form — name, email, platform — that lands in the existing waitlist sheet with `intentText = "beta build request"`; the founder adds the tester and replies. The macOS build is a direct download from the public releases repository (`lib/links.ts`, tested to point there and never at the updater endpoint).

**Identity.** The "midnight glass" tokens from `skills/design.md` (navy, mint, glass, system faces, no webfont) are the site's CSS now; the old pages' Tailwind names are re-pointed at them in `globals.css`, so the recommender and the privacy page changed palette without an edit. `backdrop-filter` is on the header only: on the panels it washed out full-page captures in Chromium and bought nothing over a static gradient; `background-attachment: fixed` was dropped for iOS Safari.

**Not done, and known:** no analytics at all (the spec's Plausible/Fathom was never wired); the sheet is the only measure of interest; no real user statements yet; Swift/C bindings are mentioned as not packaged.
