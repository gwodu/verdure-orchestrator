#!/usr/bin/env python3
"""Patch the freshly-generated Tauri Android project (app/src-tauri/gen/android).

`tauri android init` regenerates gen/android on every CI run (nothing under it is
committed), so the two Android workflows — the test-APK build (build-android.yml)
and the signed-AAB Play build (build-android-play.yml) — both run this once,
right after init, to apply the SAME two Android-only requirements for live
dictation. Keeping it in one file means the mic wiring can't drift between the
two build paths.

Two things are applied (both idempotent):

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

import pathlib

root = pathlib.Path("app/src-tauri/gen/android")

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
