# 26 — The Verdure SDK: one runtime, vendors' surfaces

**Status:** 🟡 Provisional (decided 2026-10-21 from the founder's nine-vendor teardown and SDK spec; the first crates exist, nothing has run on a customer's device). Companion to `18` (one interface over local models) and `20` (personal AI); the structure it builds is `14`'s hexagon, finally real.

This file answers: **what does Verdure sell to the companies whose apps clinicians, teachers and health workers already use, and how is that runtime built so it stays one brain under many surfaces?**

## The three people it must serve

The founder's framing (2026-10-21): every decision in the SDK comes back to three users, in this order of distance from the code and reverse order of who matters most.

- **The end user** — the clinician dictating a note between patients, the teacher whose lesson is being recorded, the community health worker (CHW) filling a form on a 4 GB phone after a home visit. They never see Verdure. What they get is the vendor's app working offline, faster, without their voice leaving the room. Every honest-numbers and never-invent rule below is for them.
- **The developer** — the vendor's engineer who has an afternoon. One install line, one `start()`, the same call shape as the cloud API they already use (a session, audio in, transcript events out, then structured output). Errors that say what to do. Nothing to learn about models.
- **The customer** — the vendor's business and compliance people. Local by default resolves their retention contradictions (Nabla's help centre and a hospital FAQ disagree about whether audio is kept); Verdure holds no data and so is not a processor; the licence is an offline token; the model licences are listed, not discovered.

## The decision

Verdure goes to its first customers with **one product**: a single Rust runtime, installed with one package-manager line, that does **speech → clean transcript → schema-validated structured output** on any device tier, with a **vendor-controlled cloud fallback**. Local retrieval is added only where the corpus already ships on the device.

The research found this exact pipeline running today in someone else's cloud at every target: Nabla, Corti and Suki expose it as streaming-WebSocket APIs; Nabla has moved its own dictation on-device (Mac, M1 Pro and later only); Medic's CHT team has publicly written off on-device models as "not viable" on field phones. That gap — Windows, Android, older Macs, 4 GB phones — is the wedge.

**Three things Verdure must do very well** (Part 3 of the spec):

1. **One-line, cross-platform install with hardware tiering and model delivery.** `start(licenceKey)`; `recommend(task)` picks the bundle for the device's tier; weights stream from the vendor's CDN on first use, never from inside the app binary (Play's 1.5 GB asset-pack and Apple's 4 GB app limits, Part 2).
2. **On-device ASR → cleanup → `extract(schema)`.** This is the product. The vendor hands over *their* schema (note sections, facts, XForm fields, a lesson-observation record) and gets valid JSON or an error that names the problem. Keyterms and replacements mirror Corti's features so a session config maps one-to-one.
3. **Local RAG only over device-resident corpora.** Kolibri channels, uLesson lesson packs, NewGlobe teacher guides, CommCare/CHT form definitions and guideline text, the clinician's own prior notes. Server-side corpora (Corti interactions, Heidi patient context, OCS indexed collections, Gemini/Drive, OpenEvidence) are out of scope by rule.

**Verdure does not build:** EHR integrations or FHIR write-back, note templates, curricula, consent UX, vendor UI, chat front-ends, billing or metering beyond the offline licence check, telemetry, or its own cloud LLM. The vendor's cloud is the fallback, through a hook they implement; Verdure never opens that socket.

## How this sits with what came before

- **`18` said: build exactly one surface (the Verdure app) over headless engines.** The SDK extends that, it does not reverse it: the brain is still one, the engines are still conducted rather than built, and the app remains the consumer surface. What changes is that *other people's surfaces* can sit on the same brain. `CLAUDE.md` already named the monorepo "surfaces over one brain"; this is the second surface. The doctrine that does not move: never a model, never an engine, never a UI for a vendor.
- **`16` (growth): the atomic unit** was "one everyday task done with a local model, in one place." For the SDK it is **one vendor integration that works offline on a device their cloud never reached** — measured per device tier, not per seat.
- **`06` (privacy)** is the default policy as data: `Policy::default()` = downloads only, telemetry off, encrypted sandbox, session-only retention, fallback disabled. "Ask, don't sniff" and the device probe were reconciled in `14`: a user-installed, local-only runtime reading its own host to size models sends nothing anywhere and reads no identifiers.
- **`14` (architecture)** described a pure core with replaceable adapters at seams; the MVP app was a flat crate growing toward it. The `crates/` workspace *is* that hexagon: `verdure-core` has no I/O and compiles for every target; the engines, the device probe, the downloader and the stores implement its ports. `14`'s seam table maps onto it: `Backend` → `LlmEngine`/`AsrEngine`, `DeviceProbe` → `verdure-hw`, `ModelSource`+`ModelStore` → `verdure-download` + the bundle directory, `Executor` → still in-process (`[OQ-58]`), `Transport` → UniFFI bindings in-process (the daemon's IPC remains future).
- **`03`/`12` (which model)**: the SDK's bundles are a *different registry* from the app's `model.rs`, chosen per task and tier and hosted by the vendor. Until `[OQ-98]` settles which ByteShape builds Verdure stands on, and until the gates below have run, every bundle is **example only** (`bundles/manifests/example.json` says so in its own `notes`).

