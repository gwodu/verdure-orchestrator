# Verdure SDK — integration guide (v0.1, 2026-10-26)

This document is complete on its own. Follow it top to bottom and you will have a working program against the Verdure SDK without downloading any model. Every command in it has been run on the repository as it stands.

## 1. What the SDK is

Verdure is a runtime a company embeds in its own app. On the user's device, with no internet, it does three things:

1. **Speech to text.** Microphone audio in, transcript events out.
2. **Text to a filled form.** You give it free text and a JSON Schema you wrote; it returns JSON that matches the schema, or an error that says exactly which fields were wrong.
3. **Search over documents already on the device.** You index documents; you ask a question; you get the best passages back.

The company keeps its own app, screens and cloud. Verdure is the engine underneath. It never opens a network connection except to download model files from a server the company configured, and only when the company's policy allows that.

### Vocabulary

| Word | Meaning |
|---|---|
| **licence token** | A signed string the company's build receives. Verified offline. Without a valid one, nothing starts. |
| **manifest** | A signed JSON file listing **bundles**. The company writes it and hosts it. It is the only place models come from. |
| **bundle** | One set of model files for one **task** (`scribe`, `tutor`, `chw_forms`) on certain device **tiers**. |
| **tier** | How capable the device is. Phones: `P1` (4 GB), `P2` (6 GB), `P3` (8 GB). Laptops: `L1` (4 GB), `L2` (8 GB), `L3` (16 GB+). |
| **policy** | What the runtime may do: network downloads on or off, which hosts, encrypted storage, retention. Defaults are the strictest sensible settings. |
| **engine** | whisper.cpp (speech) and llama.cpp (language), compiled in only when the build asks for them (see section 7). |

## 2. Prerequisites

- The `verdure-orchestrator` repository checked out. All paths below are relative to its root.
- Rust (stable) and `cargo`. `cargo --version` must print something.
- For the Python route only: Python 3.

Check the SDK builds and its tests pass (about a minute the first time):

```sh
cargo test --workspace
```

Expect `test result: ok` lines and no `FAILED`.

## 3. The sandbox credentials you can use right now

The repository contains everything a test project needs:

| File | What it is |
|---|---|
| `bundles/sandbox.token` | A licence token for vendor `sandbox`, unlimited seats, valid for ten years. Signed by the development issuer key. |
| `crates/verdure-ffi/keys/issuer.pub` | The development issuer's public key. `Verdure.start()` uses it automatically. |
| `bundles/manifests/example.json` | A signed example manifest with seven bundles, all with real hashes except `asr-afrispeech-medium` (no download URL yet). For the scribe task: **`scribe-small`** (Whisper small + Qwen2.5 0.5B, 979 MB — the recommended bundle for clinical speech, listed for 8 GB phones and laptops), **`scribe-floor`** (Whisper base + the same 0.5B, 639 MB — the live-speech pick on phones, the only pick on smaller ones, and a fine choice for everyday dictation), `scribe-L2` (Whisper small + the 1.5B, laptops, extraction unmeasured). `chw-forms-P1` for forms. The speech-only `asr-*` bundles exist for the bake-off. Every bundle carries the speeds we measured per tier (`speed`), which `recommend` reads (section 6). The two scribe bundles share the 491 MB language model, stored once. Which to pick: section 7b. Sizes everywhere in this guide are decimal MB (1 MB = 1,000,000 bytes), the unit `download_bytes` counts in. |
| `bundles/manifests/example.json.sig` | Its signature, made with the same development key, so the public key above verifies it. |
| `bundles/manifests/schema-example-chw.json` | A JSON Schema for a community-health-worker antenatal visit form. |

These are development credentials. A real company receives its own token and signs its own manifest.

## 4. Route A — a Rust program (recommended for a first project)

Create your project as a folder under `sandbox/`. That folder is excluded from the main workspace on purpose, so your experiment cannot break the SDK's own build. The quickest start is to copy the template:

```sh
cp -r sandbox/template sandbox/my-project
cd sandbox/my-project
cargo run
```

Expected output (numbers differ per machine):

```
tier L2 · 2079 MB of RAM free now · 1686 MB disk free · engines built: []
note: Intel Mac: inference runs on the CPU (the GPU path returns wrong numbers here).
```

