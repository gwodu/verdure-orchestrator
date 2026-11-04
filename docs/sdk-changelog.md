# Verdure SDK — what changed, build by build

Newest first. The build in your hands is named in `VERSION` (commit and time); the Android zip is
`verdure-android-arm64.zip` from `build-sdk-android.yml`, and this file ships inside it as `CHANGELOG.md`.
Sizes are decimal MB (1 MB = 1,000,000 bytes), the unit `download_bytes` counts in.

## 2026-11-04, second build — speed-aware choice, one copy per model, plainer errors

Prompted by the second Pixel 8a run (`knowledge/26`, "Small on the Pixel"): `recommend` had chosen a bundle
the phone could not run live, the shared language model was downloaded twice, and an error message meant
for the model reached the screen.

- **`recommend(task)` now refuses a bundle measured too slow for live speech on this device's tier**, and
  says so: `skipped` carries "scribe-small: runs at 3.2× real time on P3, too slow to keep pace with live
  speech". On a Pixel 8a `recommend(Task.SCRIBE)` therefore returns `scribe-floor` again; on a laptop it
  returns `scribe-small` as before. New: `Pace { Live, Recorded }`, `recommend_for(task, pace)` and
  `fit_for(bundle_id, pace)` — `Recorded` means the audio is already captured and the transcript may come
  later, so the slower, better bundle is allowed; `recommend` and `fit` mean `Live`. `BundleRef` gained
  `speed_x_real_time` (the measurement for this tier, or none). The rule: above 1.5× real time, measured flat
  out and sustained, is not live; a tier with no measurement is not held against a bundle.
- **The manifest carries the measurements**: a `speed` map per bundle, `{"P3": 3.2, "L2": 0.75}`;
  `verdure manifest check` prints it and refuses a speed for a tier the bundle is not built for.
- **Models are stored once per device**, in `<data_dir>/models/<sha256>`, and bundle folders hard-link to
  them. `scribe-floor` and `scribe-small` share the 491 MB language model: the second download is 488 MB,
  not 979, and both bundles cost 1,127 MB of storage, not 1,618. A device that already holds two copies
  folds them into one on its next `download` of either bundle.
- **`ProgressListener.on_status` speaks during a download**, one sentence per file: "Downloading Speech
  model (Whisper small) (488 MB)…", "… downloaded and verified.", "Checking …", "… reusing the copy another
  bundle downloaded (491 MB not downloaded again).", "… is already on this device."
- **`SchemaInvalid` messages describe the violation only** ("$.facts[0].text: still holds the placeholder
  …"). The instruction to the model ("put the value from the text, or null") stayed in the repair prompt,
  where it belongs. Log ours; show the user your own sentence.
- **Kotlin**: `BundleRef` has one more field and `Pace` is a new enum. Code that reads `BundleRef` fields
  compiles unchanged; code that constructs one by hand needs the new argument.
- **Docs**: `load` time (1.2 s `scribe-floor`, 15 s `scribe-small` on a Pixel 8a); `push_pcm` can block for
  up to two minutes with `scribe-small` on a phone; where files go; decimal MB throughout; this changelog.

## d8b8ee5 · 2026-11-04T03:30Z — Whisper small

- New `scribe-small` bundle (Whisper small + Qwen2.5 0.5B, 979 MB) leads the signed manifest for 8 GB phones
  and laptops; `scribe-floor` (Whisper base) stays listed for every tier. Measured on ten PriMock57
  consultations: clinical-term misses 1 in 3.6 → 1 in 6 (guide, section 7b).
- `scribe-L2`'s 1.5B language model has its real hash (downloadable; extraction unmeasured).
- `AsrConfig.keyterms` documented as experimental — a 40-word list made recognition worse; leave it empty
  and use `replacements`.
- Since the 2026-11-03 Android memory budget, `capabilities().available_ram_mb` follows what the process
  holds, so it falls after `load` (the first phone report had seen it unchanged).

## 513ded3 · 2026-10-30 — the build the first phone run used

- `libc++_shared.so` ships in the zip beside `libverdure.so` (the first integration attempt failed at
  `dlopen` without it).
- Android memory budget: 60 % of total RAM minus what this process holds, never below `MemAvailable`.
- One generation at a time inside the engine; the threading contract in the README.
