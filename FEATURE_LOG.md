# FEATURE_LOG.md

Living record of features built across the Verdure monorepo — the **Verdure desktop app** (`app/`), the shared decider package (`packages/decider/`), the Phase 0 website (`website/`), and the long-term Rust core (`src/`, `tests/`). Newest first. The knowledge base (`knowledge/`) is the shared brain for all of them.

## Verdure SDK (`crates/`) — one runtime, vendors' surfaces

> Started 2026-10-21 from the founder's nine-vendor teardown and build spec (`knowledge/26`). The long-term Rust core, real: `verdure-core` pure, I/O crates behind its ports, `verdure-ffi` the public surface. Three users in every decision: the end user who never sees Verdure, the developer with an afternoon, the customer's compliance team.

### 2026-11-03 — `scribe-small`: the measured speech model becomes the bundle the SDK recommends, with base kept as the fallback and the developer's pick (`knowledge/26`, "What shipped from it")

**What:** a new `scribe-small` bundle (Whisper small + Qwen2.5 0.5B, real hashes) leads the signed example manifest for P3/L2/L3; `scribe-floor` stays for every tier as the fallback and the everyday-dictation choice; `scribe-L2` gets a real hash for its 1.5B. No crate code changed — the tier walk, the memory rule and `fit()` already did the choosing; what changed is the menu. The integration guide (sections 3, 7, 7b) and the Android README say which bundle a device gets and how to pin the other one.

**Why:** the bake-off's answer was worth nothing until the SDK handed it to a device. The founder's framing: small where clinical vocabulary matters, base where it does not, the developer decides for their use case.

**Simpler or more complex for the user?** Simpler for the clinician on an 8 GB phone or a laptop: one clinical word in six missed instead of one in four, with no change to the app. For the developer, one more bundle name to know, documented in the table they already read.

**Tests:** workspace 90 (unchanged); manifest check/verify; `recommend` per tier and the memory fallback checked from the CLI; one end-to-end run on the Mac through `recommend()` → `scribe-small` (TEST_REPORT).

### 2026-11-03 — The speech bake-off: Whisper small is the next bundle's speech model; keyterms and Intron's medium ruled out as run (`knowledge/26`, "The speech bake-off")

**What:** four recogniser configurations through the SDK's own `transcribe()` on ten PriMock57 consultations, scored on WER and a clinical-term error rate. Whisper small cuts clinical-term misses from 27.8 % to 16.8 % against base; a 40-word keyterms prompt doubles errors (`[OQ-121]`); Intron's AfriSpeech Whisper medium, converted to ggml and registered as `asr-afrispeech-medium`, scores 83 % WER on this audio (cause not separated from conversion). MedASR could not be run from the autonomous session. Harness additions: `evals/rescore.py` (re-score saved transcripts after a scorer or term-list change), a hyphen-spelling fix in the term list that had hidden ten correct recognitions, `summarise.py` labelling of pre-bake-off runs.

**Why:** the founder's verdict on the phone baseline was "totally unusable" for the clinical words; the choice of speech model had to be made on the same audio, the same way, for every candidate — not by model card.

**Simpler or more complex for the user?** Simpler for the clinician once the small bundle ships (one clinical word in six missed instead of one in four), at a model 3.3× larger to download and 2.7× the compute. For the developer, nothing changes in the API; `keyterms` must be documented as experimental until `[OQ-121]` is measured.

**Tests:** evals 5 → 6 (hyphenated spelling counts via a two-word alternative). Workspace unchanged (no crate code touched). Manifest re-signed and verified (`verdure manifest verify`: 6 bundles).

### 2026-11-03 — From the first phone run: an honest Android memory budget, one generation at a time, and the threading contract (`knowledge/26`, "The first phone run")

**What:** the vendor's report from the Pixel 8a produced one bug fix and one safety change in the SDK, and seven documentation corrections. `verdure-hw`'s Android budget now subtracts the process's own resident set, so `capabilities()` and the `load` gate fall after a model is loaded instead of reporting the same number forever. `LlamaEngine` holds a lock around `generate`, so two concurrent `extract` calls queue instead of each building a context. The threading contract (runtime and streams separately locked; transcription and extraction may overlap; generations queue) is written where a developer looks: the FFI crate docs, the guide, the Android README. The guide gained "What `extract` returns" (`attempts`, `used_grammar` and why it is false, `warnings`, when `SchemaInvalid` is thrown) and corrected its stale bundle text; the README gained a threads section, engine ownership for a foreground service, and the `Throwable` note.

**Why:** the integrator had to guess thread safety, guessed conservatively, and the transcript froze for 20–35 s on every extraction. Guessing is the failure the three-audiences rule exists to prevent.

**Simpler or more complex for the user?** Simpler for the developer: two sentences replace a design decision. No API change.

