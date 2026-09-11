#!/usr/bin/env python3
"""Patch the freshly-generated Tauri Android project (app/src-tauri/gen/android).

`tauri android init` regenerates gen/android on every CI run (nothing under it is
committed), so the two Android workflows — the test-APK build (build-android.yml)
and the signed-AAB Play build (build-android-play.yml) — both run this once,
right after init, to apply the SAME two Android-only requirements for live
dictation. Keeping it in one file means the mic wiring can't drift between the
two build paths.

Three things are applied (all idempotent):

0. The Play application ID ([OQ-60]). Google Play identifies an app by its
   Gradle `applicationId`, which is PERMANENT from the first upload. Tauri has
   no config option for it (`AndroidConfig` offers only minSdkVersion /
   versionCode / autoIncrementVersionCode / debugApplicationIdSuffix), so it
   generates BOTH `namespace` and `applicationId` from the one global
   `identifier` in tauri.conf.json — which is also what macOS and iOS use.

   Changing that global identifier would drag the desktop app along: Tauri
   derives `app_data_dir()` from it, so the published, self-updating macOS build
   would silently look in a NEW folder and re-download ~1.7 GB of models it
   already has. It would also rename the Java package, and the JNI symbol in
   lib.rs is spelled from that package — a mismatch there kills the microphone
   SILENTLY (the capture thread dies; the UI hangs on "preparing").

   Android separates these two by design: `applicationId` is the install/Play
   identity, `namespace` is the Java package. So we patch ONLY `applicationId`.
   macOS and iOS never move, the JNI symbol is untouched, and — usefully — the
   eventual code-level rename becomes invisible to Play, since Play only ever
   sees `applicationId`.

1. RECORD_AUDIO in the manifest — Android needs it declared to record.
2. MainActivity wiring:
   - `initAudioContext(applicationContext)` — populates the process-global
     `ndk_context` (JavaVM + Context) that cpal's oboe backend reads to enumerate
     the mic over JNI. Tauri v2 leaves it unset, so without this the first mic
     open panics "android context was not initialized" and the capture thread
     dies (the UI hangs on "preparing"). Native symbol:
     Java_ai_verdure_porter_MainActivity_initAudioContext in lib.rs.
   - a runtime RECORD_AUDIO request (Android prompts in-app, unlike iOS's
     plist-driven prompt), re-checked on resume.

Run from the repo root: `python3 app/scripts/patch-android-project.py`.
"""

import os
import pathlib
import re

root = pathlib.Path("app/src-tauri/gen/android")

# The identifier Google Play will know this app by, FOREVER after the first
# upload ([OQ-60], founder decision 2026-09-10). Deliberately not the legacy
# "porter" name from before the 2026-06-18 pivot. See the module docstring for
# why this is patched here rather than set in tauri.conf.json.
PLAY_APPLICATION_ID = "ai.verdure.app"

# 0. Gradle: the Play application ID (leave `namespace` alone — see docstring).
gradle = root / "app" / "build.gradle.kts"
assert gradle.is_file(), f"expected Tauri's generated Gradle file at {gradle}"
g = gradle.read_text()

# `namespace` must stay as Tauri generated it: the JNI symbol in lib.rs is
# spelled from it. Assert we are not silently changing the Java package.
ns = re.search(r'^\s*namespace\s*=\s*"([^"]+)"', g, re.M)
assert ns, f"no `namespace` line in {gradle} — template shape changed:\n{g}"
assert ns.group(1) == "ai.verdure.porter", (
    f"namespace is {ns.group(1)!r}, expected 'ai.verdure.porter'. The JNI symbol "
    "in lib.rs (Java_ai_verdure_porter_MainActivity_initAudioContext) is spelled "
    "from this. Update both together or the mic dies silently."
)

app_ids = re.findall(r'^\s*applicationId\s*=\s*"([^"]+)"', g, re.M)
assert len(app_ids) == 1, (
    f"expected exactly one `applicationId` in {gradle}, found {len(app_ids)}: "
    f"{app_ids}. Refusing to guess which one Play uses."
)
if app_ids[0] != PLAY_APPLICATION_ID:
    g = re.sub(
        r'^(\s*applicationId\s*=\s*)"[^"]+"',
        lambda m: f'{m.group(1)}"{PLAY_APPLICATION_ID}"',
        g,
        count=1,
        flags=re.M,
    )
    gradle.write_text(g)
    print(f"patched {gradle}: applicationId -> {PLAY_APPLICATION_ID}")

