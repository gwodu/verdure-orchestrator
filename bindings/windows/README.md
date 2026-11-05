# Verdure SDK for Windows — what is in this zip and how to use it

Produced by `.github/workflows/build-sdk-windows.yml`, on a Windows machine, and tested there before it
is zipped: the SDK's own test suite, then this library driven from Python and Kotlin through a real
download, a known recording, a known question and a delete (the checks are in `tests/`; run them yourself).

```
lib/verdure.dll                     the runtime: whisper.cpp + llama.cpp compiled in, x64
python/verdure.py + verdure.dll     the Python binding with the library beside it — import and go
kotlin/uniffi/verdure/verdure.kt    the Kotlin binding for a desktop JVM app (JNA)
bin/verdure.exe                     the developer tool: licences, manifests, what a device would get
bundles/example.json + .sig         a signed example manifest with real models (the same one the Android kit carries)
bundles/sandbox.token, issuer.pub   a ten-year sandbox licence and its issuer's public key
bundles/schema-*.json               example form schemas
tests/sdk_smoke.py, Smoke.kt        the tests this zip passed, as runnable examples of every call
sdk-integration-guide.md            the full guide; this file is the Windows-specific part
CHANGELOG.md, VERSION               what changed, and the commit and build time
```

## What the PC needs

- **Windows 10 or 11, 64-bit (x64).** On Windows-on-Arm the x64 library runs under emulation; a native
  arm64 build is not in this zip.
- **A processor with AVX2** (Intel from Haswell, 2013; AMD from 2015). Both engines use those
  instructions. On an older or budget processor (Sandy/Ivy Bridge, most Celeron and Pentium before 2021,
  Atom) `load()` refuses with `EngineUnavailable` and a sentence naming what the processor lacks, and
  `capabilities().notes` says so up front — it does not crash your program. Decide what your app shows.
- **Nothing to install.** The library carries its own C and C++ runtime: no Visual C++ Redistributable,
  no .NET, no Python packages. `verdure.dll` is the whole runtime.

## Python (3.9 or later, 64-bit)

Copy `python/verdure.py` and `python/verdure.dll` into your package, side by side.

```python
import verdure as v
from pathlib import Path

assets = Path("bundles")
# Read text as UTF-8, explicitly. The manifest contains characters outside ASCII; Python on Windows
# otherwise decodes with the system code page and the signature check fails with `Integrity`.
read = lambda name: (assets / name).read_text(encoding="utf-8")

policy = v.PolicyConfig(network_downloads=True, allowed_hosts=[], local_counters=False,
                        encrypted_storage=True, retention_days=None)
data_dir = Path.home() / "AppData" / "Local" / "YourApp" / "verdure"      # Local, not Roaming: see below
sdk = v.Verdure.start(read("sandbox.token").strip(), policy, str(data_dir))
sdk.load_manifest(read("example.json"), read("example.json.sig"), read("issuer.pub").strip())

bundle = sdk.recommend(v.Task.SCRIBE)              # what this PC can run for live speech; bundle.skipped says why not the rest
if not sdk.is_downloaded(bundle.id):
    sdk.download(bundle.id, listener)              # on_progress(done, total), on_status(sentence), is_cancelled()
sdk.load(bundle.id)

stream = sdk.transcribe(v.AsrConfig(lang_hints=["en"], keyterms=[], replacements=[]))
for ev in stream.push_pcm(floats, 16000):          # 16 kHz mono floats in [-1, 1]
    if isinstance(ev, v.TranscriptEvent.FINAL): print(ev.text)
for ev in stream.finish(): ...

facts = sdk.extract(transcript, schema_json).json  # JSON valid against YOUR schema, or VerdureError.SchemaInvalid
reply = sdk.chat([v.ChatMessage(role=v.Role.USER, content="…")], None)
```

Every call blocks; run them off your UI thread. Errors are `v.VerdureError.*` (`NoBundle`, `Memory`,
`EngineUnavailable`, `Policy`, `Integrity`, …) and every message is a sentence.

## Kotlin or Java on the desktop

Add `kotlin/uniffi/verdure/verdure.kt` to your sources, `net.java.dev.jna:jna:5.14.0` and
`org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0` to your dependencies, and tell JNA where the
library is: `-Djna.library.path=<folder containing verdure.dll>` (or put `verdure.dll` beside your
program's `.exe`). The calls are the ones in the Android README; `tests/Smoke.kt` is a complete program.

## Rust

Depend on the `verdure-ffi` crate from the repository with `features = ["llama", "whisper"]`. Building
the engines on Windows needs Visual Studio's C++ build tools, CMake and LLVM (for `libclang`), and these
set before `cargo build`, exactly as the workflow that built this zip sets them:

```
set GGML_NATIVE=OFF            & rem whisper.cpp otherwise compiles for YOUR processor and crashes on others
set LLAMA_STATIC_CRT=1         & rem with +crt-static below: no Visual C++ runtime to install
set RUSTFLAGS=-C target-feature=+crt-static -C link-arg=/FORCE:MULTIPLE
```

`/FORCE:MULTIPLE` is needed because whisper.cpp and llama.cpp each carry their own copy of the same
tensor library; the linker keeps the first. Both engines are then tested for real output, not assumed.

## Where the files go, and shipping models inside your own installer

Give `start()` a folder under **`%LOCALAPPDATA%`**, not `%APPDATA%`: models are hundreds of megabytes, and
`%APPDATA%` ("Roaming") is copied between PCs on office networks at every sign-in. A relative folder
works too; it is resolved against your program's working directory.

- `<data>\bundles\<bundle id>\<file>` — what each bundle uses.
- `<data>\models\<sha256>` — each model once, however many bundles use it (NTFS hard links: two bundles
  that share a model cost one download and one copy; on a volume without hard links each bundle keeps
  its own copy and nothing else changes).

**No download at all:** put the model files in `<data>\bundles\<bundle id>\` yourself — from your
installer, a USB stick, a file share. `is_downloaded()` recognises them by SHA-256 and `load()` uses them;
with `network_downloads=False` the SDK never opens a connection. A file that does not match its
fingerprint is not accepted. This path is tested with the network blocked.

Folder names with spaces, accents or non-Latin characters are fine (tested: `Vérdure tèst 数据`).
`delete(DeleteScope.ALL())` unloads the engines first, so it works while a bundle is loaded, and removes
both folders above.

## If the library will not load

- `OSError: [WinError 193] %1 is not a valid Win32 application` — a 32-bit Python or JVM. Use 64-bit.
- `FileNotFoundError: Could not find module 'verdure.dll'` (Python) — `verdure.dll` is not beside
  `verdure.py`.
- `UnsatisfiedLinkError: Unable to load library 'verdure'` (JVM) — `jna.library.path` does not point at
  the folder holding `verdure.dll`.
- `Integrity: … signature does not match` on files you did not change — they were read or
  saved with the wrong encoding or line endings. Read as UTF-8; if they came through git on Windows,
  check that `core.autocrlf` did not rewrite them (the repository's `.gitattributes` prevents it).

## Not in this zip

A C# binding (most Windows business software is .NET; the same generator family produces one and it is
the next packaging step), a native arm64 library, a build for processors without AVX2, GPU acceleration
(the engines run on the CPU), a signed DLL (some locked-down corporate PCs only load signed files), and a
`pip`/Maven package. Each is recorded in knowledge/26.