**Tests:** hw 4 → 5 (the Pixel's numbers: 4543 → 3860 → 3591 as the process grows; the `MemAvailable` floor; no underflow). Workspace 90. Engines compile-checked with the lock in place.

### 2026-10-29 — The SDK for Android, as a vendor receives it (`knowledge/26`, "The Android deliverable")

**What:** a CI workflow that cross-compiles `verdure-ffi` for arm64 Android with both engines, generates the Kotlin binding and compiles it against JNA, verifies the shipped library (arm64, 16 KB alignment, the UniFFI exports, the engines' strings present and absent from the engine-less control), and zips it with the credentials, manifest, schemas and an Android README (files to copy, two Gradle lines, a Kotlin walkthrough). Green on the third run; the cache now survives a failed run.

**Why:** Corti's clinicians run on Windows EHRs, in the browser, and on an iOS/Android app; the Pixel is the device in hand and Android builds at 1×. The Corti session is building the Android client of its own simulated platform; Haiku swaps this zip in.

**Found on the way:** whisper.cpp and llama.cpp each ship their own ggml, so the library carries every ggml symbol twice. Apple's linker tolerated it (both engines ran correctly on the Mac); Android's refused, and is now told to allow it — `[OQ-114]`, with the shared-ggml fix recorded. The Pixel run is the test of whether both engines behave with one copy winning.

**Tests:** the workflow's own assertions on the artefact (positive and negative controls); no Rust change.

### 2026-10-27 — The first real run: engines built, a consultation through the SDK, and three extraction fixes (`knowledge/26`, "The first real run")

**What:** the engines compiled into the SDK on the founder's Mac (`CARGO_BUILD_JOBS=2`, 5 min 39 s), and `evals/run-primock.py` pushed a 7.6-minute PriMock57 GP consultation (real speech, two speakers mixed, CC BY 4.0) through the Python binding: device → `recommend` → `load` → `transcribe` → `extract` per 60-second window → score. New in the repo: `evals/` (fetch script with attribution, scorer with tests, the runner, README), a real `scribe-floor` bundle in the manifest with measured hashes (Whisper base + Qwen2.5 0.5B; the chw bundle now carries the same real files), `bundles/manifests/schema-facts.json` in the dummy platform's fact shape.

**Three fixes to `extract()` the run forced, each with a test:** the model sees an outline with placeholders (`verdure_grammar::shape`) instead of the schema it kept copying back; a value still shaped like a placeholder is a named violation; a one-line demonstration in the system prompt. The no-JSON error now quotes what the model said and tells unparseable from absent.

**What the numbers say:** speech at 0.24× real time, first words in 2.6 s, 27.6 % words wrong with the smallest Whisper on mixed voices; the 0.5B produced schema-valid facts in 4 of 9 windows, quote-like, zero invented numbers. The floor model cannot extract clinical facts; the 1.5B–2B class is the honest floor for `extract` and needs ~2.2 GB free, which this Mac does not have with a browser open. Everything around the models worked as designed, offline.

**Simpler or more complex for the user?** Nothing changes for the end user. For the developer, nothing: the same `extract(text, schema)`. Inside, the prompt is shorter and the errors are more specific.

**Tests:** grammar 14 → 16, core 28 → 29, scorer 4 (Python); workspace unchanged otherwise.

### 2026-10-26 — The integration guide, the sandbox, and the first proof the bindings work (`docs/sdk-integration-guide.md`, `knowledge/26`)

**What:** a self-contained paper a developer (or a small model standing in for one) follows top to bottom to build a project against the SDK with no model on disk: vocabulary, prerequisites, sandbox credentials committed to the repo (`bundles/sandbox.token`, ten years, vendor `sandbox`), a Rust route (`sandbox/template` + `cargo run -p verdure-ffi --example chw`), a Python route (generate the binding, copy the dylib, run), the full method table with the error each can return, an honest "works today / needs an engine build / do not do" section, the CLI, and a test assignment with acceptance criteria. `sandbox/` is excluded from the workspace and git-ignored except the template.

**Why:** the founder wants to test the developer story by handing the paper to Haiku before any company sees it. A paper that only contains verified steps is the only fair test.

**Verified before writing:** the Python binding generated from the built library and the whole flow run through it (start, capabilities, signed manifest, `fit`, `recommend` refusal with numbers, index/query, `extract` → `EngineUnavailable`, then via a Python `CloudFallback`, `delete(All)`); the Rust example doing the same; the template copied into a new folder and built; the §5 snippet re-run verbatim after writing.

**Tests:** suite unchanged at 86; the example compiles with the crate and is the executable form of §4.

### 2026-10-24 — The developer's hand on the dial: `fit(bundleId)` and `load_overcommit` (`knowledge/26`, "three hands")

**What:** the founder asked whether a developer should be able to enforce a model of their choice. The vendor already chooses the menu by writing the manifest; what was missing was runtime control for the developer. `registry::problems()` now judges one bundle (licence, acknowledgements, memory, disk) and both `resolve()` and the new `fit()` use it, so the automatic walk and an explicit pick cannot disagree. The FFI gains `fit(bundleId) → FitReport` (the numbers plus every problem as a sentence, no substitution) and `load_overcommit(bundleId)`, which drops the memory gate for a developer who has measured their device, documented as able to abort the host process until `[OQ-58]`. Both `load` paths now check the licence too.

**Simpler or more complex for the user?** One more record and two more methods on the surface. Justified: without them a developer who disagrees with Verdure's pick has no honest path, and would fork the manifest to force one — the same control with worse visibility.

**Tests:** core 27 → 28 (a pick is judged not substituted; every problem listed, not just the first; a fitting pick reports its highest tier; an unknown id is `NoBundle`); ffi 5 → 6 (pin + truth; both load paths fail at the engine, not the gate, when nothing is downloaded).

### 2026-10-21 — The SDK workspace: core, grammar, hw, download, vault, rag, asr/llm pure halves, UniFFI surface, CLI (`knowledge/26`)

**What:** the root `src/` scaffold became a nine-crate workspace plus a CLI, green locally with no model on disk. `verdure-core`: `Policy` whose default is `06` as data; `02`'s tiers with a classifier; `Manifest`/`Bundle`/`resolve()` that walks a device's tier down and gives the vendor one sentence per bundle it passed over; offline Ed25519 licence tokens (`Issuer`/`Verifier`); the engine ports; `extract()` — schema in the prompt, JSON found in the reply, optional nulls stripped, validated, one repair retry with the violations as plain sentences. `verdure-grammar`: JSON Schema → GBNF with every declared property emitted in order and optional ones nullable, plus a validator over the same subset. `verdure-hw`: the app's measured probes per OS. `verdure-download`: the only socket — signed manifests, resumable range downloads hashed whole, atomic rename, cancellation that keeps the part file. `verdure-vault`: AES-256-GCM with the entry name authenticated, crypto-shred. `verdure-rag`: boundary-aware chunker, FTS5 BM25 + cosine over stored embeddings, reciprocal-rank fusion. `verdure-asr`/`verdure-llm`: the segmenter (now with timestamps), resampling, keyterms and whole-word replacements, non-speech cleanup; prompt framing for ChatML/no-think/Gemma and the context arithmetic — lifted from the app, with whisper.cpp and llama.cpp adapters behind features. `verdure-ffi`: `Verdure.start → capabilities → recommend → download → load → transcribe/chat/extract/index/query/delete`, with `ProgressListener`, `TokenSink` and `CloudFallback` as vendor-implemented callbacks. `verdure-cli`: `capabilities`, `license keygen/issue/verify`, `manifest check/sign/verify`, `recommend`, `grammar`.

**What problem it solves:** the scribe, education and CHW vendors run speech → transcript → structured output in someone else's cloud; their users' devices (Windows, Android, older Macs, 4 GB phones) are where that fails. One embeddable runtime with one install line and the vendor's own schema is the product (`26`, Part 3).

**Decisions recorded rather than slipped past (`26`, "Reconciliations"):** `02`'s tier names over the spec's; the memory rule `× 1.5 + 400` because the spec's `× 1.3 + 300` would have admitted the model the app measured (`memory.rs` tests the measurement); whisper.cpp first, sherpa-onnx second (`[OQ-107]`); the grammar sampler built but off (`[OQ-108]`); synchronous calls over `async fn`.

**Simpler or more complex for the user?** For the end user, nothing visible; what reaches them is the vendor's app working offline. For the developer, the whole surface is one object with twelve methods and three callbacks, and every error names what to do. For the project, nine crates is more structure than one — justified by the three rules a flat crate cannot enforce: no socket outside `verdure-download`, no engine type above a port, one public surface.

**User impact statement:** After this change, a vendor's engineer can verify a licence, read the device honestly, pick the bundle that fits, download it verified and resumable, and extract schema-valid JSON — with the engines the only piece not yet proven in a build.

**Tests:** 84 across the workspace (core 27, grammar 14, hw 4, download 7 against an in-process HTTP server, vault 5, rag 6, asr 9, llm 7, ffi 5). CLI smoke-run on the Mac: keygen → issue → verify, manifest check/sign/verify, `recommend` refusing honestly on this 8 GB machine and picking the P1 bundle for a simulated phone, the CHW schema's grammar. **Not verified:** the `llama`/`whisper` feature builds (`[OQ-112]`), anything on a device, any gate (`[OQ-113]`).

## Verdure desktop app (`app/`) — unified interface (post-pivot)

> Pivoted 2026-06-18 (`knowledge/18`) from "install separate apps" to **one interface that runs models in-app**, then to **Verdure as the personal AI** (`knowledge/20`). The "porter" naming in code/below is legacy; see `knowledge/21-handoff.md` for current state + next steps.

### 2026-10-23 — The mobile non-negotiables: work survives switching apps, the conversation persists, permissions are asked in the app; every notification remembered unless unticked (`knowledge/11`, `25`, `22`)

**What:** four things the founder found missing on the Pixel, built on the Mac without a build. (1) **Background work:** `background.rs` — a `Guard` held for exactly as long as a reply is written, a model downloads or a dictation runs. On Android it starts the new `VerdureWork` foreground service (Kotlin, written by `patch-android-project.py`; explicit Intent through system classes only, so no app class is looked up from a native thread) with the `dataSync` or `microphone` type and stops it when the last guard drops; on iOS it calls `UIApplication.beginBackgroundTask` through `objc2` with the class resolved at run time, so the same code compiles and no-ops on macOS; dictation additionally gets the `audio` background mode in the TestFlight plist patch. (2) **Persistence:** `transcript.rs` — the conversation as JSON lines in the app's data folder, the question written before the model runs and the reply after, compacted to the newest 400 entries; the chat's memory (`22`) now comes from this file; `chat_transcript` / `chat_new` / `chat_note` commands, a `chat:done` event, a "New chat" button, and `chat.ts`'s `reconcile` so what the screen echoed and what the transcript records are painted once; the screen also catches up on `visibilitychange`. (3) **The ask:** the first chat on a phone that can remember shows one card — benefit, mechanism, "Allow in Settings" / "Not now" — once (`memory.ts`). (4) **Every notification:** the listener reads every app unless unticked (`context.off` replaces the opt-in list), non-conversation notifications land as `notice` observations with `title: text`, the memory page lists apps to untick and a Forget per app (`delete_app`), counts say "19 messages in 2 conversations · 4 notifications".

**Why:** `knowledge/11`, "Mobile non-negotiables" — the founder's words: background processing is non-negotiable; a chat that clears when you open settings is broken; a permission nobody is asked for is not consent; and the user's life is more than their chats.

**Tests:** Rust 109 (transcript round-trip with a torn line, history pairs only completed exchanges, compaction keeps the newest, the guard begins and drops on every platform and finds no UIKit on macOS, the spool drains notices with dedupe and per-app counts, unticking and forgetting an app); vitest 60 (reconcile: nothing twice, a reply that landed while away, an echo that was never recorded hides nothing; the ask asked once and only where there is something to allow; held lines). The patch script ran twice against a fresh fixture and once against last week's tree (the conversations filter is removed); the manifest parses, both Kotlin files are balanced. Screenshots reviewed: the ask card, a conversation intact after a trip to Storage with four messages before and four after, the untick list with the bank unticked, New chat clearing it. **Not verified:** anything on a device — the Android JNI branch compiles only in CI, the iOS calls only on a phone; no Actions minutes spent, by instruction (`[OQ-115]`, `[OQ-116]`).

**User impact:** after this change, a person on the Pixel can send a question, switch to WhatsApp, and come back to the answer; open the book page and come back to the conversation they were having; and is asked, in plain words, before Verdure remembers anything.

### 2026-10-22 — Verdure's memory, end to end on Android: recall, the listener, What Verdure knows (`knowledge/25`, `27`, `11`)

**What:** the store became a feature. `recall.rs` turns a question into typed lookups by rules — a two-word or one-word name → `find_person` (two matches → "Which Jackson do you mean?" with a button per person, never a guess); a date expression → a `[start, end)` range in the device's local time (today / tonight / yesterday / this or last week, month, year / last N days / weekdays by tense / "Dec 25" and "25th of December" / "the 25th" / "in September" / a year); the rest → keywords. A pure lookup ("the last thing Tunde said", "what did Ada send today", "what am I doing on the 25th", "when did I last see Ada") is rendered directly with the records under it — no model runs. A question about words hands at most eight exact records to the model under a "from the user's own phone" heading, through the same sources block the web search uses, so the citation rules are taught once. Chat asks the store first on every question, then the web if it applies, then the model alone. On Android, `VerdureListener.kt` (written into the generated project by the patch script, declared behind `BIND_NOTIFICATION_LISTENER_SERVICE` with the `conversations` filter) reads `MessagingStyle` messages from the opted-in apps and appends JSON lines to a spool the app drains into the store; the service never calls native code. "What Verdure knows" shows the feed's state with the OS settings link, per-app opt-in chips, what is held per app, conversations (tap opens the thread in a sheet over the chat, so the chat is not lost), people, coming-up events, a WhatsApp "Export chat" importer, Forget on every row and Forget everything. Android's network probe now answers Wi-Fi / cellular / offline through `ConnectivityManager`, and the manifest excludes every memory file from Android auto-backup — both asserted on the built APK.

**What problem it solves:** the founder's questions — "what did Tunde say about the rent", "whose birthday is coming up" — asked in the one chat the user already has, answered from what the phone has seen, with the source a tap away, and nothing leaving the device (`25`). Pixel first because iOS exposes nothing (`27`).

**Decisions recorded:** rules before the model and the model last (Pandey et al. 2026); the spool over JNI-from-the-service (a crash boundary and no engine load in a background service); contacts live, calendar as thin rows (`27`); "unknown network" no longer asks on every search on Android.

**Simpler or more complex for the user?** One new icon in the chat bar and one page behind it; the chat itself gained nothing to learn — questions about your own life just work when they can, and say so under the answer. More complex for Android's Settings (notification access is a system toggle), which the page explains in one sentence with the button that opens it.

**User impact statement:** on a Pixel with notification access granted, a WhatsApp message that arrives is remembered on the phone, and "what was the last thing Tunde said?" is answered in under a second without the model.

**Tests:** Rust 105 (recall 9 new, spool drain 1 new); vitest 50; frontend typecheck + build; the patch script run twice against a fixture project (idempotent, well-formed manifest); screenshots of the recall chat, the ambiguity card, the thread sheet and the memory page reviewed.

### 2026-10-22 — The information map, and the store reshaped around events (`knowledge/27`, `25`)

**What:** the founder asked for the inventory before the feed — "map out all the types of information we can get from the user's phone… mark things by events, and know that this person is associated with this event at this time." `knowledge/27` is that map: eleven sources, where each is reachable on Android vs iOS, its permission and trust cost, and when Verdure takes it. Five of them contribute people-and-time (messages, calendar, contacts, photos, location) and they decide the shape: **person, place, event, observation, participation.** `context.rs` was rewritten to it: one `observation` table for every kind (`message` / `call` / `photo` / `visit` / `note`) on one time axis, with the message-only columns (thread, sender, direction) null for the rest; `event` with an origin (`given` by a calendar, with the OS id so a re-read updates instead of duplicating; or `inferred`, deduped on title + start) and an optional place; `participation` (person × event × role × source); `place` (unique in normalised form). New lookups: `events_in_range`, `events_with`, `events_titled`, `people_at`, `at_event`, `record` (any non-message observation), `upsert_event`, `add_participant`, `attach_to_event`, `person_for`, `delete_event`; `events_as_snippets` renders an event for the prompt ("At The Wheatbaker. With Ada, Tunde."). Everything from 2026-10-21 (people, aliases, identities, merge, `find_person` with ambiguity, `last_from`, `in_range`, person-scoped FTS5, the export parser) is unchanged in behaviour.

**What problem it solves:** the first store was shaped by its first source, a message table. Most of what a phone knows is not a message, and the questions that matter most ("when did I last see Ada", "what happened at the wedding", "what am I doing on the 25th") cut across sources. Reshaping now costs nothing — no device has the file; after the Pixel listener fills it, it would be a migration on a phone.

**Decision recorded:** one observation table with a kind column over one table per kind (design-it-twice, `skills/architecture.md`): the time index, the search index, the event link and the deletes are written once; the cost is nullable columns on a message row. `25`'s "calendar is never copied" softened to "a thin row with the OS id; the OS stays the truth" because a participation link needs a stable row to point at. Contacts still live-only.

**Simpler or more complex for the user?** Invisible today. It is what makes "whose birthday is coming up" and "events with Tunde" one query later instead of a feature per source.

**User impact statement:** nothing visible yet; the foundation that the Pixel-first build (`21`, item 0) stands on.

**Tests:** 24 in `context.rs` (6 new: one timeline across kinds with the photo found by its place; events link people to a time and a place with a re-read updating not duplicating; inferred events dedupe on title + start; deleting an event forgets participation but not people, and unlinks the note; participation follows a merge; `delete_all` clears events). Full app suite green, zero warnings.

### 2026-10-21 — Context store reworked: structured first, entity-centric (`knowledge/25`)

**What:** the founder's second research report (storing personal context across Android/iOS/macOS) argued, with evidence, that the questions that matter are lookups, not retrieval — and it was right. `context.rs` now has an entity layer (`person`, `person_alias`, `identity`) with an ingest rule (the app's key → the exact alias → a new person; different names never merged by guesswork) and an explicit `merge_people`; `find_person` resolves exact → prefix → word and returns *Ambiguous* when two people match, so "Jackson" becomes a question, not a guess; `last_from` and `in_range` are indexed lookups; search can be scoped to a person; items carry `sender_person_id`; `facts` gained bi-temporal validity; the export parser learns day/month order from the file; SQLite runs WAL + NORMAL with an 8 MB cache. Calendar and contacts are decided as live OS queries, not copies.

**Why:** verified ✓ — Pandey et al. (arXiv:2603.11513): below 7B, retrieved context overturns 42–100 % of answers the model already had right; "the bottleneck is context utilization". So: few, exact records found by rules, model last. Also verified ✓: Android Go phones on Android ≤10 cannot bind notification listeners, and Android 15 redacts OTPs — both now in `25` and `[OQ-102]`.

**Tests:** Rust 86 → 91 (18 in the module): the last thing someone said is a lookup that excludes the user's own replies; exact/prefix/word resolution and the two-Jacksons ambiguity; one name across two apps is one person, a different name is not, merge moves everything and future keys land on the merged person; time ranges end-exclusive and person-scoped; person-scoped search; alias normalisation incl. Yoruba diacritics (a combining-mark bug caught: "Ṣẹ́gun" split in two until combining marks were dropped); person deletion forgets the index; the iOS export's date order learned from a day > 12.

### 2026-10-21 — The personal context store, step 1 (`knowledge/25`)

**What:** `app/src-tauri/src/context.rs` — Verdure's memory of what it has seen, on the device. A SQLite store (`items` in `threads` from `sources`, plus a `facts` table for later extraction) with FTS5 search that turns a question into its content words (`"tunde"* OR "rent"*`), stable dedupe keys so a re-posted notification or a re-shared export stores nothing, thread/count/delete calls for the transparency page, and a parser for WhatsApp "Export chat" files (Android and iOS forms, 12/24-hour clocks, multi-line messages, system lines dropped, the user's own side marked `out`). Retrieved items render through the web search's `sources_block` unchanged, so an answer from memory will cite `[n]` the same way an answer from the web does.

**What problem it solves:** the founder wants Verdure to *already know* — "what did Tunde say about the rent?", "whose birthday is coming up?" — without the user doing anything new. This is `20`'s personal context core (`[OQ-56]`) made concrete, with messages as its first source. Nothing is wired yet: this step is the pure, testable half that needs no device.

**Simpler or more complex for the user?** Nothing visible changes yet. When wired, the only new surface is "What Verdure knows" — a page to see and delete what's stored — which *ask, don't sniff* requires of a notification-access permission.

**Tests:** Rust 73 → 86. FTS5 presence; ingest once + duplicate recognised + same words later is new; search by word, sender and thread title with prefixes, stopword-only queries empty; the FTS query builder neutralises FTS syntax; threads newest-first with counts and last line, items oldest-first; deleting a thread forgets its index entries, delete-all empties everything; dedupe key stable and sensitive; civil-date round trips incl. a leap day; items → snippets → the existing sources block; Android export (multi-line, system line, directions), iOS export (12-hour clock, month-first where unambiguous — and the honest day-first default where not, `[OQ-99]`), group detection, re-import without duplicates. First attempt used a contentless FTS5 table, which cannot delete by rowid — switched to a normal FTS5 table (messages are small).

### 2026-10-21 — Web search v2: a switch, a model that asks, tappable sources; and Verdure picks the model (`knowledge/24`, `03`)