# Prove the patch did what it claims, rather than trusting the write.
after = gradle.read_text()
assert re.search(rf'^\s*applicationId\s*=\s*"{re.escape(PLAY_APPLICATION_ID)}"', after, re.M), (
    f"applicationId is not {PLAY_APPLICATION_ID} after patching:\n{after}"
)
assert re.search(r'^\s*namespace\s*=\s*"ai\.verdure\.porter"', after, re.M), (
    "namespace changed while patching applicationId — the JNI symbol would break."
)

# 0b. Gradle: a monotonically increasing versionCode.
#
# Play REJECTS a bundle whose versionCode has been used before, and Tauri's
# template defaults it to 1 (`tauri.android.versionCode`, read from
# tauri.properties). gen/ is regenerated from scratch on every CI run, so
# Tauri's own `autoIncrementVersionCode` has no state to increment and would
# also produce 1 every time. The first upload would succeed and every upload
# after it would fail — the automation breaking the first time it matters.
#
# So CI passes a number that only ever goes up (the workflow run number). Left
# unset — a local build — the file is untouched and Tauri's default stands.
version_code = os.environ.get("VERDURE_ANDROID_VERSION_CODE")
if version_code:
    assert version_code.isdigit() and 0 < int(version_code) < 2_100_000_000, (
        f"VERDURE_ANDROID_VERSION_CODE must be a positive int below 2100000000, "
        f"got {version_code!r}"
    )
    props = root / "tauri.properties"
    assert props.is_file(), f"expected Tauri's properties file at {props}"
    text = props.read_text()
    key = "tauri.android.versionCode"
    if re.search(rf"^{re.escape(key)}=", text, re.M):
        text = re.sub(rf"^{re.escape(key)}=.*$", f"{key}={version_code}", text, count=1, flags=re.M)
    else:
        text = text.rstrip("\n") + f"\n{key}={version_code}\n"
    props.write_text(text)
    # Confirm the write rather than trusting it — a versionCode that silently
    # stayed at 1 fails much later, in Play, with an opaque message.
    assert re.search(rf"^{re.escape(key)}={version_code}$", props.read_text(), re.M), (
        f"versionCode did not take:\n{props.read_text()}"
    )
    print(f"patched {props}: {key}={version_code}")

# 1. Manifest: declare RECORD_AUDIO.
manifest = next(p for p in root.rglob("AndroidManifest.xml") if "app/src/main" in str(p))
xml = manifest.read_text()
if "RECORD_AUDIO" not in xml:
    assert "<application" in xml, "unexpected manifest layout"
    xml = xml.replace(
        "<application",
        '<uses-permission android:name="android.permission.RECORD_AUDIO" />\n    <application',
        1,
    )
    manifest.write_text(xml)
    print(f"patched {manifest}")

# 2. MainActivity: audio-context init + runtime mic permission.
activity = next(root.rglob("MainActivity.kt"))
kt = activity.read_text()
# The native init symbol in lib.rs is hard-coded to this package
# (Java_ai_verdure_porter_MainActivity_initAudioContext). If Tauri ever generates
# a different package, fail HERE with a clear message rather than at runtime with
# an UnsatisfiedLinkError.
assert "package ai.verdure.porter" in kt, (
    "MainActivity package changed — update the JNI symbol in lib.rs "
    f"(Java_ai_verdure_porter_MainActivity_initAudioContext):\n{kt}"
)
if "RECORD_AUDIO" not in kt:
    # The generated template already has its own onCreate (with enableEdgeToEdge
    # etc.) — do NOT rewrite the class (an early attempt did that and produced
    # broken Kotlin). Inject after the stable `super.onCreate(...)` anchor and
    # leave the template intact.
    anchor = "super.onCreate(savedInstanceState)"
    assert anchor in kt, f"onCreate shape changed — see ground-truth log above:\n{kt}"
    # initAudioContext MUST run before any mic open (see module docstring).
    kt = kt.replace(
        anchor,
        anchor + "\n    initAudioContext(applicationContext)\n    ensureMicPermission()",
        1,
    )
    # Add the helpers + onResume just before the class's final closing brace.
    helper = (
        "\n"
        "  external fun initAudioContext(ctx: android.content.Context)\n"
        "  private fun ensureMicPermission() {\n"
        "    if (checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) !=\n"
        "        android.content.pm.PackageManager.PERMISSION_GRANTED) {\n"
        "      requestPermissions(arrayOf(android.Manifest.permission.RECORD_AUDIO), 1)\n"
        "    }\n"
        "  }\n"
        "  override fun onResume() {\n"
        "    super.onResume()\n"
        "    ensureMicPermission()\n"
        "  }\n"
    )
    idx = kt.rstrip().rfind("}")
    kt = kt[:idx] + helper + kt[idx:]
    activity.write_text(kt)
    print(f"patched {activity}")