## Reconciliations — where the spec and the knowledge base disagreed, and what won

Each of these was decided while building, and recorded here because silent divergence is the failure mode `CLAUDE.md` names.

1. **Tier names.** The spec labelled devices T0/T1 (phones) and L0/L1/L2 (laptops). `02` already owns a tier vocabulary, and the code must share one language with the knowledge base. **`02`'s names win**: T0 → `P1`, T1 → `P2`/`P3`, L0 → `L1`, L1 → `L2`, L2 → `L3`. The mapping lives once in `verdure-core/src/tier.rs` and here. One refinement from the spec kept: an Apple-Silicon phone is lifted one tier (an iPhone 12 at 4 GB is T1 in the spec's lab, and `classify` agrees), because an A-series chip at 4 GB does what a Unisoc does at 6.
2. **The memory rule.** The spec proposed `available ≥ size × 1.3 + 300 MB`. The app measured a 1211 MB model peaking ~1 GB above its file size (a `CPU_REPACK` copy of the weights plus a compute buffer, `03` 2026-09-10) and gates its 1.4 GB 4B at 2300 MB; the spec's rule would have admitted that model with less than it needs and let ggml abort the host app. **The rule shipped is `× 1.5 + 400`**, which covers the measurement and stays above the app's gate. Measured beats proposed; the test in `memory.rs` says why.
3. **The ASR engine.** The spec names sherpa-onnx (Parakeet, Moonshine, SenseVoice) as "Verdure's ASR engine", with whisper.cpp as the multilingual fallback. whisper.cpp is the engine the app has run on macOS, iOS and Android for months; sherpa-onnx is a new native crate, which costs a CI dispatch to prove and is the founder's call under the CI-prudence rule. **whisper.cpp is the first `AsrEngine`; sherpa-onnx is the second**, behind the same port, `[OQ-107]`. Nothing above the port changes when it arrives.
4. **Grammar-constrained extraction.** The spec's `extract()` constrains sampling with a GBNF grammar. The app *removed* llama.cpp's grammar sampler after it `ggml_abort()`ed the process on a multi-byte token (2026-06-23, `18`), and an abort takes the vendor's app down. **Shipped: schema-in-prompt + JSON validation + one repair retry, with the grammar sampler built but off by default** (`ExtractOptions::use_grammar`; the engine refuses a grammar unless loaded with `allow_grammar`). The ≥ 99.5 % schema-valid gate must be measured on the prompt+repair path first; the grammar returns under process isolation or a verified-safe path, `[OQ-108]`. The converter itself (`verdure-grammar`) is pure and tested, and renders *every* declared property in schema order with optional ones nullable — a fixed layout a 1–4B model handles better than "any subset, any order".
5. **Synchronous API, not `async fn`.** The spec's Rust signatures were async. UniFFI's async needs a runtime in every binding; v0.1's calls block and the host calls them off its UI thread, as it would file I/O. Smaller, portable, and the Rust does not change when async is layered on later.
6. **Who signs licences.** Verdure issues tokens (`verdure license issue`); the vendor's build carries the issuer's public key and verifies offline. The development key pair exists (public half committed in `crates/verdure-ffi/keys/issuer.pub`, secret in `~/.verdure/issuer.key` on the founder's Mac, never in the repo). Government fleets with thousands of devices and no per-seat billing are `[OQ-105]`; `start_with_issuer` exists so a vendor can run their own issuer.

## The targets (from the founder's research, 2026-10-21)

Nine vendors in three groups, chosen for how much of their stack is public and how well they fit a licensing model. Vendor-reported figures are flagged as such in the research and are not independently audited; several facts (Nabla's and Suki's LLM providers, uLesson's and NewGlobe's stacks, Intron's "offline" meaning batch) are *unverified* and must not be acted on without follow-up.

| Group | Vendor | What they run today | Where Verdure slots in |
|---|---|---|---|
| Scribes | **Nabla** (Paris/NY) | WSS transcribe + note generation in their cloud; on-device dictation on M1 Pro+ Macs only; Electron/Swift/Rust stack | Offline/sovereign mode on Windows, Android, Intel Macs: `transcribe()` + `extract(NoteSchema)`; their cloud keeps the final note, coding, EHR sync |
| Scribes | **Corti** (Copenhagen) | `/streams` WSS (transcripts ~3 s, FactsR facts ~60 s), EU-hosted open-weight "Corti Models", public SDKs | Offline facts via `extract(FactsSchema)` every 60 s, uploaded on reconnect through their existing "Add Facts" endpoint — no server change |
| Scribes | **Suki** (Redwood City) | Web/headless-React/iOS SDKs, Form Filling SDK; Android SDK "coming" | Offline form fill on iOS and the Android parity they lack |
| Education | **Kolibri** (Learning Equality) | Python/Django, SQLite, content on the school laptop/LAN server; offline AI "in development" | A Python plugin: `index()` channels at import, `query()` + `chat()` for "explain my answer" — needs a **Python binding**, `[OQ-104]` |
| Education | **uLesson** (Abuja) | Offline lesson packs (SD-card heritage) + cloud "AI homework help" | Homework help in airplane mode: `query()` over downloaded lessons + a 1–3B tutor |
| Education | **NewGlobe** | Teacher tablets with guides preloaded; May 2026 "Enterprise AI" incl. coaching from real lessons | On-tablet `transcribe()` + `extract(ObservationSchema)`; upload the features, never children's audio — the strongest privacy story |
| CHWs | **Dimagi** (CommCare + Open Chat Studio) | Cloud LLMs, multi-provider; XForms and case data on the phone | "Speak to fill the form": `extract(schema from the XForm)` on an Itel A70 |
| CHWs | **Medic CHT** | Offline-first JS + Android container; said local models "aren't viable" on field devices | The benchmark that answers them: a ≤ 1.5B model doing narrow form extraction on a 4 GB phone |
| CHWs | **Intron Health** (Lagos) | Sahara ASR, best African-accent WER, cloud API | A **model partner**, not a runtime customer: a distilled Sahara-small shipped as a Verdure bundle under their licence, `[OQ-103]` |

Baselines the ease-of-use must match: Apple's Foundation Models (`LanguageModelSession` + `@Generable`, three lines, local/cloud swap) and Android ML Kit GenAI (flagship-only — none of the 4–8 GB target phones qualify, which is the opening).

## What is built (2026-10-21)

The `crates/` workspace at the repo root replaces the empty `src/` scaffold. Nine crates and a CLI; the pure suite runs in seconds with no model on disk.

| crate | holds | tests |
|---|---|---|
| `verdure-core` | `Policy` (defaults = `06`), `VError`, events + cancellation, `Tier`/`DeviceProfile`/`classify`, `Task`, the memory rule, `Manifest`/`Bundle`/`resolve()` with skipped-reasons, Ed25519 `signing`, licence `Issuer`/`Verifier`, the engine ports (`AsrEngine`, `LlmEngine`, `Embedder`, `CloudFallback`), `extract()` | 27 |
| `verdure-grammar` | JSON Schema → GBNF (`to_gbnf`), `validate` with path-addressed violations, `strip_optional_nulls` | 14 |
| `verdure-hw` | the per-OS probe (macOS `vm_stat` formula checked against Activity Monitor, iOS jetsam headroom, Android budget, Linux `/proc`, Windows via `sysinfo`) | 4 |
| `verdure-download` | the only socket: signed manifests, resumable range downloads, SHA-256 over the whole file, atomic rename, cancellation that keeps the part file | 7 (against an in-process HTTP server) |
| `verdure-vault` | AES-256-GCM files with the name authenticated, `FileKeyStore`, crypto-shred | 5 |
| `verdure-rag` | sentence/paragraph chunker, SQLite FTS5 BM25 + brute-force cosine, reciprocal-rank fusion | 6 |
| `verdure-asr` | `Segmenter` with timestamps, resampling, keyterms prompt, whole-word replacements, non-speech cleanup; `whisper` feature | 9 |
| `verdure-llm` | `PromptStyle` (ChatML, no-think, Gemma), `format_chat`, context sizing and history fitting; `llama` feature with grammar opt-in | 7 |
| `verdure-ffi` | the UniFFI surface: `Verdure` (`start`, `capabilities`, `set_bundle_source`/`load_manifest`, `recommend`, `fit`, `download`, `is_downloaded`, `load`/`load_overcommit`, `transcribe` → `AsrStream`, `chat`, `extract`, `index`, `query`, `delete`, `set_cloud_fallback`), callback traits `ProgressListener`/`TokenSink`/`CloudFallback` | 5 |
| `verdure-cli` | `verdure capabilities | license keygen/issue/verify | manifest check/sign/verify | recommend | grammar` | smoke-run on the Mac |

Verified on the founder's Mac: keygen → issue → verify; `manifest check/sign/verify` on `bundles/manifests/example.json`; `recommend --task scribe` honestly refuses on this 8 GB machine with 2.4 GB free ("needs 2562 MB of memory, 2371 MB available") and picks `chw-forms-P1` for a simulated P1 phone; `grammar` renders the CHW schema with six validator-only notes.

**Not built, in order of what unblocks the first demo:** the engine feature builds verified anywhere (`[OQ-112]`); a CI job for `crates/` (added with this change, first run on the next PR or dispatch); the app migrating onto `verdure-llm`/`verdure-asr` so there is one copy of the prompt and context logic; sherpa-onnx; packaging of the bindings (XCFramework, AAR, wheel, napi); the evals harness and the device lab (`[OQ-113]`); Flutter/React Native/Node shells; the Kolibri Python plugin.

## Who chooses the model — three hands on the dial (added 2026-10-24)

The founder's question: should a developer be able to enforce a model of their choice? Yes, and the design gives three parties control at three moments, each honest about the one below it.

1. **The vendor chooses the menu.** The manifest is theirs: which models, which files, which tiers, which licences. Verdure never adds a model a vendor did not list. Writing a manifest with one bundle *is* enforcing one model.
2. **The developer chooses from the menu, or lets Verdure choose.** `recommend(task)` walks the device's tier down and substitutes a smaller bundle with reasons. `fit(bundleId)` judges exactly the bundle the developer named — no substitution — and returns the numbers and every problem as a sentence. `load(bundleId)` keeps the memory gate; `load_overcommit(bundleId)` removes it for a developer who has measured their own device, with the documented risk that the engine may abort the host process until `[OQ-58]` lands. The licence applies in every path.
3. **The device has the last word on what is true.** `fit` and `recommend` read the same `problems()` function, so the walk and the pin cannot disagree about what fits; the numbers come from the live probe, never a spec sheet.

What is deliberately *not* offered: a way to run a model the manifest does not list, or to skip the licence. Those are the vendor's controls, and a runtime that let a developer bypass them would make the compliance story false.

## How we test the developer story before a company sees it (2026-10-26)

The founder's method: write the integration instructions as a self-contained paper, hand it to a deliberately weak model (Haiku), and watch it build a project. If the paper has a gap, a small model falls straight into it, which is exactly what a hurried engineer would do. The paper is `docs/sdk-integration-guide.md`; it contains only steps that were run on this repo before being written down: the Rust route (`sandbox/template`, copied and built; `crates/verdure-ffi/examples/chw.rs`, the whole flow in one file), the Python route (bindings generated from the built library and the snippet run verbatim), the sandbox credentials (`bundles/sandbox.token`, ten years, vendor `sandbox`, signed by the dev issuer whose public key is compiled in), and a test assignment with acceptance criteria (§9). Projects go under `sandbox/`, which the root workspace excludes, so a broken experiment cannot break the suite; only the template is committed.

What the test scores: did the project compile; did it use `fit`/`recommend` rather than guessing; did it handle `EngineUnavailable` honestly or pretend; did it try to download the example bundles (the paper says not to); where did it stall. Every stall is a sentence missing from the paper, and the fix goes into the paper first and the API second, because the API is what a thousand developers will read through the paper.

## The first real run (2026-10-27) — what the SDK did with real speech on the Mac

The engines compiled into the SDK on this Mac after all: `CARGO_BUILD_JOBS=2 cargo build -p verdure-ffi --features llama,whisper`, 5 min 39 s cold, once the disk had 4 GB free. `[OQ-112]`'s "a second build does not fit" was a disk-state fact, not a permanent one. The Python binding regenerated from that library ran a PriMock57 consultation (Babylon Health's mock GP consultations, CC BY 4.0, 7.6 minutes, two speakers mixed into one stream) through the whole chain: `recommend(scribe)` chose `scribe-floor` (Whisper base + Qwen2.5 0.5B, the only bundle that fits 1.8 GB of free memory, and said why the L2 bundle was skipped), `load`, `transcribe`, `extract`. The harness is `evals/`.

**Speech (Whisper base, the floor):** real-time factor 0.24 on the 2019 Intel CPU, first words 2.6 s after audio began, word error rate 27.6 % against the human transcript (1,321 reference words). Two voices mixed, overlapping speech, "diarrhoea" heard as "die rule"; the medical-term error rate the gate wants (≤ 12 %) is not computed yet because the reference spells numbers as words and the term list does not exist. Whisper small is the next thing to measure.

**Extraction (Qwen2.5 0.5B, the floor):** the first attempt failed completely and taught three things that are now in the SDK. (1) Pasting the JSON Schema into the prompt made the model copy it back, `$comment` and all, until its budget ran out — so the model now sees an *outline* with placeholders (`verdure_grammar::shape`) and the schema is used only for validation. (2) It then copied the outline's placeholders — so a value still shaped like `<text>` is a named violation the repair turn can quote. (3) A one-line demonstration in the system prompt moved it from 0 of 9 windows valid to 4 of 9. Extraction runs per ~60 s window of transcript, as the platforms themselves emit facts, not over a whole consultation. The four windows' facts are transcript quotes with a plausible group; zero invented numbers. **Verdict: the 0.5B cannot do clinical fact extraction; it was never expected to.** The honest floor for `extract` is the 1.5B–2B class, which needs ~2.2 GB free with the speech model beside it — more than this Mac has with a browser open, and well within the iPad's and a P2 phone's budget. That is the next measurement, and it is the same finding the app made for grounded answers (`03`, `[OQ-97]`).

What the run proved regardless of model quality: the device was read honestly, the only bundle that fit was chosen with the refusal explained, the files were verified by hash, both engines loaded within the memory rule, audio streamed through the segmenter with timestamps, every extraction result was either schema-valid JSON or an error that named the exact problem, and the whole thing ran with Wi-Fi irrelevant. Numbers are in `evals/out/*.score.json` (git-ignored) and summarised in `TEST_REPORT.md`.

## The Android deliverable (2026-10-29)

`.github/workflows/build-sdk-android.yml` (tag `sdk-android-N` or dispatch; Linux, 1×) produces `verdure-android-arm64.zip`: `libverdure.so` for arm64 with both engines (8.1 MB stripped, 16 KB page aligned, API 26), the UniFFI Kotlin binding (compiled in CI against JNA 5.14 as a drift check), the sandbox licence, the signed manifest, the schemas, and `bindings/android/README.md` with the exact files-to-copy and Gradle lines. Three runs to green, all at 1×: run 1 failed at link on the duplicate ggml symbols (`[OQ-114]`), run 2 on my own wrong assertion (looking for engine symbols in a stripped library's export table), run 3 green in 12 minutes, 9 of them the cross-compile. The first artefact is at `~/Verdure-bisect/sdk-android/` on the founder's Mac. **Nothing has run on a phone yet**: the first Pixel run is the swap test with the Corti session's Android client, and it is where `[OQ-114]` gets its answer.

## The first vendor-side integration report (2026-10-30)

The Corti-session's Android client (built by someone who had never seen Verdure) took the zip and reported: file copying trivial, Gradle two lines, API "identical to Rust/Python patterns", docs "excellent and complete" — and the app died at startup: `UnsatisfiedLinkError … library "libc++_shared.so" not found`. Our library links the Android shared C++ runtime (the sys crates request `c++_shared`), the zip did not ship it, and the README did not mention it. On a machine with the NDK a developer copies that file by reflex; this one had no NDK, which is exactly the developer the three-audiences rule cares about. Fixed in the workflow (ship the NDK's `libc++_shared.so` beside ours, assert the `NEEDED` entry and the file) and in the README (copy both; a troubleshooting section for the three `UnsatisfiedLinkError` shapes). The report's difficulty rating was 3/4, with the C++ runtime as the one blocker; the rest it estimated at ~300 lines. Everything else in the report (silent try/catch around `start`, the architectural questions about background kills and unloading on a 4 GB phone) is written into the README's guidance or into `[OQ-109]`.

## The first phone run (2026-11-03, Pixel 8a, the vendor's own Android client)

The Corti-shaped Android client, built by a session that had never seen Verdure, took the second zip and ran the whole chain on a Pixel 8a (Android 15): `start` 82 ms; `recommend(SCRIBE)` → `scribe-floor`, nothing skipped on a P3 phone; 639 MB downloaded in 26 s over Wi-Fi and verified; `load` 1.24 s; offline transcription at roughly 0.5× real time (4–8 s of whisper per 12 s window; first segment 17 s after the stream opened); three extractions of 21–33 s each; facts posted back to the platform on reconnect, nothing lost or duplicated; the online path untouched. Memory: 683 MB PSS idle after load, 952 MB peak during `extract`. Two ggml copies gave no trouble across four visits (`[OQ-114]`, concurrent use untried). Rating from the integrator: 3 of 4.

**What it found, and what changed because of it.**
- *The Android memory budget ignored our own footprint*: `capabilities()` reported 4543 MB free before and after loading 1.3 GB, because the budget was 60 % of total with no deduction for this process. Fixed: the budget now subtracts the process's resident set (`/proc/self/statm`), never below `MemAvailable`; tested with the Pixel's numbers.
- *Threading was undocumented*, so the client serialised ASR and extraction on one thread and the transcript froze for 20–35 s during every extraction. The contract is now written at the top of `verdure-ffi`, in the guide and in the Android README: `Verdure` and each `AsrStream` are locked separately, so transcription and extraction may run on two threads; two `extract` calls queue inside the engine (a lock added to `LlamaEngine`, so two concurrent generations cannot double the memory).
- *`Extraction`'s fields, the retry policy, and why `used_grammar` is false* were not in the docs. They are now, with `[OQ-108]` named.
- *The guide was stale about bundles* (still said "placeholder, do not download") — corrected to name the one bundle that is a placeholder.
- *`onStatus` never fires on a clean download* — documented as by design.
- *Background advice contradicted a foreground service* — rewritten: the engines belong to the `Verdure` object; keep it alive as long as the service records; `load` after a drop is 1.2 s.
- *`UnsatisfiedLinkError` is an `Error`*, so `catch (e: Exception)` misses it — README says catch `Throwable`.
- Two findings that are the next work, not this fix: fixed 12 s windows cut sentences and lost one stretch (`[OQ-117]`); the 0.5B copies text instead of extracting facts, 20–35 s a window (`[OQ-118]`). Both were already known from the Mac run; the phone confirms them with numbers.

**The quality picture, honestly:** Whisper base on a two-voice TTS fixture got the drug name right once in two, wrote "1.50/95" for a blood pressure, hallucinated two clauses, and dropped one 12 s stretch. That is the smallest speech model; the next bundle (Whisper small, 1.5B) is a manifest entry, and the phone has the memory for it.

## Rules for anyone adding to the crates

(a) All public types live in `verdure-ffi`; no platform `#[cfg]` inside an export block. (b) Every engine sits behind a port in `verdure-core`. (c) No network code outside `verdure-download`, except `CloudFallback`, which calls *into* vendor code. (d) Every long operation reports progress and is cancellable. (e) Measured numbers beat proposed ones — when a rule is changed, the test states the measurement it covers.

## Model matrix v1 (provisional — targets, not measurements)

In `02`'s names. Licence posture is `09`'s: Apache-2.0/MIT by default; Llama and Gemma bundles ship only with the vendor's acknowledgement recorded in the licence token (`requires_acknowledgement`); CC-BY Parakeet carries an attribution string exposed through `capabilities().attributions`.

| Tier | Example devices | ASR | LLM (GGUF Q4_K_M) | Embedder | Target |
|---|---|---|---|---|---|
| P1 (spec T0) | Itel A70 (Unisoc T606), Tecno Spark Go | Whisper-base (multi) now; Moonshine-tiny (EN) with sherpa | Qwen2.5-1.5B-Instruct (Apache-2.0), ~1.1 GB | potion-base-8M / multilingual-e5-small | ASR RTF ≤ 0.5; 6–10 tok/s |
| P2/P3 (T1) | Helio G85/G99, Dimensity 6300, iPhone 12+ | Whisper-small now; Parakeet-tdt-0.6b-v3 int8 (CC-BY-4.0) with sherpa | Llama-3.2-3B (community) or Qwen2.5-3B (**Qwen research licence**, `[OQ-106]`) | same | RTF ≤ 0.3; 8–15 tok/s |
| L1 (L0) | Intel i3/i5 gen 6–8, 4 GB | Whisper-base | Qwen2.5-1.5B | same | 8–12 tok/s |
| L2 (L1) | refurb Intel MacBook, school-lab PC, 8 GB | Whisper-small | Phi-4-mini 3.8B (MIT) or Gemma-3-4B (Gemma terms) | bge-small | 12–18 tok/s |
| L3 (L2) | M1+, dGPU, 16 GB+ | Whisper-large-v3-turbo | Qwen3-8B or Llama-3.1-8B | bge-m3 | 20–30 tok/s |

**Release gates (a bundle cannot be signed for a customer until it passes; none has run — `[OQ-113]`):** Scribe — medical-term error rate ≤ 12 % on a MedWER-style set, AfriSpeech-clinical WER within 2 points of fine-tuned Whisper-small, ACI-Bench major hallucinations ≤ 5 % of cases, `extract()` schema-valid ≥ 99.5 %. Tutor — GSM8K ≥ 70 % (L2) / ≥ 55 % (P1), safety red-team, RAG groundedness ≥ 90 % on a Kolibri golden set. CHW — field-level F1 ≥ 0.90 on 200 dictated summaries per form, **zero fabricated numeric vitals** (the extract prompt's first rule).

## Platforms, memory, thermal

iOS 16+, Android 8.0 (API 26, arm64, 16 KB-page ready), macOS 12, Windows 10 21H2 x64, Linux glibc 2.31. Backends per platform and the background-audio, foreground-service and signing specifics are **unverified** and are acceptance gates to measure, not numbers to assume (`[OQ-109]`). Implemented: the memory rule before every load, the tier downgrade with reasons, Intel-Mac CPU pinning carried over from `[OQ-71]`. Planned: unload the LLM (keep ASR) on a memory-pressure signal, halve threads and emit `Throttled` on serious thermal state, a 4k default context capped by tier.

## Privacy and security — enforced vs. promised

Enforced in code today: downloads only (and only https, host-allowlisted when the vendor lists hosts), decided in one place (`Policy::permits_download`) that `verdure-download` must ask; a manifest's bytes are untrusted until its Ed25519 signature verifies; every downloaded file is hashed whole; the licence check is offline; telemetry is a type with `Off` as default and no code path that uploads; transcripts and caches go through the vault; `delete(All)` destroys the key before unlinking; the cloud fallback is vendor code and its output is validated against the schema like the local engine's.

Promised, not yet built: the CI test that runs the suite inside a network-denied sandbox; platform key stores (`[OQ-110]`); SQLCipher for the retrieval index (`[OQ-99]`); the vendor compliance pack (data-flow diagram, "Verdure is not a processor" statement — vendor counsel must confirm, storage paths, CycloneDX SBOM, model cards and licences, DPIA/BAA/COPPA templates).

## Integration recipes (estimates from the research; verify with each vendor)

Nabla ~60 LOC (Electron audio module + transcription client; demo: Windows laptop, Wi-Fi off, 5-minute visit → draft note). Corti ~80 (stream adapter; demo: network drops mid-visit, no facts lost). Suki ~70 (ambient session service; demo: Android offline form fill). Kolibri ~150 (new plugin; demo: import a channel, "explain my answer" offline). uLesson ~50 (homework screen; airplane mode). NewGlobe ~80 (lesson-recording module; 10-minute lesson → structured coaching features, no audio uploaded). Dimagi ~90 (form entry; Itel A70 ANC visit → prefilled XForm). Medic CHT ~100 (`cht-android` bridge + JS shim; the benchmark). Intron n/a (export pipeline; side-by-side cloud vs on-device on EHA audio).

## Open Questions

- [OQ-103] Intron as a model partner before the first scribe customer — the only credible route to on-device African-accent clinical ASR at quality (Afonja et al.: fine-tuning on accented clinical speech improves medical WER 25–34 % relative). Founder decision.
- [OQ-104] Python as a first-class binding, for Kolibri. Not in the original binding list; the research recommends it; UniFFI generates it from the same surface, so the cost is packaging (maturin wheels), not API.
- [OQ-105] Licence tokens for government fleets (NewGlobe-scale): who signs, what a seat is, offline issuance for thousands of tablets. `seats: 0` and `start_with_issuer` exist; the model does not.
- [OQ-106] Qwen-family licences through vendor legal review (Qwen2.5-3B is under the Qwen research licence; 1.5B and Qwen3 are Apache-2.0), or default v1 to Phi-4-mini / Llama with the acknowledgement rule.
- [OQ-107] sherpa-onnx as the second `AsrEngine`: a new native crate (a CI dispatch to prove, per the CI-prudence rule — the founder's call), Parakeet's CC-BY attribution, Moonshine's non-English non-commercial legacy models, Android QNN later.
- [OQ-108] Grammar-constrained extraction vs. the abort evidence: measure the schema-valid rate of prompt + validate + one repair on real bundles first; re-enable the grammar sampler only under process isolation (`[OQ-58]`) or after the multi-byte-token failure is shown fixed in the pinned llama.cpp.
- [OQ-109] Unverified platform constraints the spec listed as spikes: iOS jetsam ceiling for a 3B, background-audio rules, Android foreground-service type `microphone`, thermal curves, Windows/macOS signing of the dylib/.node, discrete-GPU detection on Linux/Windows (the probe reports `integrated`).
- [OQ-110] Key storage for the vault: `FileKeyStore` is as strong as the app sandbox; Keychain/Keystore/DPAPI/libsecret need platform code behind the `KeyStore` port. The retrieval index is a plain SQLite file until `[OQ-99]`'s SQLCipher decision.
- [OQ-111] Retrieval at scale: brute-force cosine is right for thousands of chunks; `sqlite-vec` when a corpus outgrows it; an embedder bundle (potion-base-8M, bge-small) and the groundedness gate — ties to `[OQ-101]`.
- [OQ-112] **Built on the Mac 2026-10-27** (5 min 39 s at two jobs, 4 GB disk free needed); still open: the app adopting `verdure-llm`/`verdure-asr`. Original note — the engine feature builds had compiled nowhere: a second llama.cpp + whisper.cpp debug build does not fit this Mac's disk (the shared-target-dir experiment rebuilt both from scratch — different fingerprints — and was stopped at 1.1 GB free). CI's new `sdk` job checks them on Linux at 1×; the app's migration onto `verdure-llm`/`verdure-asr` is the same question from the other side.
- [OQ-119] Google MedASR (105M Conformer, dictation-trained, HAI-DEF terms, community GGUF on `transcribe.cpp`): measure on PriMock57 with Transformers first; licence is acknowledgement-class plus a regulatory clause.
- [OQ-117] Segment windowing on a phone: 12 s force-cuts cut sentences, one stretch dropped; VAD or a configurable window, measured on PriMock57 first.
- [OQ-118] Phone extraction: 20–35 s per window on the 0.5B and it copies text; the 1.5B bundle next; streaming tokens; the platform has no `client` fact source.
- [OQ-114] Two ggml copies in one library (whisper.cpp's and llama.cpp's): Android's linker is told `--allow-multiple-definition`; verify both engines on the Pixel; the real fix is one shared ggml.
- [OQ-113] **First measurements 2026-10-27** (one consultation, the floor bundle, this Mac): ASR RTF 0.24, WER 27.6 %; 0.5B extraction 4/9 windows valid, zero invented numbers. Everything else is still unmeasured. Original note: the release gates, the device lab (Itel A70, Tecno Spark Go, Galaxy A05; Infinix Hot 40, Redmi 13C, A16 5G, iPhone 12/13/15; T460; 2017 MBP, ProBook 450; M1 Air), the `evals/harness` CLI replaying WAV + reference JSON per bundle per device. `[OQ-74]`'s harness is the vehicle; `[OQ-03]` is the device access.
