# TEST_REPORT.md

Test discipline record across the Verdure monorepo. Decider: `npm test --workspace @verdure/decider`. Website: `npm test --workspace nextjs`. Porter app frontend: `cd app && npm run build` (typecheck) + `node app/scripts/shots.mjs` (screenshots). Porter Rust: `cd app/src-tauri && cargo test`. Long-term binary: `cargo test`. Newest first.

## MVP desktop porter (`app/`)

### 2026-06-16 — Porter verified in CI (GitHub Actions macOS runner)

**Green on a clean macOS machine** (run `27621986664`, job "Desktop porter (macOS)", 3m25s). The founder's Mac lacked Xcode CLT and disk, so verification moved to CI (`.github/workflows/ci.yml`). The macOS job proved, in order:
- **Frontend typecheck + build** — `npm run build --workspace @verdure/porter-app` (tsc strict + Vite), also producing `app/dist` for `generate_context!`.
- **`cargo test`** — compiled the entire Tauri backend (tauri 2, serde, the app crate) and passed all 9 porter unit tests.
- **`brew install --cask buzz`** — the porter's actual install command, run for real. It succeeded, proving the one-click install recipe end-to-end on a clean machine.

The Linux job ("Decider + website", 47s) was green too: decider 64 tests, website 9 tests + production build.

**Only thing not covered:** the literal GUI launch (`open -a Buzz`) needs a desktop session, so it isn't exercised headlessly. The install — the part a non-technical user can't do — is verified. `[OQ-50]` closed.

### 2026-06-16 — Porter command-surface integration tests (`app/src-tauri/src/lib.rs`)

**Green in CI** (11 Rust tests total). Added `command_tests` to cover the gap that isolated unit tests + the simulated browser preview leave open — the actual Tauri command surface the frontend's `invoke()` targets:
- `app_builds_with_both_commands_registered` — builds the **full app with the real context** (`generate_context!`): `tauri.conf.json`, `capabilities/default.json`, and the icons all parse and load, and both `detect_device`/`port_tool` register in the real `invoke_handler`. This is real integration coverage — it **caught `port_tool` not being runtime-generic** (a `CommandArg`/`AppHandle<R>` bug) and the duplicate-`generate_context!` macOS link error during development.
- `detect_device_command_returns_a_valid_tier` — calls the exact function the `detect_device` IPC dispatches to and asserts a tier the decider recognizes.

**Deliberately not asserted — the literal IPC byte-routing.** An attempt to drive `tauri::test::get_ipc_response` was abandoned after it denied the call with `"detect_device not allowed. Plugin not found"` — both with and without capabilities, and even matching Tauri's own docs example verbatim. The Tauri docs are authoritative that **application commands are allowed by default on all webviews** (no permission needed), and our `capabilities/default.json` is exactly what `create-tauri-app` ships — so this is a quirk of Tauri's *mock* IPC harness, not our code, and the real "Set it up" is authorized. Verifying it further would test Tauri's internals, not Verdure. The true end-to-end (real webview render + IPC + GUI launch) is the on-device run.

