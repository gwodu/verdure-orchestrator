# TEST_REPORT.md — verdure_web

Test discipline record for the Phase 0 website. Run with `npm test`.

## 2026-05-29 — Multimodal recommendation engine (`lib/intent.test.ts`, `lib/recommend.test.ts`)

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

## 2026-05-29 — Waitlist Google Sheets backend (`lib/sheets.test.ts`)

**Unit tests written (9), full suite 22 passing.**

Covered (pure logic, no network):
- `normalizePrivateKey` — escaped `\n` → real newlines; real newlines untouched.
  This is the field that most often breaks service-account auth, so it's tested
  directly.
- `loadSheetsConfig` — returns null if any required var is missing or blank;
  resolves and normalizes the key when complete; defaults the range to
  `Sheet1!A:F` and honors an override.
- `isSheetsConfigured` — true only on a complete config.
- `buildWaitlistRow` — emits cells in the documented column order, one per
  declared column (locks the Sheet's column contract).

**Edge cases caught:** whitespace-only env values are treated as missing (a
blank var in a deploy dashboard shouldn't read as "configured"). Column count
is asserted against `WAITLIST_COLUMNS` so adding a column without updating the
row builder fails a test rather than silently misaligning the Sheet.

**Not unit-tested (deliberate):** the live `spreadsheets.values.append` call —
it requires real Google credentials and is integration territory. The module is
structured so all logic that *can* be tested without the network is pure and
covered; the append is a thin call over verified inputs. Manual verification
steps are in `SETUP_SHEETS.md` §5.