`engines built: []` is normal for this build. Section 7 explains.

Your `Cargo.toml` depends on the SDK by path:

```toml
[dependencies]
verdure-ffi = { path = "../../crates/verdure-ffi" }
serde_json = "1"
```

The crate is imported as `verdure` in code (`use verdure::...`).

### The whole flow, in order

The file `crates/verdure-ffi/examples/chw.rs` is a complete, runnable program doing all of the following. Run it from the repo root with:

```sh
cargo run -p verdure-ffi --example chw -- "$(cat bundles/sandbox.token)"
```

Step by step:

**Start.** Verifies the licence, reads the device. No network, instant.

```rust
use verdure::{PolicyConfig, Verdure};
let sdk = Verdure::start(token, PolicyConfig::default(), data_dir)?;
```

`data_dir` is a folder path (a `String`) where Verdure may store downloaded bundles and its index. Use a folder your app owns.

**Read the device.**

```rust
let caps = sdk.capabilities();
println!("{} {} MB free", caps.tier, caps.available_ram_mb);
```

**Trust a manifest.** Either from files you ship (fully offline) or from a URL (section 6).

```rust
sdk.load_manifest(manifest_json, signature, public_key)?;
```

All three are `String`s read from the files in section 3. If the signature does not match, this returns `VerdureError::Integrity` and the manifest is not used.

**Pick a bundle.** Two ways. Let Verdure choose for a task:

```rust
match sdk.recommend(Task::ChwForms) {
    Ok(bundle) => println!("use {}", bundle.id),
    Err(e) => println!("nothing fits: {e}"),
}
```

Or judge one you chose yourself. Nothing is substituted; you get the truth about that bundle:

```rust
let report = sdk.fit("chw-forms-P1".into())?;
if !report.fits { for p in report.problems { println!("{p}"); } }
```

On an 8 GB laptop with little free memory, both of these will honestly say the example bundles do not fit. That is correct behaviour, not a failure of your code.

**Index and search documents.** Works without any model.

```rust
use verdure::Doc;
let n = sdk.index("guides".into(), vec![Doc { id: "anc".into(), title: "Antenatal care".into(), text: "...".into() }])?;
let hits = sdk.query("guides".into(), "when do I refer for blood pressure?".into(), 3)?;
println!("{}", hits[0].text);
```

**Extract a form from text.**

```rust
let out = sdk.extract(text, schema_json)?;   // out.json is valid against your schema
```

With no engine built this returns `VerdureError::EngineUnavailable("no language model is loaded: call load(bundle) first")`. To exercise the full path anyway, give Verdure a cloud hook. In a real app this calls the company's own API; in a test it can return canned JSON:

```rust
use std::sync::Arc;
use verdure::{CloudFallback, VerdureError};
struct MyCloud;
impl CloudFallback for MyCloud {
    fn transcribe(&self, _pcm: Vec<f32>) -> Result<String, VerdureError> { Ok(String::new()) }
    fn extract(&self, _text: String, _schema: String) -> Result<String, VerdureError> {
        Ok(r#"{"client_name":"Amina","pregnant":true,"danger_signs":[],"referred":false}"#.into())
    }
}
sdk.set_cloud_fallback(Some(Arc::new(MyCloud)));
let out = sdk.extract(text, schema_json)?;
```

Verdure still validates the hook's output against your schema. Bad JSON from the cloud is `VerdureError::SchemaInvalid`, the same as from a local model.

**Clean up.**

```rust
use verdure::DeleteScope;
sdk.delete(DeleteScope::All)?;   // destroys the encryption key first, then the files
```

## 5. Route B — Python

Build the library and generate the Python module (both verified):

```sh
cargo build -p verdure-ffi
cargo run -p verdure-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library target/debug/libverdure.dylib --language python --out-dir sandbox/py
cp target/debug/libverdure.dylib sandbox/py/
```

(On Linux the library is `target/debug/libverdure.so`.) A warning about `yapf` not being found is harmless. Then:

```python
import sys; sys.path.insert(0, "sandbox/py")
import verdure as v

token = open("bundles/sandbox.token").read().strip()
policy = v.PolicyConfig(network_downloads=True, allowed_hosts=[], local_counters=False,
                        encrypted_storage=True, retention_days=None)
sdk = v.Verdure.start(token, policy, "/tmp/verdure-sandbox")
caps = sdk.capabilities()
print(caps.tier, caps.available_ram_mb)

sdk.load_manifest(open("bundles/manifests/example.json").read(),
                  open("bundles/manifests/example.json.sig").read(),
                  open("crates/verdure-ffi/keys/issuer.pub").read().strip())
print(sdk.fit("chw-forms-P1").problems)

sdk.index("guides", [v.Doc(id="anc", title="Antenatal care", text="Refer if BP above 140/90.")])
print(sdk.query("guides", "when do I refer?", 3)[0].text)

class MyCloud(v.CloudFallback):
    def transcribe(self, pcm): return ""
    def extract(self, text, schema_json):
        return '{"client_name":"Amina","pregnant":true,"danger_signs":[],"referred":false}'
sdk.set_cloud_fallback(MyCloud())
out = sdk.extract("Client Amina, not pregnant.", open("bundles/manifests/schema-example-chw.json").read())
print(out.json, out.warnings)
sdk.delete(v.DeleteScope.ALL())
```

Python differences from Rust: enum values are upper case (`v.Task.SCRIBE`, `v.DeleteScope.ALL()` — note the parentheses, it is a variant with no fields), `PolicyConfig` has no defaults so every field is passed, and errors are `v.VerdureError.NoBundle` and friends, catchable with `except v.VerdureError.NoBundle as e`.

## 5b. Route C — Android (Kotlin)

The CI workflow `build-sdk-android.yml` produces `verdure-android-arm64.zip`: the arm64 library with both engines, the generated Kotlin binding, the sandbox credentials and the example manifest. `bindings/android/README.md` (also inside the zip) says exactly which files go where in an Android project, the two Gradle dependencies (JNA, coroutines), and shows the start → recommend → download → load → transcribe → extract sequence in Kotlin. Same names as Rust and Python, Kotlin casing: `sdk.recommend(Task.SCRIBE)`, `stream.pushPcm(floats, 16000u)`, errors as `VerdureException.*`. Every call blocks; keep them off the main thread.

## 6. Full API reference

Every method on `Verdure`. All calls block; call them off your UI thread.

| Method | Does | Fails with |
|---|---|---|
| `Verdure.start(token, policy, data_dir)` | Verify licence, read device | `License` |
| `Verdure.start_with_issuer(token, policy, data_dir, public_key)` | Same, with a different issuer key | `License` |
| `capabilities()` | Tier, memory, disk, cores, GPU class, engines built, attributions, notes | — |
| `licence_summary()` | "vendor · seats · expiry" | — |
| `set_bundle_source(manifest_url, public_key)` | Fetch `manifest_url` and `manifest_url + ".sig"`, verify, use | `Policy`, `Network`, `Integrity` |
| `load_manifest(json, signature, public_key)` | Use a manifest you ship in the app | `Integrity`, `InvalidInput` |
| `recommend(task)` | Best bundle that fits this device now, within the licence, and keeps pace with live speech where we measured it (the same as `recommend_for(task, Pace::Live)`); `skipped` explains every bundle passed over, speed included; `speed_x_real_time` is the measurement for the chosen one, or none | `NoBundle` |
| `recommend_for(task, pace)` | Same with the pace stated. `Pace::Recorded` — the speech is already captured and the transcript may come later — allows a bundle that is slower than real time on this device | `NoBundle` |
| `fit(bundle_id)` | Judge one bundle for live speech: `fits` plus every `problem` (the same as `fit_for(bundle_id, Pace::Live)`) | `NoBundle` |
| `fit_for(bundle_id, pace)` | Same for the pace stated | `NoBundle` |
| `download(bundle_id, listener)` | Download or resume every file, verified, reusing any model another bundle on this device already holds; bytes via `on_progress`, one sentence per file via `on_status` | `License`, `Policy`, `Network`, `Integrity`, `Cancelled` |
| `is_downloaded(bundle_id)` | All files present and hashes match | — |
| `load(bundle_id)` | Load engines; refuses with numbers if memory is short. Reads the models from storage: 1.2 s for `scribe-floor`, 15 s for `scribe-small` on a Pixel 8a, and nothing runs until it returns | `Memory`, `EngineUnavailable`, `Engine` |
| `load_overcommit(bundle_id)` | Same without the memory gate; may crash the host process | as above |
| `transcribe(config)` | Open an `AsrStream`: `push_pcm(samples, rate)` → events, `finish()`. `config.lang_hints = ["en"]`; leave `keyterms` empty — measured to make recognition worse (`knowledge/26`, "The speech bake-off"); use `replacements` to fix spellings | `EngineUnavailable` |
| `chat(messages, sink)` | One conversation turn, streamed to `sink` | `EngineUnavailable`, `Engine` |
| `extract(text, schema_json)` | Schema-valid JSON, or the cloud hook if set and no engine. Blocks for the whole generation: tens of seconds on a phone. See "What `extract` returns" below | `EngineUnavailable`, `InvalidSchema`, `SchemaInvalid` |
| `index(collection, docs)` | Chunk and index; returns chunk count | `Storage` |
| `query(collection, question, k)` | Best `k` passages | `Storage` |
| `delete(scope)` | `Collection{name}`, `Indexes`, or `All` | `Storage` |
| `set_cloud_fallback(hook or none)` | Give or remove the company's cloud | — |

