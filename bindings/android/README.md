# Verdure SDK for Android — what is in this zip and how to add it

Produced by `.github/workflows/build-sdk-android.yml`. Contents:

```
jniLibs/arm64-v8a/libverdure.so     the runtime: whisper.cpp + llama.cpp compiled in, arm64, 16 KB page aligned
jniLibs/arm64-v8a/libc++_shared.so  the C++ runtime libverdure.so needs (from the NDK) — ship BOTH files
kotlin/uniffi/verdure/verdure.kt    the Kotlin binding (UniFFI); loads libverdure.so through JNA
bundles/example.json + .sig         a signed example manifest: scribe-small (Whisper small, recommended for clinical speech), scribe-floor (Whisper base, the live-speech pick on phones), scribe-L2, chw-forms-P1 — each with the speeds we measured per tier
bundles/schema-facts.json           a facts schema; schema-example-chw.json a CHW form schema
bundles/sandbox.token               a ten-year sandbox licence; bundles/issuer.pub its issuer's public key
sdk-integration-guide.md            the full guide; this file is the Android-specific part
VERSION                             commit and build time
CHANGELOG.md                        what changed in this build and the ones before it — read it when you update
```

## Add it to an Android app (minSdk 26, arm64)

1. Copy **both** `jniLibs/arm64-v8a/libverdure.so` and `jniLibs/arm64-v8a/libc++_shared.so` to
   `app/src/main/jniLibs/arm64-v8a/`. The second is the Android C++ runtime our library links against;
   without it the app fails at startup with `UnsatisfiedLinkError … library "libc++_shared.so" not found`.
   If your app already ships a `libc++_shared.so` (from another native library), keep one copy and add
   `packaging { jniLibs { pickFirsts += "**/libc++_shared.so" } }` to `app/build.gradle.kts`.
2. Copy `kotlin/uniffi/verdure/verdure.kt` to `app/src/main/java/uniffi/verdure/verdure.kt`.
3. In `app/build.gradle(.kts)` add:
   ```kotlin
   implementation("net.java.dev.jna:jna:5.14.0@aar")
   implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.9.0")
   ```
   and make sure `ndk { abiFilters += "arm64-v8a" }` (or ship a universal APK that includes arm64).
4. Put `bundles/*` in `app/src/main/assets/verdure/`.

## Use it

Every call blocks; run them on a background thread or a coroutine dispatcher, never on the main thread.

```kotlin
import uniffi.verdure.*

val policy = PolicyConfig(networkDownloads = true, allowedHosts = emptyList(),
                          localCounters = false, encryptedStorage = true, retentionDays = null)
val sdk = Verdure.start(assets.open("verdure/sandbox.token").readBytes().decodeToString().trim(),
                        policy, filesDir.resolve("verdure").absolutePath)
val caps = sdk.capabilities()                     // caps.tier e.g. "P3", caps.availableRamMb

sdk.loadManifest(asset("verdure/example.json"), asset("verdure/example.json.sig"), asset("verdure/issuer.pub").trim())
val bundle = sdk.recommend(Task.SCRIBE)           // live speech: scribe-floor on a Pixel 8a — scribe-small is passed over at 3.2× real time, and bundle.skipped says so
// val bundle = sdk.recommendFor(Task.SCRIBE, Pace.RECORDED)  // audio already captured, transcript may come later: scribe-small, the better words
// bundle.speedXRealTime: the measured speed on this tier (1.25 for scribe-floor on a Pixel 8a), or null when we have not measured it
if (!sdk.isDownloaded(bundle.id)) sdk.download(bundle.id, object : ProgressListener {
    override fun onProgress(done: ULong, total: ULong) { /* bytes across the whole bundle */ }
    override fun onStatus(message: String) { Log.i("Verdure", message) }  // "Downloading Speech model (Whisper small) (488 MB)…", "… reusing the copy another bundle downloaded", "… downloaded and verified."
    override fun isCancelled() = false
})
sdk.load(bundle.id)                               // 1.2 s for scribe-floor, 15 s for scribe-small on a Pixel 8a; nothing runs until it returns

// Microphone: AudioRecord at 16 kHz, 16-bit mono. Convert each buffer to floats in [-1, 1].
// keyterms stays empty: a 40-word clinical list made recognition worse in our measurements; use replacements for spellings.
val stream = sdk.transcribe(AsrConfig(langHints = listOf("en"), keyterms = listOf(), replacements = listOf()))
for (ev in stream.pushPcm(floats, 16000u)) when (ev) {
    is TranscriptEvent.Final -> show(ev.text, ev.startMs)
    is TranscriptEvent.Partial -> { }
}
stream.finish()

val facts = sdk.extract(transcriptSoFar, asset("verdure/schema-facts.json")).json   // schema-valid JSON
```

