#!/usr/bin/env python3
"""Patch the freshly-generated Tauri Android project (app/src-tauri/gen/android).

`tauri android init` regenerates gen/android on every CI run (nothing under it is
committed), so the two Android workflows — the test-APK build (build-android.yml)
and the signed-AAB Play build (build-android-play.yml) — both run this once,
right after init, to apply the SAME two Android-only requirements for live
dictation. Keeping it in one file means the mic wiring can't drift between the
two build paths.

Two things are applied (both idempotent), plus the Play application ID:

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

# 3. Manifest: ACCESS_NETWORK_STATE — a normal permission (no prompt) that the
#    web search's Wi-Fi-only rule needs to read ConnectivityManager (web.rs
#    `network_state`). Without it the probe throws, the answer is "unknown",
#    and every search asks about mobile data (knowledge/24).
xml = manifest.read_text()
if "ACCESS_NETWORK_STATE" not in xml:
    xml = xml.replace(
        "<application",
        '<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />\n    <application',
        1,
    )
    manifest.write_text(xml)
    print(f"patched {manifest}: ACCESS_NETWORK_STATE")

# 4. Keep Verdure's memory off Google's servers (knowledge/25, "the rule most
#    likely to be broken silently"). Android auto backup copies an app's data
#    root to Google Drive; `context.sqlite` (and SQLite's -wal/-shm sidecars)
#    live at that root (Tauri's app_data_dir on Android is `Context.dataDir`),
#    so they are excluded by name in both rule formats: fullBackupContent for
#    Android 11 and below, dataExtractionRules for 12 and up. Models are not
#    excluded — the per-app backup quota (25 MB) already keeps them out.
res_xml = manifest.parent / "res" / "xml"
res_xml.mkdir(parents=True, exist_ok=True)
STORE_FILES = [
    "context.sqlite", "context.sqlite-wal", "context.sqlite-shm", "context.sqlite-journal",
    # The listener's hand-off file and the per-app opt-in list (context.rs).
    "context.spool", "context.spool.draining", "context.apps",
]
excludes = "".join(f'    <exclude domain="root" path="{f}" />\n' for f in STORE_FILES)
(res_xml / "backup_rules.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n<full-backup-content>\n' + excludes + "</full-backup-content>\n"
)
(res_xml / "data_extraction_rules.xml").write_text(
    '<?xml version="1.0" encoding="utf-8"?>\n<data-extraction-rules>\n'
    "  <cloud-backup>\n" + excludes + "  </cloud-backup>\n"
    "  <device-transfer>\n" + excludes + "  </device-transfer>\n"
    "</data-extraction-rules>\n"
)
xml = manifest.read_text()
if 'android:allowBackup="false"' not in xml:
    for attr, value in (
        ("android:fullBackupContent", "@xml/backup_rules"),
        ("android:dataExtractionRules", "@xml/data_extraction_rules"),
    ):
        present = re.search(rf'{re.escape(attr)}="([^"]*)"', xml)
        if present is None:
            xml = xml.replace("<application", f'<application\n        {attr}="{value}"', 1)
        else:
            assert present.group(1) == value, (
                f"the manifest already sets {attr}={present.group(1)!r}; refusing to "
                "guess which backup rules win. Reconcile them in this script."
            )
    manifest.write_text(xml)
    print(f"patched {manifest}: backup rules exclude {STORE_FILES}")

# Prove it on the manifest as written, not on the string we meant to write.
after = manifest.read_text()
assert "android.permission.ACCESS_NETWORK_STATE" in after, "ACCESS_NETWORK_STATE missing after patching"
assert 'android:allowBackup="false"' in after or (
    'android:fullBackupContent="@xml/backup_rules"' in after
    and 'android:dataExtractionRules="@xml/data_extraction_rules"' in after
), f"backup rules missing after patching:\n{after}"
for name in ("backup_rules.xml", "data_extraction_rules.xml"):
    text = (res_xml / name).read_text()
    assert all(f in text for f in STORE_FILES), f"{name} does not exclude every store file"

# 5. The notification listener (knowledge/25, [OQ-100]): a small Kotlin service
#    that reads MessagingStyle conversations from the apps the user opted into
#    and appends one JSON line per message to `context.spool` in the app's
#    private folder. It never calls native code — a background service that
#    crashed inside Rust would take the listener down with it — so the app
#    drains the spool into the store when it runs (context.rs `drain_spool`).
#    The JSON field names are the contract with `SpoolLine` in context.rs, and
#    DEFAULT_APPS mirrors `context::DEFAULT_APPS`; change both together.
#    Declared with BIND_NOTIFICATION_LISTENER_SERVICE so only the system can
#    bind it, and with the "conversations" default filter (Android 13+) so it
#    is never even offered non-conversation notifications.
listener = activity.parent / "VerdureListener.kt"
listener.write_text(
    """package ai.verdure.porter

