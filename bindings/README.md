# Bindings

The public API is written once, in Rust, in `crates/verdure-ffi/src/lib.rs`
(knowledge/26 §4.3). Everything here is **generated** from it by UniFFI and is
not committed: regenerate after any change to that file.

## Generate

Build the library, then run the bundled generator against it:

```sh
cargo build -p verdure-ffi --release            # add --features llama,whisper for the real engines
cargo run -p verdure-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library target/release/libverdure.dylib --language swift  --out-dir bindings/swift/generated
cargo run -p verdure-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library target/release/libverdure.dylib --language kotlin --out-dir bindings/kotlin/generated
cargo run -p verdure-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library target/release/libverdure.dylib --language python --out-dir bindings/python/generated
```

(`.so` on Linux/Android, `.dll` on Windows, and the iOS build goes through an
XCFramework of the `staticlib` — the packaging per platform is `knowledge/11`'s
job and is not written yet.)

**Windows (2026-11-05):** `build-sdk-windows.yml` builds `verdure.dll`, generates the Python and Kotlin bindings from it on Windows, and tests the zip there; `bindings/windows/README.md` is what a vendor reads, `bindings/tests/` holds the checks (`pe_check.py` inspects the library file, `sdk_smoke.py` is the customer path from Python on any desktop, `Smoke.kt` the same from a JVM).

**Verified 2026-10-26 on macOS:** the Python generation above runs and the module works end to end against `target/debug/libverdure.dylib` (copy the dylib next to `verdure.py`); see `docs/sdk-integration-guide.md` §5. Swift and Kotlin generation are the same command with `--language swift|kotlin` and are not yet exercised.

## The one-liners the generated code gives a vendor

| Platform | Install | Start |
|---|---|---|
| Swift | SPM package (planned: `verdure-swift`) | `let v = try Verdure.start(licenseKey: key, policy: PolicyConfig(), dataDir: dir)` |
| Kotlin | Maven artefact (planned: `ai.verdure:verdure`) | `val v = Verdure.start(key, PolicyConfig(), dir)` |
| Python | wheel (planned: `pip install verdure`) | `v = verdure.Verdure.start(key, PolicyConfig(), dir)` |

Then `v.recommend(Task.CHW_FORMS)`, `v.download(id, listener)`, `v.load(id)`,
`v.transcribe(config)`, `v.extract(text, schemaJson)`, `v.index(...)`,
`v.query(...)`, `v.delete(scope)`. Node/Electron, Flutter and React Native wrap
the C ABI or the Kotlin/Swift bindings; none exists yet (knowledge/26, next steps).