Callback traits you implement: `ProgressListener` (`on_progress(done, total)` — bytes across the whole bundle during a download; `on_status(message)` — one sentence per file as the download goes ("Downloading Speech model (Whisper small) (488 MB)…", "Speech model (Whisper small) downloaded and verified.", "Checking Language model (Qwen2.5 0.5B)…", "Language model (Qwen2.5 0.5B): reusing the copy another bundle downloaded (491 MB not downloaded again).", "… is already on this device.") and when something changes course, such as a downgrade or a throttle; `is_cancelled()`), `TokenSink` (`on_token(piece) -> bool`, return false to stop), `CloudFallback` (`transcribe`, `extract`).

**What `extract` returns.** `Extraction { json, attempts, used_grammar, warnings }`. `json` is compact and valid against your schema. `attempts` is how many generations it took: 1 means the first reply validated; 2 means the model was shown its violations once and the repaired reply validated. If the second reply still fails, `extract` throws `SchemaInvalid` whose message lists every violation by path — that is the "did not match the schema" error, and it comes *after* the model has run, so it costs the same time as a success. The message is written for your log, not your screen: it says what is wrong where (`$.facts[0]: missing required property "group"; $.facts[0].text: still holds the placeholder "One atomic clinical statement…"`) and nothing else; the instruction the model is given on the repair turn is not in it. Show the user a sentence of your own. `used_grammar` is `false` in this release: grammar-constrained decoding is built but switched off, because the app that this SDK grew out of saw llama.cpp's grammar sampler abort the whole process on a multi-byte token (knowledge/26, `[OQ-108]`); it returns once the engine runs isolated. `warnings` lists schema keywords the grammar could not have expressed anyway (`minimum`, `minLength`, …); they are enforced by validation.

**Threads and blocking.** Every call blocks for as long as the work takes: `push_pcm` for as long as the engine needs when a 12 s segment completes — a few seconds with `scribe-floor`, up to two minutes with `scribe-small` on a Pixel 8a, where it runs at 3.2× real time (section 7b); `extract` for tens of seconds on a phone (16–114 s measured with the 0.5B); `download` for the whole download; `load` for 1–15 s. `Verdure` and each `AsrStream` are separately locked, so a stream's `push_pcm` on one thread and `extract` on another run concurrently — keep transcription on its own thread and extraction on another, and the transcript will not stall. Two `extract` calls queue; they never run at once. Nothing may run on the main thread.

**Transcript events.** The whisper engine emits `Final` only, one per utterance, cut at pauses or at 12 seconds; `Partial` is reserved for engines that stream. A fixture with no pauses is cut every 12 seconds, mid-sentence; real speech pauses.