Errors arrive as `VerdureException` subclasses (`VerdureException.NoBundle`, `.EngineUnavailable`,
`.Memory`, …); every message is a sentence meant for a screen.

## If the library will not load

`java.lang.UnsatisfiedLinkError: Unable to load library 'verdure'` — read the second line:
- `library "libc++_shared.so" not found` → step 1 above: the C++ runtime file is missing from `jniLibs`.
- `library "libverdure.so" not found` → the APK does not contain arm64 libraries; check `abiFilters` and
  that the file is under `src/main/jniLibs/arm64-v8a/`, not `libs/`.
- anything mentioning `page size` → the device requires 16 KB alignment and an old build was used; this
  zip's library is aligned, rebuild the APK with it.
Never wrap `Verdure.start` in a try/catch that swallows the exception: show its message on a screen, so a
missing file is a sentence, not a silent no-op. Note that `UnsatisfiedLinkError` is a Java `Error`, not an
`Exception`: `catch (e: Exception)` does not see it. Catch `Throwable` around the first call.

## Threads

Every call blocks. `pushPcm` blocks when a 12 s segment completes — a few seconds with `scribe-floor`, up to
two minutes with `scribe-small` on a Pixel 8a, where it runs at 3.2× real time; `extract` blocks for 16–114 s
on a Pixel 8a with the 0.5B model; `download` for the whole download; `load` for 1–15 s. `Verdure` and each
`AsrStream` are locked separately, so run transcription on one dispatcher and extraction on another and
they overlap — the transcript does not wait for the facts. Two `extract` calls queue inside the engine.
Nothing on the main thread. The whisper engine emits `TranscriptEvent.Final` only; `Partial` is for
engines that stream and never arrives from this one.

## Memory on a phone, and who owns the engines

`recommend` and `load` read what the phone can give *right now* — 60 % of total RAM minus what this
process already holds, never below the kernel's `MemAvailable` — and refuse with the numbers when a
bundle does not fit. The figure falls after a `load` because the models now count against it. The
`scribe-floor` bundle needs about 1.3 GB (measured on a Pixel 8a: 683 MB PSS idle after load, 952 MB
peak during `extract`); `scribe-small` about 1.7 GB by the same rule (measured on a Pixel 8a: 635 MB PSS idle, 1,143 MB peak during transcription, 1,106 MB during `extract`); a 1.5B-class bundle about 2.5 GB.
Storage: a model is kept once per device, under `models/<sha256>`, and bundle folders link to it — `scribe-floor`
and `scribe-small` share the 491 MB language model, so holding both costs 1,127 MB of flash and the second
download is 488 MB, not 979. Sizes are decimal MB (1 MB = 1,000,000 bytes), as `downloadBytes` counts them.

The engines live inside the `Verdure` object: keep one instance for as long as your foreground service
records, in a scope that outlives the Activity (an application-level holder or the service itself). Drop
it only when the service stops; `load` after that is as fast as the first time (1.2 s for `scribe-floor`,
15 s for `scribe-small` on the Pixel 8a).
`delete(DeleteScope.All)` is not "unload" — it erases downloaded bundles and indexes.

## Not in this zip

An `.aar`, a Maven coordinate, and x86_64 for emulators. v0.1 ships the pieces; the packaging is a
recorded next step (knowledge/26).
