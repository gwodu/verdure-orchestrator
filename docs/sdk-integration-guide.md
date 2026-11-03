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
| `bundles/manifests/example.json` | An example manifest with three bundles. `scribe-floor` and `chw-forms-P1` carry real, measured hashes (Whisper base + Qwen2.5 0.5B, 609 MB) and download fine. `scribe-L2` has a placeholder hash and must not be downloaded. |
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
| `recommend(task)` | Best bundle that fits this device now, within the licence; explains what it skipped | `NoBundle` |
| `fit(bundle_id)` | Judge one bundle; `fits` plus every `problem` | `NoBundle` |
| `download(bundle_id, listener)` | Download or resume every file, verified; progress via `listener` | `License`, `Policy`, `Network`, `Integrity`, `Cancelled` |
| `is_downloaded(bundle_id)` | All files present and hashes match | — |
| `load(bundle_id)` | Load engines; refuses with numbers if memory is short | `Memory`, `EngineUnavailable`, `Engine` |
| `load_overcommit(bundle_id)` | Same without the memory gate; may crash the host process | as above |
| `transcribe(config)` | Open an `AsrStream`: `push_pcm(samples, rate)` → events, `finish()` | `EngineUnavailable` |
| `chat(messages, sink)` | One conversation turn, streamed to `sink` | `EngineUnavailable`, `Engine` |
| `extract(text, schema_json)` | Schema-valid JSON, or the cloud hook if set and no engine. Blocks for the whole generation: tens of seconds on a phone. See "What `extract` returns" below | `EngineUnavailable`, `InvalidSchema`, `SchemaInvalid` |
| `index(collection, docs)` | Chunk and index; returns chunk count | `Storage` |
| `query(collection, question, k)` | Best `k` passages | `Storage` |
| `delete(scope)` | `Collection{name}`, `Indexes`, or `All` | `Storage` |
| `set_cloud_fallback(hook or none)` | Give or remove the company's cloud | — |

Callback traits you implement: `ProgressListener` (`on_progress(done, total)` — bytes during a download; `on_status(message)` — a sentence when something changed course, such as a downgrade or a throttle, so it is normal for it never to fire during a clean download; `is_cancelled()`), `TokenSink` (`on_token(piece) -> bool`, return false to stop), `CloudFallback` (`transcribe`, `extract`).

**What `extract` returns.** `Extraction { json, attempts, used_grammar, warnings }`. `json` is compact and valid against your schema. `attempts` is how many generations it took: 1 means the first reply validated; 2 means the model was shown its violations once and the repaired reply validated. If the second reply still fails, `extract` throws `SchemaInvalid` whose message lists every violation by path — that is the "did not match the schema" error, and it comes *after* the model has run, so it costs the same time as a success. `used_grammar` is `false` in this release: grammar-constrained decoding is built but switched off, because the app that this SDK grew out of saw llama.cpp's grammar sampler abort the whole process on a multi-byte token (knowledge/26, `[OQ-108]`); it returns once the engine runs isolated. `warnings` lists schema keywords the grammar could not have expressed anyway (`minimum`, `minLength`, …); they are enforced by validation.

**Threads and blocking.** Every call blocks for as long as the work takes: `push_pcm` for up to a few seconds when a segment completes, `extract` for tens of seconds on a phone, `download` for the whole download. `Verdure` and each `AsrStream` are separately locked, so a stream's `push_pcm` on one thread and `extract` on another run concurrently — keep transcription on its own thread and extraction on another, and the transcript will not stall. Two `extract` calls queue; they never run at once. Nothing may run on the main thread.

**Transcript events.** The whisper engine emits `Final` only, one per utterance, cut at pauses or at 12 seconds; `Partial` is reserved for engines that stream. A fixture with no pauses is cut every 12 seconds, mid-sentence; real speech pauses.

Schema support for `extract`: objects with `properties`/`required`/`additionalProperties: false`; `string` (with `enum`, `const`, `minLength`, `maxLength`); `number`; `integer` (with `minimum`, `maximum`); `boolean`; `null`; arrays with `items`, `minItems`, `maxItems`; `anyOf`/`oneOf`; `$ref` to `#/$defs/...`; nullable types as `["string", "null"]`. Recursive schemas are refused.

## 7. What works today and what does not — read before judging results

**Works with no model on disk:** everything in sections 4 and 5: licence, device, manifest, `recommend`, `fit`, `download` (against a real server), indexing and search, `extract` through the cloud hook, `delete`.

**Needs an engine build:** `load`, `transcribe`, `chat`, and `extract` without a cloud hook. The engines compile in only with `CARGO_BUILD_JOBS=2 cargo build -p verdure-ffi --features llama,whisper`, which builds llama.cpp and whisper.cpp (about six minutes and a few GB of disk; verified on an 8 GB Intel Mac). After that build, regenerate the Python binding (section 5) from the new library. `evals/README.md` shows a full run on real speech. Your project should **not** assume engines are present: check `capabilities().backends` and handle `EngineUnavailable` by telling the user plainly or by using the cloud hook.

**Do not do these:**

- Do not try to download `scribe-L2`. Its hash is a placeholder and verification will fail by design. `scribe-floor` and `chw-forms-P1` are real.
- Do not hand-write a licence token or a signature. Use `bundles/sandbox.token` or mint one: `cargo run -p verdure-cli -- license issue --key ~/.verdure/issuer.key --vendor you`.
- Do not catch errors with a blanket handler and print "failed". Every `VerdureError` carries a sentence meant for the screen; show it.
- Do not add a workspace member or edit anything under `crates/`. Your project lives under `sandbox/`.

## 8. The developer tool

`cargo run -p verdure-cli -- <command>`:

```
verdure capabilities                               what this machine is, as JSON
verdure license keygen [--out FILE]                new issuer key (secret to FILE, public printed)
verdure license issue --key FILE --vendor ID [--seats N] [--days N] [--bundle ID]... [--ack NAME]...
verdure license verify --public KEY TOKEN
verdure manifest check FILE                        list bundles with memory and download sizes
verdure manifest sign --key FILE FILE              writes FILE.sig
verdure manifest verify --public KEY FILE
verdure recommend --manifest FILE --task scribe|tutor|chw_forms [--tier L2] [--ram MB] [--disk MB]
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