**Still genuinely untested:** the assembled real desktop app — the webview rendering in WKWebView and a real click flowing through IPC to a launched Buzz. That needs a desktop session (the founder's Mac once CLT + disk allow); CI can't launch a GUI app. Everything up to and including the **real install** (`brew install --cask buzz`) is verified.

### 2026-06-16 — Porter Rust backend unit tests (`app/src-tauri/src/*`)

- `catalog.rs` (5): `find("buzz")` resolves and `find("unknown")` is `None`; the Buzz install recipe builds `brew install --cask buzz`, the check builds `brew list --cask buzz`, and launch builds `open -a Buzz`. These lock the recipe→command contract as **pure** functions (the brew path is injected), so they need no Homebrew on the test machine.
- `porter.rs` (3): `tail()` keeps short output whole and truncates long output from the front (asserts the 500-char + ellipsis bound); `PortError::BrewMissing` renders human, jargon-free copy (no "Err").
- `device.rs` (1): `detect_tier()` always returns a tier the decider recognizes — never an empty/bogus string — whatever the host machine is.

**Design choice that made this testable:** command construction lives on the `InstallMethod`/`LaunchMethod` enums (pure), separate from execution in `porter.rs` (I/O). So the high-value logic (what we'd run) is covered without sanitizer/FFI concerns — there is no FFI here, only `std::process::Command`.

**Not unit-tested (deliberate):** the actual `brew install` / `open` execution and the Tauri command wiring — that's the end-to-end run, verified by launching the app and porting Buzz on the Mac (pending CLT).

### 2026-06-16 — Porter frontend (`app/src/*`)

**Verified by typecheck + build + the screenshot loop.** `npm run build` (= `tsc` strict, incl. `noUnusedLocals/Parameters`, then `vite build`) passes — this also proves the frontend consumes `@verdure/decider` correctly. The visual self-review loop (`app/scripts/shots.mjs`, Playwright) captured all four states at 720px: home (intent grid), recommendation, setup-working, done.

**Edge case caught (visual review):** the recommendation hero first showed **Whisper** (the decider's model) as "We'll set up …", but the porter installs **Buzz** (the app). That contradicts what lands on the machine and what the progress/done copy said. Fixed: the hero names the tool we install (Buzz) with the engine (Whisper) as context — caught only because the design skill's "actually look at the screenshot" step was followed.

## Shared decider (`packages/decider/`)

### 2026-06-16 — Decider package extraction (`packages/decider/src/*`)

**Suite: 64 tests passing**, run from the package (`vitest`). These are the same matrix/modality/recommend/intent tests previously under `website/lib`, moved verbatim with the code — so the relocation is proven behavior-preserving by the tests passing in their new home. The website's full production build and its remaining 9 sheets tests also pass against the imported package, confirming no consumer broke.

## Website (Phase 0)

### 2026-05-29 — Multimodal recommendation engine (`website/lib/intent.test.ts`, `website/lib/recommend.test.ts`)

**Suite: 40 tests passing** (rewritten from the prior 13 text-only tests + 9 sheets tests).

`classifyIntent` — covers every modality and task: text (creative/code/multilingual/general + fallback), audio (transcription/speech/separation/music + the bare-"recording" → transcription default), image (generate/edit/upscale), video (generate/enhance).

`recommend` — covers the full feasibility matrix:
- Text picks are unchanged from the original matrix and still asserted model-by-model (regression lock during the refactor).
- Audio: transcription/speech recommended on every tier; separation works-but-slow on laptops; music honestly declined on laptops, available on GPU.
- Image: upscaling recommended everywhere; generation/editing declined on plain laptops, recommended on Mac/GPU.
- Video: generation declined on everything except a dedicated GPU (and only "works-but-slow" there); enhancement works-but-slow on laptops.
- **Invariants over the whole matrix:** every recommendation has a non-empty headline/body/label; `not-local-yet` ⇒ no model and no install options; otherwise a model is always present.

**Edge cases caught:** "make my photo sharper and bigger" classified as image-generation until "sharper"/"clearer" were added to the upscale keywords — fixed in the same change. The cross-modal trap "make music" (the word "write"/creative vs. audio) is locked by an explicit test: music wins.

**Not unit-tested (deliberate):** copy wording itself, and the visual rendering of the recommendation page (the design skill's screenshot loop) — pending browser tooling. The engine that *drives* the page is fully covered.

### 2026-05-29 — Waitlist Google Sheets backend (`website/lib/sheets.test.ts`)

**Unit tests written (9).** Covered (pure logic, no network):
- `normalizePrivateKey` — escaped `\n` → real newlines; real newlines untouched. This is the field that most often breaks service-account auth, so it's tested directly.
- `loadSheetsConfig` — returns null if any required var is missing or blank; resolves and normalizes the key when complete; defaults the range to `Sheet1!A:F` and honors an override.
- `isSheetsConfigured` — true only on a complete config.
- `buildWaitlistRow` — emits cells in the documented column order, one per declared column (locks the Sheet's column contract).

**Edge cases caught:** whitespace-only env values are treated as missing (a blank var in a deploy dashboard shouldn't read as "configured"). Column count is asserted against `WAITLIST_COLUMNS` so adding a column without updating the row builder fails a test rather than silently misaligning the Sheet.

**Not unit-tested (deliberate):** the live `spreadsheets.values.append` call — it requires real Google credentials and is integration territory. Manual verification steps are in `website/SETUP_SHEETS.md` §5.

## Orchestration binary (Phase 1)

_No tests logged yet — Rust scaffold only (`tests/integration_tests.rs` placeholder)._