**Speed, and how `recommend` uses it.** Each bundle in the manifest may carry `speed`: the speech recogniser's measured processing time per second of speech on a tier, fed flat out and sustained (a throttled phone), as × real time — `"speed": {"P3": 3.2, "L2": 0.75}`. For `Pace::Live`, a bundle measured above **1.5× real time** on the device's tier is passed over with the number in `skipped` (or `problems`, for a pin): on the Pixel 8a, Whisper base at 1.25× kept pace through a live visit because speech has pauses, while Whisper small at 3.2× finished a ten-minute visit twenty minutes late. A tier with no measurement is not held against the bundle — the SDK does not guess — and `speed_x_real_time` comes back as none. The 1.5 threshold is ours and provisional, from one phone (`knowledge/26`, "Production pass"); the numbers in the shipped manifest are in section 7b.

**Where files go.** `<data_dir>/bundles/<bundle id>/<file>`, one directory per bundle, with each file a hard link to `<data_dir>/models/<sha256>` — so a model two bundles share (`scribe-floor` and `scribe-small` both use the 491 MB language model) is downloaded once and stored once, and a device that already holds two copies folds them into one on its next `download` of either bundle. `is_downloaded` reads the bundle directory; `delete(All)` removes both.

Schema support for `extract`: objects with `properties`/`required`/`additionalProperties: false`; `string` (with `enum`, `const`, `minLength`, `maxLength`); `number`; `integer` (with `minimum`, `maximum`); `boolean`; `null`; arrays with `items`, `minItems`, `maxItems`; `anyOf`/`oneOf`; `$ref` to `#/$defs/...`; nullable types as `["string", "null"]`. Recursive schemas are refused.

## 7. What works today and what does not — read before judging results

**Works with no model on disk:** everything in sections 4 and 5: licence, device, manifest, `recommend`, `fit`, `download` (against a real server), indexing and search, `extract` through the cloud hook, `delete`.

**Needs an engine build:** `load`, `transcribe`, `chat`, and `extract` without a cloud hook. The engines compile in only with `CARGO_BUILD_JOBS=2 cargo build -p verdure-ffi --features llama,whisper`, which builds llama.cpp and whisper.cpp (about six minutes and a few GB of disk; verified on an 8 GB Intel Mac). After that build, regenerate the Python binding (section 5) from the new library. `evals/README.md` shows a full run on real speech. Your project should **not** assume engines are present: check `capabilities().backends` and handle `EngineUnavailable` by telling the user plainly or by using the cloud hook.

**Do not do these:**

- `scribe-L2`'s 1.5B hash comes from Hugging Face's file metadata and has not been downloaded on our side yet; if its verification fails, tell us rather than working around it.
- Do not hand-write a licence token or a signature. Use `bundles/sandbox.token` or mint one: `cargo run -p verdure-cli -- license issue --key ~/.verdure/issuer.key --vendor you`.
- Do not catch errors with a blanket handler and print "failed". Every `VerdureError` carries a sentence meant for the screen; show it.
- Do not add a workspace member or edit anything under `crates/`. Your project lives under `sandbox/`.

## 7b. Which speech model to pick — our recommendation for clinical speech

The SDK does not choose a speech model for you by quality; it chooses the bundle that fits the device from the menu the manifest offers (section 6, `recommend` and `fit`). What goes on that menu is a quality decision, and we have measured it, so here is our recommendation.

**For medical or clinical speech, use Whisper small, not Whisper base.** We ran both through the SDK's own `transcribe()` on ten real two-speaker GP consultations (PriMock57, 14,700 reference words) and scored them on word error rate and on a list of a hundred clinical terms — drug names, symptoms, conditions — because the clinical words are what a note is made of.

| speech model | download | word error rate | clinical terms missed | speed on an 8 GB Intel Mac | speed on a Pixel 8a (8 GB phone) |
|---|---|---|---|---|---|
| Whisper base (`scribe-floor`) | 148 MB | 26.5 % | 1 in 3.6 | 0.28× real time | 1.25× real time (0.7–1.8) |
| **Whisper small** (`scribe-small`) | 488 MB | **21.6 %** | **1 in 6** | 0.75× real time | **3.2× real time** (2.7–4.0) |