**What:** (1) The per-search approval card is gone. A **globe switch** in the chat bar turns web search on (off by default); the first time, a card says what leaves the device, with "Got it". While it's on, time-sensitive questions are searched with the user's own scrubbed words, and for anything else the model — now *told* it can check the web — may answer with one line `SEARCH: <words>`, which the app runs through the data rule and answers with. Off, the model is told it has no web and says so when asked. (2) Citations `¹` and the source rows open the page in the browser. (3) The "results weren't used" badge is removed; the honest badges (no connection / service didn't answer / nothing found) stay, and the model is told the same reason. (4) Grounded answers no longer stop mid-sentence: the reply budget is half the window up to 512 tokens. (5) **The model picker is gone.** Verdure runs the most capable model that fits memory *and* disk; the chip at the top names it and opens **Your model**: which model (tier + name + size), and when a smaller one is running, exactly how much more free memory or storage the best one needs, with **Check again** and the platform's honest make-room help. The 27B stays an opt-in experiment on 16 GB iOS.

**Why:** the iPad pilot (`testflight-15`). The founder's own test: the Balanced model used web results but was called "from memory"; the Powerful one cited well but was cut off; a question without a time-word never got an offer and the model then denied it could search twice. On the model choice: the small and medium models aren't good enough at grounded answers to be a choice the user can get wrong; the honest version is to pick for them and say what more room would buy.

**Simpler or more complex for the user?** Simpler on both counts: one switch instead of a tap per search, and no model decision at all on first launch — chat opens directly. The added screen (Your model) is informational and only matters when the device is short; it replaces a picker that asked the user to know what "Balanced" meant.

**ByteShape licence (founder's question):** Apache-2.0 on the model card and the base Qwen3 model, no restrictions or attribution beyond Apache. The risk is availability (a four-person org, unpublished method) → `[OQ-96]` mirror the GGUFs.

**Tests:** Rust 69 → 73 (system prompt variants, the `SEARCH:` first-line parser incl. bent formats, the scaling reply budget, plan: best fits / memory gap / storage gap / nothing fits / disk-aware pick / experiment opt-in + where offered; `prompt_cap` invariant kept against the new budget). App vitest 50 (switch opt-in default, modes without "off", shortfall copy ×4; picker tests retired with the picker). Website 30 (lockfile guard held through the opener install — only `@tauri-apps/api` moved 2.11→2.12). Screens reviewed at phone width: web off reply, switch-on card, fast-path and model-asked searches with footers, Your model with the 700 MB note, Storage. **Not on a device yet**, and the opener plugin is the app's first mobile (Swift) plugin — the next iOS build carries link risk.

### 2026-10-20 — Web search: the local model can check the web, on the user's say-so (`knowledge/24`)

**What:** For a question that looks like it needs fresh facts ("naira to dollar rate today", "who won the AFCON final"), chat offers to check the web: an AI message shows the scrubbed search words in an editable field, **Check the web** / **Answer without it**, and a small "What leaves your device?" disclosure (open the first time, a one-line toggle after). On the tap, the words go through a relay on verdureai.com (`website/app/api/search`) to Brave's LLM Context API and ≤5 short snippets return; the model answers from them with `[n]` citations, and the reply lists its sources by site. If the reply cites nothing, a badge says it answered from memory. Wi-Fi only by default; on mobile data a one-time ask ("Allow on mobile data") becomes a setting; offline, the model is told the check couldn't happen and the answer says so. Storage gets a **Web search** section (Wi-Fi only / Wi-Fi and mobile data / Off) with the same disclosure.

**What problem it solves:** a local model's knowledge stops at its training date, and the people Verdure is for ask about prices, rates, results and news. The research (`24`) showed every cheap route — scraping, WebViews, SearXNG — leaks the phone's IP and fingerprint to a search engine and breaks or gets blocked; this route sends only the approved words, through one hostname, for a few KB.

**Simpler or more complex for the user?** One extra tap on the questions where it matters, and only there — the offer is a local rule, never a network call. Justified: the tap *is* the privacy guarantee (nothing is sent the user hasn't read), and the words being editable is principle 4 of `06` made concrete. No byte counts, no settings wizard: the only setting is the connection rule, and it's asked in the moment, once.

**User impact statement:** After this change, the user can get an up-to-date answer to a time-sensitive question more easily because Verdure offers the search, shows exactly what would be sent, and marks which parts of the answer came from the web.

**Tests:** Rust 53 → 67 (`web.rs` scrub incl. trailing punctuation and multibyte caps, budget bounds, fit order/trim/count, sources block framing, reply parsing and the no-jargon error, the flags→state rule, relay host; `llm.rs` grounding placement and history exclusion). App vitest 36 → 50 (`web.ts` default/gate/offer heuristic/disclosure claims; `format.ts` citations and `citedSources`). Website 9 → 25 (`lib/search.ts` validation, budget clamp, upstream URLs carry no user parameters, both Brave reducers, trim, a maximal reply < 4 KB). Route exercised locally: GET → 405, no key → 503 with a plain sentence. Screens reviewed in the browser preview at phone width. **Not yet:** a live keyed call (`[OQ-90]`), on-device on the iPad, iOS cross-compile.

**Pending the founder:** `BRAVE_SEARCH_API_KEY` on Vercel; a `testflight-N` tag.

### 2026-10-20 — Website: "newer Mac" visitors get a model that fits

**What:** The default text recommendation for a newer Mac (L3) is now **Qwen3 8B** (5.2 GB), replacing **Mistral Small**. `knowledge/03` had listed Mistral Small as a ~5 GB 7B model; it is a 22–24B model and a **14.3 GB** download (measured from the Ollama registry), on machines that often have 8 GB. **Why Qwen3 8B:** a real 8B in the size the page promises, under Apache 2.0 (`09`: lead with Apache/MIT). Llama 3.1 8B stays as the coding pick. **Tests:** decider 83 with fixtures updated. Reaches verdureai.com once `[OQ-81]` is fixed.

### 2026-10-20 — Website: the recommended app can actually run the recommended model

**What:** For image generation on Windows/Linux graphics cards, the website now recommends **SDXL** with Fooocus (the friendly app) and lists **FLUX.1 schnell** as the higher-quality alternative that needs ComfyUI. Image editing there offers ComfyUI only. **Why:** Fooocus runs SDXL only, so a user who followed the friendly path got a different model than the one named, or for editing, an app that can't do it. **Knowledge first:** `12` corrected, then the matrix and copy. **Tests:** decider 83 (+1 invariant, with a positive control). Website-only; it reaches verdureai.com once `[OQ-81]` is fixed.

### 2026-10-20 — Emoji and accented letters stop disappearing from replies

**What:** Chat replies are decoded with one UTF-8 decoder for the whole reply instead of a fresh one per token. **Why:** a character can be split across two tokens; decoded separately, each half was invalid and silently dropped, so emoji, CJK and some accented letters could vanish mid-reply. **User impact:** replies show every character the model wrote. **Tests:** Rust 53 (+1); not yet confirmed against a real model on this machine.

### 2026-10-20 — The pre-pivot porter and its screens are removed

**What:** Deleted the unreachable intent-grid → recommendation → one-click-setup flow and the Buzz installer behind it (`catalog.rs`, `porter.rs`, `port_tool`). Also deleted device-tier detection, which ran on every launch only to feed those screens; file transcription (`transcribe_audio`, the `symphonia` decoder, ~150 lockfile lines); the "best model only" code; and an always-on rollback flag. The command list is now one `handlers()` shared by the app and its integration test.

**Why:** The founder's call during the cleanup (`[OQ-60]`'s legacy half): none of it was reachable since the app opened into chat in July. It was kept "pending a decision", and the decision was remove. Git history is the archive.

**Simpler or more complex for the user?** Nothing visible changes. The app does less work on launch (no tier detection) and ships a smaller dependency tree. THIRD-PARTY-NOTICES no longer lists `symphonia`, and now points to the source of every MPL-2.0 component (only `symphonia` had a pointer before).

**Tests:** Rust 66 → 52. The removed tests covered only deleted code: 9 for catalog/porter, 2 for tier detection, 3 for best-model-only. The notices guard now checks every MPL row, and a positive control confirmed it fails when a pointer is missing. tsc clean, vitest 36.

### 2026-10-20 — Plain errors, the right settings path, and a recommended first choice

**What:**
- **First-run picker:** pre-selects and labels the most capable standard model that fits this device ("Recommended for this device"), so the first screen is one tap. The experiment is never recommended.
- **Error messages:** errors from the engine, the microphone and downloads now show a plain sentence ("Verdure couldn't finish that reply. Please try again.") and log the technical detail. The memory message no longer names the internal model or points at a "Settings" screen that doesn't exist; it says to tap the model name at the top.
- **Microphone help:** names *this* device's settings path (macOS, iOS, Android) instead of Android's everywhere.
- **Storage:** lists chat models by tier ("Powerful chat model").
- **Chat screen:** stops quoting a hard-coded "0.4–1.4 GB".
- **Updates:** a failed update install no longer shows a raw exception.

**Why:** The truck-driver test. Each of these put jargon, a wrong instruction, or a decision with no default in front of a user (2026-10-20 inspection).

**Simpler or more complex for the user?** Simpler: one obvious first tap, and every error tells the user what to do next.

**Tests:** vitest 36 (+6: `recommendedChoice` ×4, per-OS mic help ×2), Rust 66 (a download-message contract shared with the UI's resume detection). Visual check: the first-run picker was screenshotted in the browser preview, showing three cards with Powerful pre-selected and labelled.

### 2026-10-20 — Sizes read the same everywhere; code in replies stays literal

**What:** Every screen now shows sizes one way: MB under 1 GB, else GB to one decimal. Storage used to say "1336 MB" for a model the picker called "1.3 GB". In chat replies, text inside backticks now stays literal: `**x**` or `a*b*c` in a code span used to come out bold or italic.

**Why:** Both came out of moving the UI's pure formatting into `app/src/format.ts`, so it could be tested at all (`main.ts` starts the app when imported). The code-span bug was found by the first test of that function.

**Simpler or more complex for the user?** Simpler: one way of writing sizes, and code in answers shows exactly as written.

**Tests:** vitest 5 → 29, with the injection cases first (scripts, event handlers, `javascript:` links, SVG in headings, lists and code spans are all escaped). tsc and the vite build are clean.

### 2026-10-20 — Chat sizes its memory with the same number the load check uses

**What:** The conversation window is now sized from `available memory − runtime_mb`, the model's full resident footprint and the same figure the load gate uses. It used to be `available − file size − 700 MB`.

**Why:** The two disagreed. A model let in at exactly its gate could then be given a window that pushed it past that gate, which is an OOM abort that kills the app (`[OQ-58]`). Now the window only grows into memory nothing else was promised.

**Simpler or more complex for the user?** Invisible on typical machines: the 4B with 3.5 GB free still reaches the 4096-token cap. Near the memory edge the window is smaller, and the app doesn't crash.

**Tests:** Rust 65 (+1 new: at the gate edge the window is the floor; the spare test now asserts the full footprint). `knowledge/22` updated first.

### 2026-10-20 — The 27B experiment no longer takes the 4B away from iPhones

**What:** The picker's Powerful choice is the Qwen3 4B again on every device. The Qwen3.8 27B is now an extra fourth card, labelled **Experimental** ("A much larger model we're still testing. A very big download, and it may be slow or fail to start."), offered only on iOS devices with 16 GB of memory (`model.rs::chat_model_menu`, keyed on *total* memory). If a saved choice isn't on this device's menu, the picker falls back to the first one it does offer.

**Why:** The 2026-10-08 version swapped the 4B for the 27B on every iOS build, so iPhones and 8 GB iPads lost a model they could run and were offered one they never could, with "close some apps" advice that could never work. Found by the 2026-10-20 inspection.

**Simpler or more complex for the user?** Simpler for everyone except the 16 GB iPad, which sees one extra, clearly labelled card.

**Tests:** Rust 64 (3 new: the 4B is on every menu; the 27B only on 16 GB iOS; a stored 27B id on a small device falls back instead of refusing). tsc clean, vitest 5. Reaches TestFlight with the next `testflight-N`.

### 2026-10-08 — iOS builds reuse one certificate instead of minting one per run

**What:** `build-testflight.yml` now signs manually with one stored Apple Distribution certificate (`IOS_DIST_CERT_P12` / `IOS_DIST_CERT_PASSWORD`), imported into a throwaway keychain. A new script, `app/scripts/ios-signing.py`, fetches the App Store profile for that certificate from the App Store Connect API (replacing one fixed-name profile each run) and checks that the profile grants the entitlements the build requests. The API cannot enable kernel capabilities such as Increased Memory Limit (HTTP 409), so those are ticked once in the portal. It uses only Python's standard library plus `openssl`, with no third-party action, so the API key stays in code we own. The signing steps now run first.

**Why:** Automatic signing on an empty runner keychain made a new certificate every build, until Apple's per-account limit stopped `release-14`. Now no build can create a certificate.

**Cost:** A bad or missing secret fails in about a minute instead of after the Rust build. Builds no longer contact Apple to set up signing, apart from one profile request.

**Simpler or more complex for the user?** Invisible to app users. For the founder: two more secrets once, and renewal once a year, in exchange for builds that stop failing on a limit nobody can see coming.

**Tests:** `test_ios_signing.py` 10/10 locally; live runs `testflight-9`…`-12`, first green at -12 (TEST_REPORT 2026-10-08). No CI job runs the script tests yet (cleanup queue in `21`).

### 2026-10-08 — iPad experiment: Qwen3.8 27B in the Powerful slot

**What:** On iOS builds only, the picker's Powerful choice is now ByteShape's Qwen3.8 27B (`IQ2_XXS`, 8.8 GB download, ~10 GB estimated to run) instead of the Qwen3 4B. Mac, Android and Windows are unchanged. The TestFlight workflow now asks iPadOS for the increased memory limit and extended virtual addressing; without them, iPadOS kills the app long before a 10 GB model fits. The picker now draws only the models the backend offers, so each platform shows exactly three cards.

**Why:** The founder wants to find the iPad's ceiling. This is explicitly temporary: `[OQ-84]` lists what the first run settles and how to revert.

**Simpler or more complex for the user?** Unchanged for everyone off the iPad. On the iPad, the Powerful card now costs a much bigger download, and its description says it needs a 16 GB iPad. The memory check still refuses the model honestly on a device that can't hold it.

**Tests:** 63/63 Rust (2 new), TS type-check clean, 5/5 vitest. The iOS branch of the `cfg` was not compiled locally (no iOS target on this Mac); CI compiles iOS on a PR.

### 2026-09-11 (later) — Play uploads automated, without handing the key to a stranger

**What:** `android-play-N` now builds, signs, **and uploads** to Google Play's closed track — the Android equivalent of what `release-N` already does for TestFlight, restoring the pipeline's "one tag → three platforms" property.

**Why it isn't a marketplace action.** The obvious implementation is `r0adkll/upload-google-play`. That step receives a credential which can publish to the founder's Play account on every run, and one compromised release of that action would be enough. `app/scripts/publish-to-play.py` talks to Google's API directly — about 100 lines, no supply-chain surface beyond Google's own auth library, credential never leaving code we own. This is the "we don't win on code, we win on trust" line applied to a build step.

**What it deliberately cannot do.** The **first** bundle of a brand-new app must be uploaded by hand in Play Console; Google's API refuses it. The step therefore *skips without failing* when `PLAY_SERVICE_ACCOUNT_JSON` is absent, printing what to do instead, and the artifact upload remains as the manual path.

**A bug caught before it could bite.** Play rejects a repeated `versionCode`, and Tauri's template hardcodes a default of `1`. Tauri's own `autoIncrementVersionCode` would not have helped — `gen/` is regenerated every CI run, so there is no state to increment, and it would emit `1` forever. Upload #1 would have succeeded and upload #2 failed: **the automation breaking the first time it was actually used.** Fixed by feeding `github.run_number` into the patch script.

**Also this session:** the Play identity was verified in the shipped artifact rather than the build log — `android-build-10` is green, and the APK's own binary manifest contains `ai.verdure.app` (plus `ai.verdure.app.fileprovider`) alongside `ai.verdure.porter.MainActivity`, confirming both halves: Play sees the new name, the JNI symbol keeps the old package.

**Did this make Verdure simpler for the user?** It makes shipping simpler, which is upstream of everything users get. No user-facing surface changed.

### 2026-09-11 — The Play identity (`ai.verdure.app`), without moving macOS or iOS

**What:** Google Play will know Verdure as **`ai.verdure.app`** from its first upload. Applied by patching the generated Gradle project's `applicationId` in `app/scripts/patch-android-project.py`; `tauri.conf.json`'s `identifier` is untouched.

**The problem it solves.** The Play `applicationId` is permanent after the first upload, and the legacy name `ai.verdure.porter` predates the 2026-06-18 pivot — freezing it forever would be a permanent artefact of a direction we abandoned.

**Why it looked expensive and wasn't.** Tauri generates the Android package from the one global `identifier`, and offers no override (`AndroidConfig` has only `minSdkVersion`/`versionCode`/`autoIncrementVersionCode`/`debugApplicationIdSuffix` — checked against the CLI's own schema). Changing that identifier would have meant:
- **macOS/iOS data loss.** `app_data_dir()` derives from it, so the published, self-updating desktop build would look in a new folder and re-download ~1.7 GB of models it already had.
- **A silently dead microphone.** The JNI symbol `Java_ai_verdure_porter_MainActivity_initAudioContext` is spelled from the Java package. A mismatch doesn't error — the capture thread dies and the UI hangs on "preparing" until the watchdog. That is the bug root-caused in August.

**The actual fix.** Android separates `applicationId` (install/Play identity) from `namespace` (Java package) by design; Tauri's template emits them on **two separate lines** (confirmed by extracting the template from the CLI binary). Patch only `applicationId`. macOS and iOS never move, the JNI symbol stays valid, no migration exists to get wrong.

**A property worth keeping.** Play only ever sees `applicationId`. So the eventual code-level rename — `namespace`, crate, data dir — is now **invisible to Play** and can happen whenever, on its own merits, with the migration it needs. Decoupling made the expensive change optional instead of urgent.

**Did this make Verdure simpler for the user?** Neutral for users today; it protects them from a silent 1.7 GB re-download and a dead mic. It made the *system* simpler: one platform's identity changed, on one line, instead of three platforms moving together.

**Tests:** the patch script is exercised against a fixture built from Tauri's real template — correct rewrite, `namespace` untouched, idempotent on re-run — and all three guards verified to fire on bad input (changed `namespace`, duplicate `applicationId`, moved Gradle file), with a passing control. Not yet run through a real `tauri android init`; this machine has no Android SDK, so CI is the first real execution.

### 2026-09-10 — Qwen 3.5 2B: a hybrid model with 12× cheaper conversation memory

**What:** added Qwen 3.5 2B (Apache 2.0) to the chat registry as the "Balanced" tier, replacing Qwen 2.5 1.5B. Added Qwen 3.5 4B as data, gated off pending a footprint measurement (`[OQ-82]`). New `PromptStyle::ChatMlNoThink`.

**The problem it solves.** A beta tester reported *"it ran out of context."* The root cause is arithmetic, not a bug: a dense transformer caches K and V on **every** layer, so the 4B we ship costs **144 KB per token** of conversation, and `llm.rs::context_for_budget` divides the machine's spare RAM by exactly that. Reaching the 4096-token ceiling needs ~576 MB free — which our target devices do not have, so the window collapses and old turns get evicted.

Qwen 3.5 is a **hybrid**: only 6 of its 24 layers carry a KV cache (the rest are Gated DeltaNet, whose recurrent state is a fixed 19 MB regardless of conversation length). That is **12 KB per token** — the same cost as the 0.5B floor model, for a 1.88 B model. Same footprint class, ~10× more conversation remembered.

**What running it revealed that reading about it did not.**
1. **It is a reasoning model.** Framed as plain ChatML it emitted 256 tokens of `<think>` scratchpad and the answer never arrived. The documented off-switch is a pre-filled *empty* thinking block after the assistant marker. This is invisible to any test that only asserts a non-empty reply, so it is now asserted directly (`no_think_style_prefills_an_empty_thinking_block`).
2. **Peak RAM is ~1 GB above the file size.** The load log shows a `CPU_REPACK` buffer — llama.cpp re-laying-out weights for CPU SIMD, a second resident copy — at **490 MB on a 1211 MB model**, plus a ~500 MB compute buffer. Measured peak ~2218 MB. This retro-justifies the 4B's `runtime_mb: 2300`, which `[OQ-64]` had flagged as ~900 MB over-cautious against a "measured 1.40 GB" that was **weights-only**. The stale claim was corrected in three files before it could talk someone into lowering an OOM guard.

**A hypothesis that did NOT survive.** Going in, the likeliest suspect for the "fake facts / stupider answers" reports was our aggressive **2.77 bpw** quantization of the 4B. Run head-to-head on the same probe through the same build, the incumbent answered well (one factual slip: it conflated aurora australis with borealis). One prompt is not an eval, so the quantization question is neither confirmed nor cleared — it is exactly what `[OQ-74]` exists to settle. **The KV-cache win, by contrast, is arithmetic and needs no eval.**

**Rejected, with reasons:** MoE at every tier we serve (cuts compute, never resident memory — all experts stay in RAM; smallest useful one ~5 GB). **Meta Muse Glimmer** (30B, smallest GGUF 16.8 GB, wants 24 GB VRAM, and our vendored llama.cpp has no architecture entry for it — it would not load).

**Did this make Verdure simpler for the user?** Yes, and without a new control: the same three-item picker, one rung better, and the machine remembers more of the conversation without anyone choosing anything. No context-length setting was added (`knowledge/22` — the jargon knob Verdure exists to delete).

**Tests:** 60 Rust tests pass (was 54). Verified on-device via the `[OQ-71]` repro harness on the 2019 Intel Mac: loads through our pinned `llama-cpp-2 0.1.154`, `graph splits = 1`, all CPU (Metal fix holding), coherent output.

### 2026-08-21 — Chat works on Intel Macs (the Metal fix)

**What:** chat now produces correct answers on the founder's 2019 Intel Mac. It never had — every reply was token salad or a single repeating character.

**Cause:** we were running inference on the **Metal GPU backend**. Metal targets Apple Silicon; on an Intel Mac's integrated GPU it returns wrong numbers instead of failing. `Cargo.toml` claimed we were "plain CPU" and we never were.

**Fix:** `llm.rs::model_params()` now chooses the backend explicitly — CPU device only on x86_64 macOS, unchanged everywhere else. Costs little: upstream's CPU build does ~42 tok/s on this machine.

**User impact:** the desktop app's headline feature works on the founder's own machine for the first time. It also removes the false premise — carried in the handoff since June — that this Mac was "too weak" for these models. It never was.

**Verification:** coherent answer with the fix on, CPU buffers only; Metal and the garbage both return with it off. All 51 tests green locally.

**Cleanup before shipping it:** removed the two diagnostic models (`Q8_0` and `fp16` 0.5B) added to isolate the — now disproven — K-quant theory. They were probes, not products: the picker showed them as "Test: Q8" and "Test: fp16", two baffling extra options for anyone who isn't debugging ggml kernels. Gone from `model.rs` (constants, `chat_candidates`, `MODELS`), the picker's `MODEL_TIERS`, and the CI URL check. They never shipped in a tag and were never downloaded on-device, so nobody has a stray file or a stale selection. The picker now also falls back to the first tier when the stored model id names a tier that no longer exists — previously a retired model left the picker with nothing selected, and Verdure will retire models again.

### 2026-08-28 — Verdure credits what it's built on

**What:** the app now ships `THIRD-PARTY-NOTICES.md` — 532 open-source components, 245 distinct licence texts — bundled inside the `.app` and reachable from Storage → "Open-source licences".

**Why it isn't just paperwork.** MIT and Apache-2.0 hand us the right to use, modify and sell software built on this code, commercially, for free. The one thing asked in return is that the copyright notice travels with the binaries we distribute. We were about to hand out a `.dmg` carrying none.

The better reason is the second one: `knowledge/18` commits Verdure to never building a model or engine. **llama.cpp and whisper.cpp *are* the engine.** Building a product on Georgi Gerganov's work, pitched to the same open-source community, without naming him, is inconsistent with what this project says it is. The file costs nothing and is simply true.

**Generated, not hand-maintained** — `app/scripts/generate-notices.py` reads the resolved dependency graph and the licence texts cargo already vendored, so it needs no network and can't drift from what compiled. Two tests keep it honest: every `Cargo.lock` package must appear in the file, and the engines must be credited by name.

**One thing worth knowing:** `llama-cpp-sys-2` vendors all of llama.cpp but publishes no LICENSE for it. Its text is checked in separately, fetched from upstream — which mattered, because llama.cpp's copyright reads 2023-**2026** while whisper.cpp's reads 2023-**2024**. Copying one to the other would have shipped a false notice.

### 2026-08-28 — Verdure can update itself, and only if you let it

**What:** the desktop app now checks for, downloads, and installs new versions. macOS has no store to do this for us, and with `[OQ-58]` open a stuck build is a real risk — the fix for a crash could exist for months while a tester sits on the version that vanishes. iOS and Android are untouched; their stores own updates, so every code path is desktop-gated.

**The decision that mattered wasn't technical.** An update check is a network request that reveals someone runs Verdure, roughly when, and from what IP — the exact shape of quiet signal `knowledge/06` promises we don't collect. Shipping a silent launch-time check would have been a small, invisible breach of the product's central claim. So it is **opt-in**: asked once, in wording that names the actual thing (*"checking asks our server whether one exists"* — not *"check for updates?"*, which hides that checking **is** the request), and changeable from Storage forever, because a choice you can't revisit is a trap rather than consent. Three modes — tell me, install automatically, never check — and "never check" is styled like the others, since a greyed-out decline would make the whole question theatre. Full rationale in `knowledge/23`.

**Also fixed: Verdure was over-reporting free memory** (`[OQ-77]`). `available_ram_mb` added `purgeable` pages to a sum that already included them via active/inactive — 1976 MB claimed where 1895 MB was true. That leaned the one dangerous way, since this number gates an allocation whose failure kills the whole app. Dropping the term makes our figure *equal* macOS's own accounting rather than approximate it, because the page categories partition memory exactly.

**User impact:** a tester can install once and receive every fix afterwards without being asked to re-download anything — and without Verdure quietly acquiring a habit of phoning home. Updates are ~30 MB, not 1.4 GB, because models live outside the app bundle.

### 2026-08-22 — Build stamp in the corner; honest download sizes; CI off the push path

**Build stamp.** Every screen now carries a tiny, dim line bottom-right saying which build it is — `release-14` when the commit is tagged, `dev 1097ec8` otherwise, with a trailing `+` when the tree had uncommitted changes. Captured at compile time by `vite.config.ts` from git and baked in via Vite `define` (the running app has no git and no shell).

*Why it exists:* on 2026-08-22 a stale, translocated copy of Verdure was tested for several minutes because two builds looked identical on screen. "Which build am I looking at?" should never be a guess. Deliberately faint (10px, 38% opacity, `pointer-events: none`) — findable when you look for it, invisible when you don't.

**Honest download sizes.** `approx_mb` was a set of round guesses; every one was wrong against the actual `Content-Length`. The worst mattered: **"Small" claimed 400 MB and is really 469** — and `approx_mb` feeds the picker's `fits_disk` check, so Verdure could tell the most disk-constrained user a download fits when it doesn't. All five corrected to measured values (Whisper 141, Small 469, Balanced 1066, Powerful 1336, Gemma 2963).

**CI no longer runs on push.** Pushing code to GitHub is now free; verification happens on this machine before the push. Details in the changelog and `skills/testing.md`.

### 2026-08-22 — Chat confirmed working on the founder's Mac (`release-14`)

**What:** the Metal fix shipped. `release-14` built, signed, notarized and stapled; installed from the CI artifact (via `gh`, so no Safari quarantine flag and no AppTranslocation). Asked it a question, got a coherent answer.

**Why it matters:** chat has never worked on this machine. Every reply since June was token salad. `[OQ-71]` is now closed end-to-end — repo → CI → signed build → on-device.

**Two assumptions it killed:**
1. **"The 8 GB Intel Mac only exercises the fallback chat tier."** It doesn't — it answered on the **Powerful (Qwen3 4B)** tier, weights resident. That belief formed while the Metal bug made every model produce garbage here, so a *correctness* failure was misread as a *memory ceiling*.
2. **"`runtime_mb: 2300` is about right for the 4B."** Measured RSS ≈ **1.40 GB** resident, ~1.55 GB with a full 1024-token KV window. The gate is ~750 MB too conservative and is denying the 4B to machines that could run it. Recorded under `[OQ-64]`, deliberately **not** acted on yet: one short conversation isn't a peak measurement, and undershooting this number OOM-`abort()`s the app.

**Not shipped:** iOS. The TestFlight archive failed on an Apple *account* limit (certificate cap reached), not on anything in this change. Pre-existing, founder-side, non-blocking.

**Still unverified:** dictation under the hardened runtime. `Entitlements.plist` grants the mic, but no notarized build has ever had that path exercised on a device.

### 2026-08-17 — macOS signing + notarization; Storage shows where models live; CI cost cuts

**Signed + notarized macOS builds.** `build-app.yml` now signs with a Developer ID Application certificate, notarizes through Apple's service, and staples the ticket — verified in CI by `codesign --verify`, `spctl --assess` (Gatekeeper's own check: `accepted / source=Notarized Developer ID`) and `stapler validate`. Users no longer need `xattr -cr`; the app opens with a double-click. Notarization reuses the existing App Store Connect key, so only the signing certificate was new. **`Entitlements.plist` was required, not optional:** notarized apps run under the hardened runtime, which denies microphone access unless entitled — without it dictation would work unsigned and silently fail for every notarized user. Signing degrades gracefully: without the secrets the job still builds unsigned and warns.

**Storage shows where models live** (`models_dir` / `reveal_models_dir`). macOS is the only platform where deleting the app leaves the models behind — and it offers no uninstall hook to ask about them — so instead of an impossible prompt, Storage shows the folder path with an "Open folder" button and says plainly that removing Verdure leaves it. iOS/Android delete their container on uninstall and get the opposite line.

**Chat renders Markdown.** Models emit `**bold**` and lists unprompted (they're trained on it, and telling them not to costs quality), and we were showing it literally. ~40 lines covering what these models actually produce, with the output HTML-escaped first so nothing the model emits can inject markup. User messages stay literal.

**System prompt shortened again.** The 08-14 anti-confabulation version made replies measurably worse on the capable 4B, not just the tiny models — every instruction token competes with the user's question for attention. Cut from five clauses to three lines, keeping "if you don't know, say so."

**CI cost reduced ~90%** — see the changelog for the four leaks. Not user-facing, but it's what makes iteration affordable.

### 2026-08-14 — Beta feedback: three fixes (batch overflow, doubled dictation, confabulation)

First round of real beta-tester reports, all three real:

**1. "It ran out of context — then said so instead of offloading."** A regression from the chat-memory work. `LlamaBatch::new(512, 1)` is a fixed-capacity buffer; before memory, prompts were one message and never approached it, but once history was included they routinely exceeded 512 tokens, `batch.add` failed, and the raw engine error reached the user. **Eviction was working correctly** — the window was sized fine; the batch couldn't hold what the window allowed. Prefill now decodes in `n_batch`-sized chunks (position tracked across them, logits only on the true final token), plus a last-resort clamp for a single message too long to evict (keeps the tail rather than failing).

**2. "The mic keeps transcribing the same output twice."** Leaked event listeners. `startLiveTranscription` registers four listeners and returns a cleanup, but the `onError` path and the 30-second watchdog both ended sessions without calling it — so the next session added a second set and every `live:segment` was handled twice. Sticky: one mic error poisoned every later session until navigation. Every exit path now releases, plus a release at session *start* (the cleanup handle is only assigned once the start promise resolves, so an error mid-start could slip past the per-path fix).

**3. "It keeps giving fake facts."** The characteristic failure of a 0.5B–4B model, not a patchable bug — but the prompt was making it worse: asking only for answers that are "direct, clear and concise" reads to a small model as *always produce a confident answer*, and it complies by inventing one. The instruction now states plainly that "I'm not sure" is a good answer and asks for hedging on dates, numbers, names and quotes — the specifics this tier gets wrong most confidently. **Lowers confabulation; does not eliminate it.**

**Fallout worth keeping:** the instruction tripled in length, which would have silently eaten the history budget. The token reserve is no longer a hardcoded constant — it is the reply budget plus the instruction *tokenized on the actual model* plus slack, so changing the prompt can never quietly cost conversation memory again.

**Verification:** CI green (macOS `cargo test` + iOS/Android cross-compile checks); tsc + vite clean. On-device confirmation of all three pending the next build.

### 2026-08-14 — Mobile builds shipped Tauri's logo, not ours (icon pipeline fix)

**What:** every iOS and Android build since June shipped **Tauri's** yellow/cyan logo on the home screen, despite correct icons in the repo. `gen/` is regenerated from Tauri's template on every CI build (nothing under it is committed), and `tauri <platform> init` scaffolds with the *template's* icons — `src-tauri/icons/` was never consulted. (`tauri icon` updates an existing `gen/` project; in CI the project is created afterwards.) New `app/scripts/install-icons.py` copies our icons in after init, wired into all three mobile workflows.

**Found by:** extracting the shipped `.ipa` and `.apk` and looking inside — not by reading repo files. An earlier check of the repo icons concluded "Android is fine", which was wrong precisely because source and artifact had diverged. **The lesson kept in `skills/design.md`: when the question is "what does the user see", verify the artifact.**

**Fails loudly now:** the script exits non-zero if no generated project is found, or if iOS's asset catalog references a filename we didn't supply. A red job costs one push; the silent version cost six builds.

**Verification:** icons extracted from the `release-11` IPA and APK and visually confirmed — mint V on both.

### 2026-08-14 — App icon: a mint V on navy (replaces the stock Tauri mark)

**What:** Verdure has its own icon — a single mint **V** on the app's own navy background, lit by a soft mint glow, using the "midnight glass" palette exactly (`#34c69a`→`#4cd8ad` gradient on `#050f1e`/`#0c253e`). Regenerated for every platform: macOS `.icns`, Windows `.ico`, the iOS AppIcon set, and all Android mipmaps including the adaptive-icon foregrounds.

**Why:** the icon was still the default Tauri logo from the June scaffold — a white infinity mark that belongs to the framework, not to Verdure. It was the last surface still wearing someone else's identity after the midnight-glass retheme.

**Design decisions worth keeping:** the mark is legible at **32px** (verified against the real generated file, magnified nearest-neighbour, not judged from the 1024 master) — one heavy shape, no detail to lose. Padding clears both the iOS squircle and the Android circle mask. **Two masters:** a full-bleed square for the platforms that apply their own mask, and a rounded-rect variant for the macOS `.icns`, which macOS displays unmasked.

**Reproducible, not a mystery binary:** `app/scripts/make-icon.py` carries the palette and geometry and regenerates both masters; the spec is in `skills/design.md` ("Visual identity"). Changing the brand color means editing three constants, not opening a graphics editor.

**Verification:** rendered and reviewed at 32/64/128px. Ships on the next tag build — icons are bundled at package time, so `release-9` (already out) still carries the old mark.

### 2026-08-12 — Chat memory: the conversation is finally remembered ([OQ-59])

**What:** the chat now remembers the conversation. Three parts of one mechanism (`knowledge/22`): prior turns are sent in the model's own chat template; the model's weights stay **resident** across messages instead of being re-read from disk every turn; and when the window fills, the **oldest whole turns** are dropped — never a partial turn, which would leave a dangling marker and corrupt the structure the model relies on.

**The diagnosis that shaped it:** the chat had no memory *and it was never a context-length problem*. `decide()` built a prompt of exactly `SYSTEM + one user message`, so the 1024-token window sat nearly empty every single turn. The first fix cost nothing from the memory budget — send the turns we already had.

**Sized per device AND per model.** KV-cache cost is exact arithmetic (`2 × layers × kv_heads × head_dim × 2 bytes`), and our models differ by more than 10× — ~12 KB/token on the 0.5B, ~144 KB on the 4B. One hardcoded `n_ctx` was therefore simultaneously too stingy for the small model and too risky for the large one. The window is capped at 4096 by *prefill time* (CPU-bound), not only by memory.

**All three platforms, one formula.** Sizing reads `available_ram_mb`, which each platform already computes its own honest way (macOS `vm_stat`, iOS `os_proc_available_memory()` = real jetsam headroom, Android `MemAvailable`). A tighter device simply gets a smaller window. No per-platform memory code was added.

**User impact:** after this change the user can refer back to what they just said ("what did I ask you?", "make that shorter") and the model actually knows — because the conversation it can see is the conversation it receives. Replies are also faster after the first, since the weights are no longer re-read per message.

**No new setting.** There is deliberately no context-length control: it is the jargon knob Verdure exists to delete (`skills/design.md`). The window is computed silently. If a control is ever added it should be "Remember our conversations: on/off" — a privacy choice the user genuinely owns — and that belongs with cross-session persistence, which is a separate change.

**Deliberately not done:** summarizing evicted turns (`[OQ-68]` — costs an extra model call per fold), releasing the resident model when backgrounded (`[OQ-69]` — benign failure mode for now), KV-cache reuse across turns (needs a self-referential struct; measure before adding that complexity).

**Verification:** 8 new Rust unit tests over the pure logic (budget arithmetic, eviction, both prompt templates); tsc + vite clean; compiled and tested in CI on macOS, plus the new iOS and Android cross-compile checks.

### 2026-08-12 — "Midnight glass" visual identity (styles.css + design.md)

**What:** full retheme from the prairie-daylight palette to the brand video's world — deep-space navy gradient background, frosted-glass panels (`backdrop-filter` blur + mint-tinged borders + glow shadows), mint CTAs as glowing pills with dark ink (never white-on-mint), the VERDURE wordmark as wide-tracked serif with a mint gradient and a one-shot glint sweep, drifting glass orbs in the background, and a staggered fade-up on entry. Palette measured from the video with ffmpeg palettegen (`#021329` navy family, `#34c69a`/`#4cd8ad`/`#b7f3e6` mint family, `#479e96` sea-teal), not eyeballed. Warn/danger moved to warm coral (`#ff9d8a`) — the brick-red vanished on navy. Fonts unchanged and now documented as a decision: `ui-rounded` body + `ui-serif` display is the exact pairing the video uses, and system faces keep the app free of font downloads.

**Why:** founder direction with a brand video — "sleek and simple, Apple but warm." One stylesheet themes all three platforms (macOS/iOS/Android share the webview UI). The canonical identity now lives in `skills/design.md` ("Visual identity — midnight glass"): palette, glass rules, type, motion rules, contrast floor.

**User impact:** after this change, the app feels like one deliberate product with the brand's own atmosphere — and every existing affordance (fit warnings, progress bars, recording states) stays exactly where it was, retinted.

**Motion & floor-machine care:** all animation is entry-only, transform/opacity (GPU-cheap on the 2019 Intel Mac), and fully disabled under `prefers-reduced-motion`. Recording/preparing mic states keep their distinct look (coral pulse vs neutral breathing).

**Verification:** tsc + vite clean. Visual self-review loop actually run this time (Chrome + vite preview): model picker, chat empty state, and a chat exchange reviewed on-screen — wordmark, glass, CTA glow, bubbles all render as intended. On-device confirmation on the next tag builds (blur performance on the Intel Mac is the one thing preview can't prove).

### 2026-08-12 — Help users make room for models ([OQ-67], steps 1–4)

**What:** four space-recovery features, own-sandbox first — Verdure never deletes a byte it didn't create:
1. **Unfinished downloads surfaced** (`model.rs` `partial_bytes`, Storage screen): an abandoned `.part` (up to ~1.4 GB) now shows as "Unfinished download · N MB so far — resumes if kept" with Remove, instead of invisibly eating the disk. Totals include it.
2. **Old-model offer on switch** (`renderFreeOldModel`): switching chat models offers one optional tap to remove the now-redundant old model ("free 1.1 GB"), honest that switching back means re-downloading. Never automatic.
3. **Android "make room"** (`free_up_space` command, JNI): when a model doesn't fit the disk, one tap asks Android's `StorageManager.allocateBytes` to clear *other apps' clearable cached files* — the OS decides what's junk; we never see or touch anyone's data. `getAllocatableBytes` is checked first so the failure message is honest ("can clear about X MB — needs Y MB").
4. **macOS/iOS honest pointers**: macOS gets "See what's filling your disk" (opens the System Settings Storage pane, falls back to System Settings if the community-documented pane id is refused); iOS gets instructions in the warning copy (Settings → General → iPhone Storage), since no public API or deep link exists.

**Why:** real user reports — not enough space to install any model. The app detected and declined honestly but left the user alone with the problem. Platform survey and the no-cleaner decision (duplicate-finders rejected) are in `[OQ-67]`.

**Problem it solves:** the ~550 MB (minimum) to ~1.5 GB (Powerful) a user needs free, recovered from space Verdure itself created first, then from OS-sanctioned mechanisms only.

**Verification:** new Rust unit test (partial-download surfacing + reclaim); tsc + vite clean; macOS Rust compiles in CI on push. The Android JNI path compiles only in the Android workflow (tag build) and needs on-device confirmation on the Pixel 8a — tracked in `[OQ-67]`. macOS deep-link pane id is provisional with a safe fallback.

### 2026-08-10 — Friendly error fallback at the display boundary (`main.ts`)

**What:** the three places that showed a caught error verbatim (`String(err)` — mic setup, chat send, legacy porter button) now route through one `friendlyError` helper: backend messages (already human-written sentences from `llm.rs`/`model.rs`) pass through, but a thrown object or empty string shows "Something went wrong. Please try again." instead of `[object Object]` or a blank.

**Why:** the truck-driver test — a raw JS error representation is jargon by definition. This was an inspection finding (2026-06-29, deferred to the redesign; re-flagged 2026-08-10) — the backend half was already honest, the frontend just leaked whatever it caught.

**Verification:** tsc + vite clean. The chat-send path's resume-detection regex is unaffected (string errors pass through unchanged). Frontend-only; no Rust touched.

### 2026-08-03 — Android Play distribution: signed-AAB workflow (open-testing path)

**What:** the Android app now has a Play-ready build path. New `build-android-play.yml` (tag `android-play-N`) produces a **signed Android App Bundle (`.aab`)** using a **persistent upload keystore** held as GitHub secrets (`ANDROID_KEYSTORE_BASE64` + password/alias secrets — founder-owned, same trust model as the iOS App Store Connect keys). Gradle emits an unsigned `.aab`; we sign it with `jarsigner` (the JAR-signing scheme AABs use) so Google **Play App Signing** can take over (upload key → Play re-signs delivered APKs). This is the Android analog of `build-testflight.yml`; the throwaway test-APK workflow stays for quick sideload smoke tests.

**Why now:** the APK is proven on-device (mic works), which was `[OQ-66]`'s gate for moving to a shareable link. Target is Play **open testing** — a public "click → install → auto-update" link, no tester-email collection. Verdure's local/no-telemetry design makes the Data Safety form the best case ("no data collected/shared") and `RECORD_AUDIO` trivially justified (on-device dictation).

**Refactor (no drift):** the manifest + `MainActivity` patch (RECORD_AUDIO + the `initAudioContext`/`ndk_context` mic fix) moved out of the inline workflow heredoc into one shared script — `app/scripts/patch-android-project.py` — that **both** Android workflows run, so the test-APK and Play-AAB paths can never diverge on the mic wiring.

**Founder-side (account, not code):** generate the upload keystore + add the four secrets; then in Play Console create the app under the permanent package name (`ai.verdure.porter` unless changed first — permanent once published), fill the store listing / Data Safety / content rating / privacy-policy URL, upload the first `.aab` by hand to establish Play App Signing, and open the **open-testing** track for the public link. Automated Play upload (a service-account step) folds in after the app exists.

**Verification:** both workflow YAMLs parse; the shared patch script syntax-checks. The `--aab` cross-build + `jarsigner` step run once the keystore secrets are set and an `android-play-N` tag is pushed (CI is the compiler here). `[OQ-66]`.

### 2026-08-02 — Android mic fix: initialize the `ndk_context` oboe reads (root cause found on-device)

**Problem (on-device Android, `[OQ-66]`):** the live mic never worked — tap the mic, it hangs on "preparing" and the 30s watchdog fires; no text, no error. Debugged on-device with `adb logcat`. Ruled out permission (force-granting `RECORD_AUDIO` via `adb` didn't help) and the model (Whisper `base.en` loads cleanly in the log). The smoking gun:

```
thread 'verdure-live' panicked at ndk-context-0.1.1/src/lib.rs:72:30:
android context was not initialized
```

**Root cause:** cpal's Android backend (oboe) enumerates the microphone over JNI and reads the app's `JavaVM` + `Context` from the process-global `ndk_context`. **Tauri v2 never populates that global.** So the first mic-open on the capture thread panics, the thread dies, and — because a panic emits no `live:error` — the UI just hangs until the watchdog. macOS/iOS never hit this (CoreAudio, no JNI); Android is the only platform on this path.

**Fix:** initialize `ndk_context` ourselves at launch. `MainActivity.onCreate` now calls a native `initAudioContext(applicationContext)` (`lib.rs` `Java_ai_verdure_porter_MainActivity_initAudioContext`, `#[cfg(android)]`), which stores the `JavaVM` + a global ref to the Application context into `ndk_context` (once, leaked deliberately — the Application context is process-lifetime). New Android-only deps (`jni`, `ndk-context`), decoupled from cpal via raw pointers; the macOS dependency tree is untouched (target-gated). The CI `MainActivity` patch gained the `external fun` declaration + the call, plus a package assertion so a future Tauri package rename fails the build loudly instead of an `UnsatisfiedLinkError` at runtime.

**Verification:** the porter compiles on macOS CI (Android code is cfg-gated out there); the Android cross-compile + APK build is exercised by the `android-build-7` tag. **Confirmed working on-device (Pixel 8a, 2026-08-03):** the `ndk-context` panic is gone, `AudioRecord` opens (oboe negotiates a channel config — the `getMinBufferSize: Invalid channel configuration` probe lines are benign), the mic goes to "Listening…", and live transcription streams. Live dictation now works on all three platforms (macOS/iOS/Android). Closes the core mic unknown of `[OQ-66]`.

### 2026-08-01 — Download UX: live progress bar + friendly one-tap resume; mic button states

On-device Android polish:
- **Progress bar.** The resumable download now emits `PROGRESS <done> <total>` MB lines (`model.rs` `download` computes total from Content-Range/Content-Length, falls back to the model's approx size); the chat install card renders a filling bar + "377 of 400 MB · 94%" instead of a static "installing…" line.
- **Friendly pause + resume.** An interrupted download no longer shows a raw "os error 103" — it shows "Download paused — your progress is saved" with a **Resume download →** button that re-attempts (which resumes from the `.part` on disk). Refactored `submit` → `runChat(text)` so resume re-runs without re-posting the question.
- **Mic button states** (from the same review): idle → **preparing** (neutral dim) → **recording** (red) — the mic only goes red once capture is really live, and a 30s watchdog replaces an infinite "Starting the microphone…" hang with an actionable "allow microphone in Settings" message. The Android build also re-requests mic permission on `onResume`.

**Verification:** frontend `tsc` + `vite` clean; the `download`/`ensure` signature change compiles in CI. On-device confirmation in the next Android build. `[OQ-66]`.

### 2026-07-31 — Resumable model downloads + keep-screen-awake (mobile reliability)

**Problem (on-device Android):** big-model downloads (1.4 GB) never finished — when the screen sleeps or the app backgrounds, Android suspends the process and tears down the socket (`ECONNABORTED` / "Software caused connection abort, os error 103"). Worse, the old flow *deleted* the partial file on any interruption and restarted from zero, so a big model over mobile never caught up.

**Fix — resumable downloads (the robust foundation, all platforms):** `model.rs` `download()` now resumes from the existing `.part` file — macOS via `curl -C -`, iOS/Android via an HTTP `Range: bytes=N-` request (served by Hugging Face's CDN; 206 → append, 200 → restart). `ensure()` keeps the `.part` across attempts (no longer deletes on interruption) and reports "Resuming — N of ~M MB…". So every interruption just costs a pause: reopen Verdure and it continues from where it stopped, instead of restarting.

**Fix — keep screen awake during download:** a best-effort Screen Wake Lock (`withScreenAwake`, no-op where unsupported) is held while a model downloads, so an idle screen doesn't suspend the app mid-download. Install copy updated ("Keep Verdure open; this can take a few minutes").

**Deferred (the gold standard):** a true background download (Android foreground service / DownloadManager) so it continues with the app fully closed — real Kotlin/plugin work, noted for later. Resume + wake-lock covers the common case now.

**Verification:** frontend `tsc` + `vite` clean; the `download`/`ensure` changes compile in CI. On-device confirmation (big model finishing across an interruption) in the next Android build. `[OQ-66]`.

### 2026-07-31 — Android test build green (APK via CI) [OQ-66]

**What:** Verdure builds for Android — the same one-interface app, same shared Rust core and GGUF flow, cross-compiled for `aarch64-linux-android` via the NDK. Same-branch `cfg(target_os = "android")` gating; macOS/iOS paths untouched (Mac CI stayed green). New `build-android.yml` (tag `android-build-N`, **Linux runners — 1/10 the macOS cost**): `tauri android init`, manifest `RECORD_AUDIO` + runtime permission request patched into MainActivity, NDK build, zipalign + apksigner with a throwaway key → 9 MB installable `.apk` artifact.

**Key finding: whisper.cpp + llama.cpp + cpal cross-compile for Android** — proven in CI (the decisive `[OQ-66]` unknown; **NOT MediaPipe** — the shared Rust runtime ports directly). Confirms the same "compile the engines in, wrap them" approach works on a third platform.

**Backend (cfg-gated `device.rs`):** total RAM via `libc::sysinfo`, available via `/proc/meminfo MemAvailable` (pure parser + unit test), disk via `statvfs`, ARM tier default. Frontend: Storage screen names ARM correctly (was hardcoded Intel/Apple), plus a phone-width CSS pass (~360–412px) since Android phones are narrower than the iPad.

**Two rounds (iOS-style iteration):** round 1 — native compile fully succeeded, Kotlin failed because the MainActivity permission patch rewrote the class (the template already has an `onCreate`); round 2 — inject at the stable `super.onCreate` anchor → green.

**Install:** sideload the APK (enable "install unknown apps"). Test-signed with a per-run throwaway key, so uninstall an old build before installing a new one. Play-Console testing link (shareable URL) comes next with a persistent upload keystore.

### 2026-07-27 — On-device fixes: plain chat (no JSON tool-calling), picker delete, disk fail-states

**First real on-device run (TestFlight build 6) — big signal:** the chat model **loads and generates on the iPad** (wrote a poem, answered "Hey"). `[OQ-64]`'s core question — does the model run on-device — is **answered YES**. Two bugs surfaced, both from the JSON tool-calling wrapper:

**Bug 1 — chat replied with a transcription prompt to normal questions** (a poem request → "I can set you up to turn recordings into text"). The tiny model over-triggered the `set_up_transcription` tool-action. **Bug 2 — raw JSON leaked into the chat** (the poem came out as `{"action":"answer","reply":…}`) when the model emitted malformed/nested JSON that the lenient parser dumped verbatim.

**Fix — chat is now a plain conversation.** Removed the JSON tool-calling entirely: `llm.rs` system prompt is plain ("answer directly, clearly, concisely"), `decide()` returns the generated text as the reply (no JSON parse, no `parse_decision`/`extract_json_object`), and the UI always shows the reply (removed the `set_up_transcription` action branch + `classifyIntent` cross-check). Transcription is the mic button; capabilities-as-tools returns later, constrained + process-isolated (`[OQ-55]`). Also filter Whisper's non-speech markers (`[BLANK_AUDIO]`, `[ Silence ]`) out of dictated text.

**Model management (founder ask):** added a **Remove** button to each downloaded model **in the picker** (delete one to free room for another, in one screen), a **disk fail-state warning** ("Not enough free space to download (needs X). Remove a model to make room"), and a **free-space readout** at the top of the picker (space + memory free) — the Storage-screen transparency, where you choose.

**Verification:** `tsc` + `vite build` clean; `llm.rs` prompt-framing tests kept, obsolete parser tests removed; Rust compiled in CI. Shipped in TestFlight build 7.

### 2026-07-25 — Dictation-into-chat, model picker, install notice

Three chat improvements:

**1. Mic dictates into the chat box** (not a separate screen). The chat mic is now a record toggle: tap → live transcription streams words into the input as you pause, tap again to stop, then edit + Send. Reuses the `live.rs` engine (`startLiveTranscription`/`onSegment`); the mic pulses red while listening. The standalone transcription screen stays for file transcription (still reached from the "set it up" chat action), just not behind the mic. Closes the core of the AI-first dictation vision (`[OQ-57]`).

**2. First-launch model picker** (Small · Balanced · Powerful). New `Choose your model` screen shows the three chat models with plain tier names, one-line blurbs, **download size, and whether each fits this device's free memory** (design.md — honest space, model name demoted). Choice persists (`localStorage`), is shown as a chip in the chat header, and is changeable anytime. Backend: `chat_model_options` command (size + `fitsMemory`/`fitsDisk` per device) and `chat_model_choice(id, available)` — loads the picked model if it fits, else declines honestly; unknown/empty id falls back to the auto-picker. Replaces the old "best model only" toggle (the explicit pick supersedes it). The lineup is all-lightweight (0.4 / 1.1 / 1.4 GB) per the founder's call.

**3. "Installing your model" notice.** On the first message with an un-downloaded model, a prominent one-time notice names the tier + size ("Setting up your Powerful model — a one-time 1.4 GB download…") instead of a silent wait.

**User-impact statement:** *the user talks or types to Verdure, sees clearly which model they're using and what it costs in space, and is told plainly when a one-time download is happening.*

**Verification:** `tsc` + `vite build` clean; new `model.rs` unit tests for the explicit-choice + options logic. Rust compiled in CI (below); on-device behavior (real dictation, real download) confirmed in the next TestFlight build.

**Follow-up (same day, on founder review):** deleted the standalone transcription screen entirely (`renderTranscribe` + file-drop/tabs/speed-estimate helpers, ~280 lines) — dictation lives in chat now, so the page was vestigial. The mic now **asks permission before installing the speech model**: first tap (when Whisper isn't downloaded) shows a consent message with the size ("~140 MB, runs on your device") and an "Install & start talking" button, matching the chat-model install prompt; confirm → download → record. The chat "set up transcription" action routes into that same mic flow instead of the deleted page. Unused transcription CSS left for a later sweep.

### 2026-07-25 — TestFlight distribution working (signed iOS build via CI) [OQ-65]

**What:** `build-testflight.yml` (tag `testflight-N`) now builds a **code-signed** iOS app with the founder's paid Apple Developer account and uploads it to **TestFlight** — replacing the unsigned-sideload path (no more 7-day cert, no Sideloadly). Reuses the proven iOS build steps, then: patch build number (run number) + `ITSAppUsesNonExemptEncryption=false`, stage the App Store Connect API key, archive with Xcode-managed automatic signing, export a signed App-Store `.ipa`, upload via `xcrun altool`.

**Problem it solves:** the founder's 8 GB Mac can't run the models; the iPad can, but sideloaded builds expired weekly and needed a Mac to re-sign. TestFlight installs through the TestFlight app, no expiry, and can add testers — the sustainable test channel.

**The four-round debug (all account-side, no code bugs):** (1) export failed "Cloud signing permission error" — the **App Manager** API key can't do cloud *distribution* signing; fixed by regenerating the key as **Admin**. (2) Two agreements (Paid Apps + EU trader) were unaccepted, which also blocks distribution signing. (3) Upload failed "Cannot determine the Apple ID from Bundle ID `ai.verdure.porter`" — the build signed as `ai.verdure.porter` (Xcode-managed XC App ID) but the "Verdure AI" App Store record is bound to the explicit App ID `ai.verdure.porter.898PY37K5S`; fixed by overriding the iOS bundle ID at archive time to match (macOS build + `tauri.conf.json` untouched). (4) Green end-to-end.

**Boundary honored:** all signing credentials are the founder's, created by them and added as GitHub secrets (`ASC_API_KEY_P8` / `ASC_API_KEY_ID` / `ASC_API_ISSUER_ID` / `APPLE_TEAM_ID`); Claude never handled a private key.

**Follow-up:** the dual-App-ID mess (`ai.verdure.porter` vs `…898PY37K5S`) is cosmetic debt to fold into the identifier rename (`[OQ-60]`). `build-ipad.yml` (unsigned) kept as a fallback.

### 2026-07-17 — Chat-first redesign + "prairie" aesthetic

**What:** Reworked the app's front door and visual language. (1) **Opens straight into chat** — the 4/6-tile task grid is no longer the entry (`boot()` → `renderChat()`); chat is the home. (2) A **microphone button** in the chat input row opens dictation (the Whisper flow) in one tap. (3) A quiet **gear icon** in the header reaches Storage; back-nav from Storage/Transcribe now returns to chat. (4) New **"prairie on a summer day" palette** — a layered gradient background (summer sky → sun-glow → meadow-green → wheat-cream, fixed), warm meadow green replacing cold emerald, earth-olive ink, a sun-dot brand mark, soft gradient bubbles/buttons, frosted warm-paper panels. Retinted through the CSS design tokens so the change cascades everywhere at low risk, with targeted gradient overrides on the hero surfaces (background, bubbles, buttons, mic). Icons are inline SVG (CSP-safe, no remote assets).

**User-impact statement:** *After this change, the user lands where the value is — a calm, warm chat they can talk or type to — instead of a cold grid of choices; dictation is one tap away.*

**Problem it solves:** the old front door made the user choose a "task" before doing anything, and the visual tone was hard-edged/clinical. The redesign removes a whole decision step (design.md — "remove a step") and softens the feel to something inviting.

**Legacy:** the task grid + recommendation/porter flow (`renderHome` and friends) is retained but **unwired** — the only path to un-wrapped modalities (image/video) — pending the legacy-porter keep-or-remove decision (`[OQ-60]`). Flagged in-code.

**Verification:** `tsc --noEmit` clean; `vite build` clean. **Visual self-review (design.md) NOT automated** — the browser extension wasn't connected and no headless browser is installed; reviewed via local `vite preview` and to be confirmed on-device in the next iPad build. No new unit tests (pure presentation; no logic changed — the model picker, chat command, and parsers are untouched).

### 2026-07-04 — iPad test build: the app runs on iOS (unsigned IPA via CI) [OQ-65]

**What:** A temporary iPad port of the whole app as a **test vehicle** — the founder's 8 GB Mac can't exercise the ByteShape 4B (`[OQ-64]`); his iPad can. Deliberate, flagged deviation from `knowledge/11`'s thin-decider iOS strategy (the shippable-iOS decision stays with `11`). Backend made iOS-compatible behind `cfg` gates with the **macOS paths byte-identical**: `model.rs` downloads via in-process `ureq`/rustls on iOS (no process spawning there; macOS keeps its on-device-verified `curl`); `device.rs` reads syscalls instead of shell-outs — `sysctlbyname` (RAM), `statvfs` (disk), and `os_proc_available_memory()` — the app's **real headroom before iOS jetsam-kills it**, feeding the existing chat RAM gate so the picker sizes against iOS's true constraint. New `build-ipad.yml` (tag `ipad-build-N`): `tauri ios init` fresh each run, mic-permission plist injected, Rust lib pre-built with cargo (bypassing the generated xcode-script phase, which requires a parent tauri-CLI process CI can't provide), project rewritten to link the prebuilt lib, xcodebuild unsigned → `verdure-ipad-unsigned.ipa` artifact.

**User-impact statement:** *After this change, the founder can test chat and live transcription on the one device he has that can actually run the models — because CI now produces an installable iPad build without needing a local toolchain or an Apple Developer account.*

**Problem it solves:** every on-device unknown (`[OQ-64]`: does the 4B load, is it usable) was blocked on a machine that can no longer run the app. This unblocks testing entirely.

**Key finding:** **whisper.cpp + llama.cpp + the full backend cross-compile for `aarch64-apple-ios`** — proven in CI (the decisive unknown of the port, answered in round 2 of 3).

**Install:** sideload with a free Apple ID (Sideloadly/AltStore; 7-day cert) — no $99 account. See `knowledge/21` "How to build / ship".

**Not verified (on-device, tracked in [OQ-65]):** jetsam ceiling vs the ~2.3 GB 4B footprint; cpal/AVAudioSession mic capture; TCC mic grant for a sideloaded app; WebView UI fit on iPadOS.

### 2026-07-01 — "Best model only" chat toggle (kills silent fallback for testing + honesty)

**What:** A chat setting — *"Use the best chat model only — tell me if it won't fit, don't switch to a smaller one"* (persisted in `localStorage`, off by default). When on, chat uses the preferred model (the ByteShape 4B) or **declines with an honest message naming the model and the memory shortfall**, instead of silently downgrading to Qwen 0.5B. Backend: a pure `model::choose_chat_model(available_mb, allow_fallback)` returning either the chosen model or a `ChatModelTooBig` (carries model name + needed vs available MB + a `strict` flag); the `chat` command gained an `allow_fallback` arg and formats the message in human GB. Factored the candidate ordering into one `chat_candidates()` list shared by the RAM picker and the preferred/strict pick, so they can't drift.

**User-impact statement:** *After this change, the user can tell exactly which chat model is running — because best-only mode either uses the 4B or says why it can't, instead of quietly substituting a weaker one.*

**Problem it solves:** silent fallback made the 4B impossible to test — a reply might be the 4B or the 0.5B and you couldn't tell. Now best-only mode is a deterministic lever: a reply means the 4B ran; the decline message means it didn't fit (and says how much memory it needs). It's also better UX per "honest performance / feedback" — no silent downgrade behind the user's back.

**Tests:** three new `model.rs` unit tests — preferred = top candidate; best-only uses-preferred-or-errors-strict (no downgrade even when a smaller model would fit); fallback downgrades and only errors (non-strict) when nothing fits. Frontend `tsc` + `vite build` clean.

**Not done:** visual self-review of the toggle (no browser tooling / Tauri toolchain locally) — verify on-device in the tagged build. A general "which model is active" indicator (for fallback mode too) is a natural follow-up, deferred to the redesign's chat UX pass.

### 2026-07-01 — ByteShape Qwen3 4B as the base chat model (`model.rs`)

**What:** Wired [ByteShape](https://byteshape.com/)'s ShapeLearn-quantized **Qwen3-4B-Instruct-2507 `Q3_K_S-2.77bpw`** (~1.4 GB, Apache 2.0) as the top-of-pick chat model in the RAM-aware picker, with the Qwen 0.5B/1.5B pair kept as the always-present floor/fallback (`USE_BYTESHAPE_QWEN3_4B = true`). Refactored `chat_model_for_available` from nested conditionals to a flag-built candidate list so adding a gated model no longer amplifies edits. Added its URL to the CI resolve check. Qwen3 uses ChatML, so the existing `PromptStyle::ChatMl` framing applies unchanged — no new prompt plumbing.

**Problem it solves:** the 8 GB Intel Mac forced chat onto Qwen 0.5B, which produced garbage; a *real* 4B is far more coherent, and ByteShape's low-bit quant squeezes it to ~1.4 GB so it can plausibly fit the machine's available RAM. The direct lever on "chat doesn't work."

**Why it's the safe route:** the GGUF's base type is a *standard* llama.cpp K-quant (ByteShape's "KQ" = per-tensor mixing on stock GGML kernels — no engine fork), and Qwen3 (2025) is within our pinned `llama-cpp-2 0.1.150`'s arch support (it already runs Apr-2026 Gemma 4). Enabling it can't strand a tight machine: if the 4B doesn't fit available RAM it's simply not picked and Qwen answers instead. One-line revert (`USE_BYTESHAPE_QWEN3_4B = false`) if an on-device run shows it fails to load.

**Not yet confirmed (on-device):** (a) that it *loads* on our pinned llama.cpp — low risk, but a load failure OOM-aborts the app until process isolation lands (`[OQ-58]`); (b) whether it *fits* the founder's 8 GB Mac at all — with ~1.5 GB free the gate falls back to Qwen, so the 4B needs RAM freed to be exercised. To help, `N_CTX` was lowered 2048 → 1024 (`llm.rs`) and `runtime_mb` set to match (~2300, provisional, to calibrate on-device like the predictor `19`) — but the 1.4 GB of weights is a hard floor, so no context cut makes it fit under ~2.3 GB free. Tracked as `[OQ-64]`.

**Verification:** can't compile on this Mac (build gated on Xcode CLT); pure-logic covered by new `model.rs` unit tests (base-when-it-fits + fallback-when-it-doesn't + ChatML style); CI compiles the backend, runs the tests, and resolves the new URL on push.

### 2026-06-26 — Chat: real sampling (fixes the `uids!!!!` garbage + slowness)

**What:** Replaced the pure-greedy sampler in `llm::decide` with a proper chain — repetition penalty → top-k → top-p → light temperature → distribution pick (`penalties(64, 1.1, 0, 0)`, `top_k(40)`, `top_p(0.9)`, `temp(0.6)`, fixed seed). Lowered `MAX_NEW_TOKENS` 512 → 256.

**Problem it solves (observed 2026-06-26):** with greedy argmax, the q4 0.5B chat model collapsed into a repetition loop — every reply was `uids!!!!!!!!!!!…`. Greedy is deterministic argmax with no penalty, so once a tiny quantized model (pushed off-distribution by the JSON-forcing system prompt) latches onto a token it repeats it forever. Worse, the loop **never reaches an end-of-turn token**, so generation ran the full 512-token budget every message — which is *also* why it was slow. One cause, both symptoms. A repetition penalty breaks the loop; a light temperature keeps replies natural; the model can then actually stop (EOG fires early). This stays within the no-grammar constraint from the crash fix below — plain sampling, lenient parse, UI cross-check unchanged.

**Not the whole speed story:** `llm::decide` still reloads the model from disk every message (~0.4–1 GB) — `[OQ-59]` (resident model in Tauri state) is the rest of the latency fix.

**Verification:** could not compile on this Mac (build gated on Xcode CLT — see `[OQ-61]`/handoff); the existing `parse_decision` tests are unaffected and the sampler API was checked against `llama-cpp-2` 0.1.150 docs. On-device run still pending.

### 2026-06-26 — Model picking now weighs *active* compute, not just total params (`knowledge/03`)

**What:** Added an "Active parameters, not just total: the speed axis" section to `03-model-matrix.md` and a Gemma 4 E2B model entry. The matrix keyed only on total params (= footprint); this captures that **generation speed tracks the params activated per token**, which sparse-activation models (MoE; Gemma 4's Per-Layer Embeddings) keep far below their stored size. Net rule: *total params → does it fit; active params → does it feel fast.*

**Why (user request):** evaluating Gemma 4 E2B for the chat front door because sparse activation should give snappy short-prompt replies. Research surfaced the catch worth recording: E2B is ~2.3B *effective* but a **~3.1 GB** Q4_K_M file — the <1.5 GB active footprint needs runtime PLE offload (unverified in our pinned `llama-cpp-2`), and it uses Gemma's chat template, not ChatML. So it's a *roomy-tier* pick, **not** a floor replacement for the 0.5B. Tracked for adoption as `[OQ-61]`.

**Code wiring (decision: "add as roomy-tier, verify first"):** registered `GEMMA4_E2B` in `model.rs` (in `MODELS` so Storage manages it uniformly), but held OUT of the live pick behind a `GEMMA4_E2B_VERIFIED` flag (currently `false`) so the picker never auto-selects an unverified model that could OOM-abort the app. Added a `PromptStyle` (ChatMl | Gemma) that travels with each model; `llm::decide` + `format_tool_prompt` now branch on it (Gemma uses `<start_of_turn>`/`<end_of_turn>` + a tokenizer-added BOS, no system role). The template formatting is **the part testable without a model**, so it's unit-tested now; flipping the model on after on-device verification is then a one-line change. The current shipping default is unchanged (the Qwen pair).

### 2026-06-23 — The real chat-crash fix: remove the GBNF grammar sampler

**What:** Removed the grammar-constrained sampler from `llm::decide`. The model now samples plainly (greedy) and `parse_decision` is **lenient** — it extracts the first `{...}` JSON object if present, otherwise treats the whole output as a plain answer. Tool routing does not depend on perfect JSON: the UI's deterministic `classifyIntent` cross-check is the real router.

**Problem it solves — the actual chat crash.** A **symbolicated** crash report (2026-06-23) showed chat's `SIGABRT` was **not** memory: the trace is `ggml_abort ← llama_grammar_reject_candidates ← llama_grammar_apply_impl ← llama_sampler_sample ← llm::decide`, with only 716 MB malloc'd. llama.cpp's GBNF grammar aborts the **whole process** when it reaches a state with no valid continuation (e.g. a multi-byte token the `char` rule can't extend). The grammar we'd added "for reliability" was the crash. (My first diagnosis blamed RAM off an *unsymbolicated* report + the prior 1.5B OOM history — wrong; corrected here.)

**Why removing it is right:** the grammar was redundant — `classifyIntent` already routes transcription deterministically regardless of the model's output — and actively dangerous (an in-process `abort()` is uncatchable). Plain sampling + lenient parse + the cross-check is safer and simpler. A grammar can return only once the model runs in an **isolated process** (knowledge/21 — now the top robustness P0).

**Tests:** 3 new `llm.rs` parse tests (plain prose → answer; JSON wrapped in prose → extracted; unknown action → answer) alongside the existing two.

### 2026-06-23 — Live machine-context + memory-aware model picking

**What:** Verdure reads a **live snapshot of the machine's free resources** — available RAM, free disk, cores, arch, Low Power Mode (`device::system_resources`, `system_resources` command) — and uses it to **pick the chat model that fits the memory free *right now***, declining honestly when nothing fits. A "Your computer" readout on the Storage screen makes that context visible.

**What it does and doesn't fix:** this is the machine-as-context foundation the user asked for, and it prevents a *genuine* OOM-abort when memory is truly tight. It did **not** fix the chat crash that prompted it — that was the grammar sampler (see the entry above); memory was fine (~2 GB free, 0.5B fit). Keeping it anyway: it's the right substrate for honest orchestration ("you at the core of context", knowledge/20), extending the static `device.rs` specs + the transcription predictor (knowledge/19) to *live* resources.

**Architecture (`skills/architecture.md`):** the `vm_stat`/`df`/`pmset` parsers are **pure** so they're unit-tested; the I/O wrappers shell out to fixed paths (PATH-safe, like the existing sysctl). `runtime_mb` is a per-model footprint estimate (0.5B ≈ 1100 MB, 1.5B ≈ 2400 MB) — **provisional, to calibrate on-device**.

**Tests:** 4 new `device.rs` parser tests (vm_stat available-memory math, df column, low-power, missing-page-size fallback) + 1 `model.rs` test (picks larger when roomy, small when tight, declines when nearly full). Frontend typechecks; Storage screenshot shows the readout.

### 2026-06-23 — Live (microphone) transcription — speak, watch text appear

**What:** A second way into on-device transcription, alongside drop-a-file: **record live from the mic**. The transcribe panel gained an **Upload a file / Record live** toggle. In live mode, a Start/Stop button captures the microphone (`cpal`, pure-Rust CoreAudio) and streams finished sentences into the transcript as you speak, entirely on-device. A red recording button + pulsing dot + elapsed timer make the recording state obvious.

**Problem it solves:** the file path only helps if you already *have* a recording. Live capture is the everyday case — a meeting, a voice note, dictation — without any other app. It's the foundation the roadmap's "AI-first dictation" (knowledge/21 P2) builds on.

**Architecture (`skills/architecture.md`):**
- **Whisper isn't streaming, and the floor device is CPU-bound** (Intel Mac, base.en ≈ 3.9× real-time). Re-transcribing a growing buffer every second would fall behind. So `live.rs` **segments and commits**: a pure, unit-tested `Segmenter` (energy-gated VAD-lite) cuts the mic stream at natural pauses — never mid-word — and each segment is transcribed **once** as it completes. This keeps up with real time on this hardware; text lands a few seconds after each pause (the normal live-captioning cadence).
- **Resident model per session.** The Whisper context is loaded **once** when recording starts (`transcribe::load`) and reused for every segment — no per-segment reload. A step toward the broader "resident model" debt in knowledge/21.
- **cpal `Stream` is `!Send`**, so the capture stream is built, used, and dropped on one owned thread; samples flow to the worker over an `mpsc` channel; an `AtomicBool` + a join (in Tauri managed state) make Start/Stop clean. Mic frames reuse the file path's `downmix_to_mono`/`resample_linear` (one tested audio core).
- **Events mirror the existing pattern:** `live:status` / `live:segment` / `live:error` / `live:stopped`, consumed in `bridge.ts` exactly like `transcribe:progress`.

**UX (`skills/design.md`):** one task ("Turn speech into text"), two inputs behind a calm toggle; the file emoji moved to 📄 so 🎙️ now means *record*. Honest browser-preview simulation so the flow is reviewable without a mic.

**Permissions:** `src-tauri/Info.plist` adds `NSMicrophoneUsageDescription` (plain, no-harvesting language) — required or macOS kills the app on mic access instead of prompting.

**Robustness caveat (carried, knowledge/21 gap #4):** ggml can `abort()` the whole process on a model error, and live mode runs Whisper repeatedly — raising that exposure. The durable fix (runtime in a separate process, P0 #3) is **not** in this change; the model-file validation in `model::ensure` is the only guard for now. Logged as the next hardening step.

**Tests:** 5 new Rust unit tests for the pure `Segmenter` (cuts on a pause after speech, respects the minimum length, force-cuts a non-stop talker, discards pure silence, flushes trailing speech). Frontend typechecks + a new `2b-live.png` visual-review shot of a mid-session live transcript.

**Status:** 🟡 Built; **on-device confirmation pending** (a tagged Intel build on the founder's Mac). CI can't exercise a microphone, but it compiles + cross-compiles the new `cpal` path and runs the segmenter tests. A specific risk to verify on-device: an unsigned/ad-hoc `.app` must still be granted macOS Microphone permission. See TEST_REPORT.md.

### 2026-06-20 — Chat front door + grammar tool-calling + RAM-gating

**What:** A local chat model (llama.cpp via `llama-cpp-2`, compiled in) is the conversational front door. It both answers and **orchestrates**: it emits a **GBNF-grammar-constrained** tool call (always-valid JSON — `answer` or `set_up_transcription`), `intent.ts` cross-checks, and the user confirms before Verdure acts. Model is **RAM-gated** (`model::chat_model_for`): Qwen2.5 0.5B on ≤8 GB, 1.5B on ≥12 GB.

**Problem it solves:** "apps for AI" step one — capabilities the model invokes, not just buttons. Small-model unreliability is handled by the grammar (can't malform a call) + the deterministic cross-check + a confirm step (human in the loop).

**Crash fixed:** the 1.5B model OOM-aborted an 8 GB Mac (ggml calls `abort()` on a failed alloc, killing the app). Cut chat context 4096→2048 + batch 512, validated model-file size, then **RAM-gated to 0.5B** on ≤8 GB. (Deeper fix — process isolation so ggml can't crash the UI — is a logged P0 next step.)

**Tests:** `llm.rs` parse tests; CI compiles + cross-compiles llama.cpp (the `common` feature gates the grammar samplers; deployment-target fix shared with whisper). On-device 0.5B confirmation pending.

### 2026-06-19 — Performance predictor (predict from specs, calibrate from runs)

**What:** `packages/decider/performance.ts` estimates a model's speed/feasibility from detected specs (`device.rs` reads cores/RAM/arch → `device_spec`), shown before download as a category + "× real-time" range, then **replaced by the real measured speed** after a run (`transcribe_audio` returns timing). Anchored to a real measurement (founder's Mac: base.en ≈ 3.9× real-time, RTF 0.254). `bestModelFor` picks per device.

**Problem it solves:** honest performance without making users benchmark (resolves the data gap behind `[OQ-51]`/`[OQ-03]`). 10 decider tests.

### 2026-06-19 — In-app transcription (whisper.cpp) + storage management

**What:** Drop an audio file → text, entirely in Verdure. `audio.rs` decodes mp3/m4a/wav/flac via symphonia (no ffmpeg) → 16 kHz mono; `transcribe.rs` runs whisper.cpp (`whisper-rs`, compiled in) on CPU. `model.rs` is a **registry** (data) — models download on first use (curl) to app data, with a size-validation guard. **Storage**: per-model size + Remove (in the transcribe panel) and a global "Manage storage" screen.

**Problem it solves:** the first wrapped modality — proves the pivot (model in one UI, no separate app, no Homebrew/CLT). Storage control is first-class for the floor user's tight disk. Verified on-device (~3.9× real-time). Rust unit tests for the resampler/downmix, catalog, model registry.

## MVP desktop porter (`app/`) — the original build (pre-pivot, now legacy)

### 2026-06-16 — The Buzz slice: intent → one-click port (v1.0 first slice)

**What:** A Tauri desktop app (`app/`) that proves Verdure's atomic unit — *one OSS tool installed & running, from the user's intent, in one click* (`knowledge/17-build-plan.md`). Two screens: pick a task (the reused intent cards) → see the recommendation and press **Set it up**. The Rust backend (the "porter") installs the tool from its official source and launches it. First wired tool: **Buzz** (transcription), via the Homebrew cask, launched with `open -a Buzz`.

**Problem it solves:** The website tells a non-technical user *what* to use; it can't *do it for them*. The porter closes that gap — the install/launch a normal person can't be expected to perform unattended happens on one click, locally, free, no account.

**Architecture (`skills/architecture.md`):**
- **The decision is not re-implemented.** The frontend imports `@verdure/decider` (the same `recommend()` the website uses) — single source of truth, no drift (resolves `[OQ-49]`; see the decider extraction below).
- **The catalog is data, not code** (`app/src-tauri/src/catalog.rs`). Adding a ported tool = a `Tool` entry with an `InstallMethod` + `LaunchMethod`; the porter (`porter.rs`) interprets recipes and knows nothing tool-specific. Matches `knowledge/15`/`17` ("more ported OSS tools = data entries").
- **Command construction is pure** (on the method enums), so *what command runs* is unit-testable without touching the system; `porter.rs` does only the I/O.
- **macOS reality handled:** Finder-launched apps get a minimal PATH, so `brew` is located explicitly (`/opt/homebrew`, `/usr/local`, then PATH) rather than trusted on PATH.
- **Device detected, not asked** (`device.rs`): the porter is on the machine, so it infers the tier (Apple Silicon vs Intel) and removes the website's device-picker screen — one less step (`skills/design.md`).

**UX (`skills/design.md`):** verb-first headlines from the decider; the **answer is the tool we actually install (Buzz)**, shown large, with the engine (Whisper) as supporting context — caught and fixed a first pass that showed "Whisper" (the model) as the thing being set up, which contradicted what the porter installs. Honest progress feedback during the (minute-long) download; a clear success state.

**Browser-previewable:** the Tauri calls degrade to a simulated port when served by plain Vite, so the UI runs and screenshots in a browser without the Rust backend compiled (`app/src/bridge.ts`).

**Tests:** Rust unit tests ship with the code — `catalog.rs` (lookup + command construction), `porter.rs` (output truncation, human error copy), `device.rs` (tier is always valid). Frontend verified end-to-end via the screenshot loop (`app/scripts/shots.mjs`): home → recommendation → working → done.

**Status:** ✅ **Verified in CI.** The founder's Mac lacked Xcode CLT + disk, so the build moved to GitHub Actions: a macOS runner compiled the Tauri backend, passed `cargo test` (9 porter tests), and ran the **real `brew install --cask buzz`** — all green (run `27621986664`). The Linux job (decider 64 tests, website 9 + build) is green too. Only the GUI launch (`open -a Buzz`) is uncovered (needs a desktop session). The local desktop run remains a nice-to-have once CLT + disk allow (`cd app && npm run tauri dev`).

## Shared decider (`packages/decider/`)

### 2026-06-16 — Extracted the matching brain into `@verdure/decider`

**What:** The decision logic + taxonomy + cards moved from `website/lib` into a workspace package (`packages/decider/`): `tiers`, `modality`, `matrix` (the pure `decide()`), `intent` (the classifier), `recommend` (presentation). The website and the new desktop porter both import `@verdure/decider`; npm workspaces symlink it. `session.ts`/`sheets.ts` (browser/Google-specific) stayed in `website/lib`.

**Problem it solves:** `CLAUDE.md` requires the matrix be the single source of truth across surfaces. With two surfaces (website + porter), copy-paste would drift. One package, imported by both, makes drift impossible (`[OQ-49]` — chose shared package over copy).

**Key decisions:** the package ships **TypeScript source** (no build step) consumed directly by Next (`transpilePackages`), Vite, and Vitest. A barrel (`src/index.ts`) re-exports everything; removed a duplicate `Feasibility` re-export so `export *` stays unambiguous. Root `package.json` declares the npm workspaces (`packages/*`, `website`, `app`).

**Tests:** the decider's own suite (64 tests — matrix/modality/recommend/intent) moved with it and passes from the package. The website's remaining suite (9 sheets tests) and a full production build both pass against the imported package, proving the refactor preserved behavior.

## Website (Phase 0)

### 2026-05-29 — Multimodal recommendations (audio · image · video)

**What:** The recommendation engine went from text-LLM-only to multimodal. The classifier (`website/lib/intent.ts`) now resolves a request to a `(modality, task)` pair across text/audio/image/video (`website/lib/modality.ts`); the engine (`website/lib/recommend.ts`) dispatches by modality and returns one of three honest outcomes — `recommended`, `works-but-slow`, or `not-local-yet`.

**Problem it solves:** Typing "transcribe my recording" or "make a video" used to silently return a generic chat model. Now audio/image/video map to real local tools (Whisper, Kokoro, Real-ESRGAN, Stable Diffusion, LTX-2…) where the device can run them — and to an honest "your device can't do this locally yet" where it can't (e.g. video on a laptop), instead of a fake recommendation.

**Key design decisions:**
- **Feasibility is graphics-bound, not RAM-bound** for non-text work, so the five device tiers collapse into three capability classes (cpu-laptop / apple-silicon / gpu).
- **Copy is verb-first and jargon-free** per the "Assume zero technical knowledge" principle in `skills/design.md`: the headline says what the user will *do*; the model name is demoted to an ignorable "the tool we'd use" detail with a learn-more link.
- **Install paths are modality-specific data** returned by the engine (Whisper apps for audio, Draw Things/Fooocus for image, etc.) — Ollama/LM Studio only ever apply to text. The UI renders them generically.

**Knowledge base:** grounded in research captured in `knowledge/12-multimodal-models.md` (the source of truth); `03` scoped to text; thesis, licensing, runtime-backends cross-referenced; OQ-28..31 opened.

**Tests:** `website/lib/intent.test.ts` and `website/lib/recommend.test.ts` rewritten for the multimodal shape (40 tests total, all passing). Text model picks are unchanged and still asserted.

**Not yet done:** visual self-review (screenshot loop) of the new recommendation page — no browser tooling installed locally. Device-picker still uses RAM/Apple-Silicon jargon (single-card redesign queued).

### 2026-05-29 — Waitlist persistence via Google Sheets

**What:** `/api/waitlist` now appends each signup as a row in a Google Sheet instead of logging to the console. New deep module `website/lib/sheets.ts` owns auth (service-account JWT), private-key parsing, column order, and the append call.

**Problem it solves:** Phase 1 launch-list capture was a `console.log` stub — signups weren't durable. Closes the persistence half of `[OQ-10]`.

**Behavior:**
- Configured (env vars set): appends a row; an append failure returns a retryable 502 so no real signup is silently lost.
- Not configured: logs the signup server-side and returns success, keeping the four-screen flow exercisable before the Google Cloud setup is done.

**Setup:** `website/SETUP_SHEETS.md` (Sheet + service account + env vars), template in `website/.env.example`. The user owns the Google Cloud setup.

**Tests:** `website/lib/sheets.test.ts` — 9 unit tests over the pure helpers (key normalization, config loading, column order). The networked append is not unit-tested; it needs live credentials (see `TEST_REPORT.md`).

## Orchestration binary (Phase 1)

_No features logged yet — Rust scaffold only (`src/`)._