import android.app.Notification
import android.app.Person
import android.os.Build
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import org.json.JSONObject
import java.io.File

/**
 * Verdure's memory feed (knowledge/25): the structured conversation each
 * messaging app posts as a notification, written as JSON lines to a spool
 * file in the app's private folder. Nothing leaves the device; the app reads
 * the spool into its store. Per-app opt-in via `context.apps` (one package
 * per line; absent = the defaults below). Never auto-replies, never acts.
 */
class VerdureListener : NotificationListenerService() {
  private val defaultApps = setOf(
    "com.whatsapp", "com.whatsapp.w4b", "org.telegram.messenger", "com.facebook.orca",
    "com.instagram.android", "com.google.android.apps.messaging", "com.snapchat.android",
  )

  override fun onNotificationPosted(sbn: StatusBarNotification) {
    try {
      record(sbn)
    } catch (t: Throwable) {
      // A malformed notification must never take the listener down.
    }
  }

  private fun allowed(pkg: String): Boolean {
    val f = File(dataDir, "context.apps")
    if (!f.exists()) return pkg in defaultApps
    return f.readLines().map { it.trim() }.any { it == pkg }
  }

  private fun record(sbn: StatusBarNotification) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) return
    val n = sbn.notification ?: return
    if ((n.flags and Notification.FLAG_GROUP_SUMMARY) != 0) return
    if (!allowed(sbn.packageName)) return
    val extras = n.extras ?: return
    @Suppress("DEPRECATION")
    val raw = extras.getParcelableArray(Notification.EXTRA_MESSAGES) ?: return
    val messages = Notification.MessagingStyle.Message.getMessagesFromBundleArray(raw)
    if (messages.isEmpty()) return
    val isGroup = extras.getBoolean(Notification.EXTRA_IS_GROUP_CONVERSATION, false)
    val title = (extras.getCharSequence(Notification.EXTRA_CONVERSATION_TITLE)
      ?: extras.getCharSequence(Notification.EXTRA_TITLE) ?: "").toString()
    @Suppress("DEPRECATION")
    val self = extras.getParcelable<Person>(Notification.EXTRA_MESSAGING_PERSON)
    val threadKey = (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) n.shortcutId else null)
      ?: sbn.tag ?: title.lowercase()
    val lines = StringBuilder()
    for (m in messages) {
      val text = m.text?.toString()?.trim() ?: continue
      if (text.isEmpty()) continue
      val who = m.senderPerson
      // MessagingStyle marks the user's own messages with a null sender, or
      // with the style's own user Person.
      val mine = who == null ||
        (self != null && ((who.key != null && who.key == self.key) || who.name == self.name))
      val o = JSONObject()
      o.put("app", sbn.packageName)
      o.put("threadKey", threadKey)
      o.put("threadTitle", title)
      o.put("isGroup", isGroup)
      o.put("sender", if (mine) "" else (who?.name?.toString() ?: ""))
      o.put("senderKey", if (mine) "" else (who?.key ?: who?.uri ?: who?.name?.toString() ?: ""))
      o.put("out", mine)
      o.put("atMs", m.timestamp)
      o.put("text", text)
      lines.append(o.toString()).append('\\n')
    }
    if (lines.isEmpty()) return
    File(dataDir, "context.spool").appendText(lines.toString())
  }
}
"""
)
print(f"wrote {listener}")

xml = manifest.read_text()
if "VerdureListener" not in xml:
    service = (
        '        <service\n'
        '            android:name=".VerdureListener"\n'
        '            android:exported="true"\n'
        '            android:label="Verdure"\n'
        '            android:permission="android.permission.BIND_NOTIFICATION_LISTENER_SERVICE">\n'
        '            <intent-filter>\n'
        '                <action android:name="android.service.notification.NotificationListenerService" />\n'
        '            </intent-filter>\n'
        '            <meta-data\n'
        '                android:name="android.service.notification.default_filter_types"\n'
        '                android:value="conversations" />\n'
        '        </service>\n'
        '    </application>'
    )
    assert xml.count("</application>") == 1, "unexpected manifest layout (application close tag)"
    xml = xml.replace("    </application>", service, 1) if "    </application>" in xml else xml.replace("</application>", service, 1)
    manifest.write_text(xml)
    print(f"patched {manifest}: VerdureListener service")

after = manifest.read_text()
assert 'android:name=".VerdureListener"' in after and "BIND_NOTIFICATION_LISTENER_SERVICE" in after, (
    f"listener service missing after patching:\\n{after}"
)
assert listener.is_file() and "class VerdureListener" in listener.read_text()