Base turned *diarrhoea* into "die rule" and *eczema* into "exam"; small gets those right and still misses some (*stools* as "stall", *paracetamol* as "parasitibol"). Small is 3.3× the download and about 2.7× the compute. On a laptop that is comfortably live. On a Pixel 8a it is not: the same ten consultations run on the phone through the same API (2026-11-04) gave the same quality as the Mac — 21.1 % word error, 1 in 5.8 clinical terms missed — at 3.2× real time. A ten-minute visit finishes transcribing about twenty minutes after it ends, and a single 12 s window can block `push_pcm` for a minute or two. Base on the same phone ran at 0.7–1.8×. Nothing is lost — the SDK queues the audio — but the transcript is not live (`knowledge/26`, "Small on the Pixel").

**How to act on it.** Nothing, if you call `recommend(Task.SCRIBE)`: it knows these speeds (they are in the manifest, section 6). On a laptop it returns `scribe-small`. On an 8 GB phone it returns `scribe-floor` for live speech and tells you why in `skipped` — "scribe-small: runs at 3.2× real time on P3, too slow to keep pace with live speech". When the audio is already captured and the transcript may come later, call `recommend_for(Task.SCRIBE, Pace.Recorded)` and the same phone gets `scribe-small`: the better transcript, after the visit. On a 4–6 GB phone it is `scribe-floor` either way. If you want small everywhere, or base everywhere, pin it: `fit("scribe-small")`, or `fit_for("scribe-small", Pace.Recorded)`, tells you whether it fits that device and at what speed, then `load` it. Base is a reasonable choice for everyday dictation; small is the one for clinical vocabulary. The device still has the last word: a pin that does not fit the memory is refused with the numbers, and `load_overcommit` exists for machines you have measured yourself.

**Two things not to do.** Do not fill `AsrConfig.keyterms` with a vocabulary list — a 40-word clinical list doubled the error rate in the same measurement; use `replacements` for spellings you need fixed. Do not assume a model fine-tuned "for medical speech" is better: the one we tried (a clinical Whisper medium fine-tuned on African-accented read speech) scored 83 % word error on these consultations.

**What the recommendation does not cover yet:** Whisper medium and large-v3-turbo (bigger, likely better, not measured here), phones other than the Pixel 8a, and languages other than English. The licence is the same for every Whisper size: MIT.

## 8. The developer tool

`cargo run -p verdure-cli -- <command>`:

```
verdure capabilities                               what this machine is, as JSON
verdure license keygen [--out FILE]                new issuer key (secret to FILE, public printed)
verdure license issue --key FILE --vendor ID [--seats N] [--days N] [--bundle ID]... [--ack NAME]...
verdure license verify --public KEY TOKEN
verdure manifest check FILE                        list bundles with memory, download sizes and measured speeds
verdure manifest sign --key FILE FILE              writes FILE.sig
verdure manifest verify --public KEY FILE
verdure recommend --manifest FILE --task scribe|tutor|chw_forms [--pace live|recorded] [--tier L2] [--ram MB] [--disk MB]
verdure grammar SCHEMA.json                        the constrained-sampling grammar for a schema
```

## 9. A test assignment

Build a command-line program under `sandbox/visit-form/` that:

1. Starts the SDK with the sandbox licence and prints the device tier and free memory.
2. Loads the example manifest from the files in section 3 and prints, for `chw-forms-P1`, whether it fits and why not if it doesn't.
3. Reads a text file given on the command line containing a dictated home visit, for example: "Visited Amina today, she is about twenty eight weeks pregnant, blood pressure one fifty over ninety five, complains of a bad headache, I referred her to the clinic."
4. Calls `extract` with `bundles/manifests/schema-example-chw.json`. If no engine is available, uses a `CloudFallback` whose `extract` returns JSON you construct from the text (any reasonable approach), and prints the final validated JSON.
5. Indexes `bundles/manifests/schema-example-chw.json`'s own `description` strings as a tiny "guideline" collection, then answers "what counts as a danger sign?" with the top passage.
6. Exits 0 on success and prints every `VerdureError` as its message, never as a debug dump.

Acceptance: `cargo run -- visit.txt` from inside `sandbox/visit-form/` prints the tier, the fit verdict with its sentences, valid JSON matching the schema, and one guideline passage.
