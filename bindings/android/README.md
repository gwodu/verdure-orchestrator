# Verdure SDK for Android — what is in this zip and how to add it

Produced by `.github/workflows/build-sdk-android.yml`. Contents:

```
jniLibs/arm64-v8a/libverdure.so     the runtime: whisper.cpp + llama.cpp compiled in, arm64, 16 KB page aligned
jniLibs/arm64-v8a/libc++_shared.so  the C++ runtime libverdure.so needs (from the NDK) — ship BOTH files
kotlin/uniffi/verdure/verdure.kt    the Kotlin binding (UniFFI); loads libverdure.so through JNA
bundles/example.json + .sig         a signed example manifest (scribe-floor and chw-forms-P1 have real hashes)
bundles/schema-facts.json           a facts schema; schema-example-chw.json a CHW form schema
bundles/sandbox.token               a ten-year sandbox licence; bundles/issuer.pub its issuer's public key
sdk-integration-guide.md            the full guide; this file is the Android-specific part
VERSION                             commit and build time
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
val bundle = sdk.recommend(Task.SCRIBE)           // or sdk.fit("scribe-floor") to judge a pin
if (!sdk.isDownloaded(bundle.id)) sdk.download(bundle.id, object : ProgressListener {
    override fun onProgress(done: ULong, total: ULong) { /* bytes */ }
    override fun onStatus(message: String) { }
    override fun isCancelled() = false
})
sdk.load(bundle.id)

// Microphone: AudioRecord at 16 kHz, 16-bit mono. Convert each buffer to floats in [-1, 1].
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
missing file is a sentence, not a silent no-op.

## Memory on a phone

`recommend` reads what the phone can give right now and refuses with the numbers when nothing fits;
`load` refuses the same way. The `scribe-floor` bundle needs about 1.3 GB; a 1.5B-class bundle about
2.2 GB. Unload (`delete(DeleteScope.All)` is not that — just drop the `Verdure` object) when the app
goes to the background on a 4 GB device.

## Not in this zip

An `.aar`, a Maven coordinate, and x86_64 for emulators. v0.1 ships the pieces; the packaging is a
recorded next step (knowledge/26).
