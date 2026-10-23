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
    # The listener's hand-off file and the unticked-apps list (context.rs).
    "context.spool", "context.spool.draining", "context.off",
    # The conversation itself (transcript.rs).
    "chat.jsonl",
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
#    that reads what the phone shows the user — conversations message by
#    message (MessagingStyle), anything else as a notice with its title and
#    text — from every app except the ones the user unticked, and appends one
#    JSON line per record to `context.spool` in the app's private folder. It
#    never calls native code — a background service that crashed inside Rust
#    would take the listener down with it — so the app drains the spool into
#    the store when it runs (context.rs `drain_spool`). The JSON field names
#    are the contract with `SpoolLine` in context.rs; `context.off` (one
#    package per line) is the contract with `read_off`/`write_off` there.
#    Declared with BIND_NOTIFICATION_LISTENER_SERVICE so only the system can
#    bind it. No default filter: every kind of notification is offered, and
#    the service decides what is information and what is machinery.
listener = activity.parent / "VerdureListener.kt"
listener.write_text(
    r"""package ai.verdure.porter

import android.app.Notification
import android.app.Person
import android.os.Build
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import org.json.JSONObject
import java.io.File

/**
 * Verdure's memory feed (knowledge/25): what the phone shows the user, written
 * as JSON lines to a spool file in the app's private folder. A conversation
 * (MessagingStyle) is written message by message with sender and time; any
 * other notification is written as a notice with its title and text. Every
 * app is read unless the user unticked it (`context.off`, one package per
 * line). Nothing leaves the device; the app reads the spool into its store.
 * Never auto-replies, never acts.
 */
class VerdureListener : NotificationListenerService() {
  // The system's own chatter is not the user's life: Android itself, the
  // status bar, the store, Play services — and Verdure's own notifications.
  private val systemApps = setOf(
    "android", "com.android.systemui", "com.android.vending", "com.google.android.gms",
  )
  // Machinery, not information: playback controls, progress bars, service
  // notices, the phone's own status, timers and turn-by-turn directions.
  private val noise = setOf(
    Notification.CATEGORY_SERVICE, Notification.CATEGORY_PROGRESS, Notification.CATEGORY_TRANSPORT,
    Notification.CATEGORY_SYSTEM, Notification.CATEGORY_STATUS, Notification.CATEGORY_STOPWATCH,
    Notification.CATEGORY_NAVIGATION,
  )

  override fun onNotificationPosted(sbn: StatusBarNotification) {
    try {
      record(sbn)
    } catch (t: Throwable) {
      // A malformed notification must never take the listener down.
    }
  }

  private fun unticked(pkg: String): Boolean {
    val f = File(dataDir, "context.off")
    if (!f.exists()) return false
    return f.readLines().map { it.trim() }.any { it == pkg }
  }

  private fun record(sbn: StatusBarNotification) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) return
    val pkg = sbn.packageName
    if (pkg == packageName || pkg in systemApps || unticked(pkg)) return
    val n = sbn.notification ?: return
    if ((n.flags and Notification.FLAG_GROUP_SUMMARY) != 0) return
    // Ongoing notifications are state, not events: a call in progress, a
    // download, a foreground service's own notice.
    if ((n.flags and (Notification.FLAG_ONGOING_EVENT or Notification.FLAG_FOREGROUND_SERVICE)) != 0) return
    val category = n.category
    if (category != null && category in noise) return
    val extras = n.extras ?: return
    @Suppress("DEPRECATION")
    val raw = extras.getParcelableArray(Notification.EXTRA_MESSAGES)
    val messages = if (raw != null) Notification.MessagingStyle.Message.getMessagesFromBundleArray(raw) else emptyList()
    val lines = StringBuilder()
    if (messages.isNotEmpty()) {
      val isGroup = extras.getBoolean(Notification.EXTRA_IS_GROUP_CONVERSATION, false)
      val title = (extras.getCharSequence(Notification.EXTRA_CONVERSATION_TITLE)
        ?: extras.getCharSequence(Notification.EXTRA_TITLE) ?: "").toString()
      @Suppress("DEPRECATION")
      val self = extras.getParcelable<Person>(Notification.EXTRA_MESSAGING_PERSON)
      val threadKey = (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) n.shortcutId else null)
        ?: sbn.tag ?: title.lowercase()
      for (m in messages) {
        val text = m.text?.toString()?.trim() ?: continue
        if (text.isEmpty()) continue
        val who = m.senderPerson
        // MessagingStyle marks the user's own messages with a null sender, or
        // with the style's own user Person.
        val mine = who == null ||
          (self != null && ((who.key != null && who.key == self.key) || who.name == self.name))
        val o = JSONObject()
        o.put("app", pkg)
        o.put("threadKey", threadKey)
        o.put("threadTitle", title)
        o.put("isGroup", isGroup)
        o.put("sender", if (mine) "" else (who?.name?.toString() ?: ""))
        o.put("senderKey", if (mine) "" else (who?.key ?: who?.uri ?: who?.name?.toString() ?: ""))
        o.put("out", mine)
        o.put("atMs", m.timestamp)
        o.put("text", text)
        lines.append(o.toString()).append('\n')
      }
    } else {
      // Anything else the phone showed: its title and its text, as the user
      // saw them. The expanded forms first — they carry the whole message.
      val title = (extras.getCharSequence(Notification.EXTRA_TITLE_BIG)
        ?: extras.getCharSequence(Notification.EXTRA_TITLE) ?: "").toString().trim()
      val body = (extras.getCharSequence(Notification.EXTRA_BIG_TEXT)
        ?: extras.getCharSequence(Notification.EXTRA_TEXT)
        ?: extras.getCharSequenceArray(Notification.EXTRA_TEXT_LINES)?.joinToString("\n")
        ?: "").toString().trim()
      if (title.isEmpty() && body.isEmpty()) return
      val o = JSONObject()
      o.put("kind", "notice")
      o.put("app", pkg)
      o.put("title", title)
      o.put("text", body)
      o.put("atMs", sbn.postTime)
      lines.append(o.toString()).append('\n')
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
        '        </service>\n'
        '    </application>'
    )
    assert xml.count("</application>") == 1, "unexpected manifest layout (application close tag)"
    xml = xml.replace("    </application>", service, 1) if "    </application>" in xml else xml.replace("</application>", service, 1)
    manifest.write_text(xml)
    print(f"patched {manifest}: VerdureListener service")
# An older patch limited the listener to conversations; every notification is
# read now (knowledge/25, 2026-10-23). Drop the filter if a tree still has it.
xml = manifest.read_text()
if "default_filter_types" in xml:
    xml = re.sub(r"\s*<meta-data\s+android:name=\"android\.service\.notification\.default_filter_types\"[^>]*/>", "", xml)
    manifest.write_text(xml)
    print(f"patched {manifest}: removed the conversations-only filter")

after = manifest.read_text()
assert 'android:name=".VerdureListener"' in after and "BIND_NOTIFICATION_LISTENER_SERVICE" in after, (
    f"listener service missing after patching:\n{after}"
)
assert "default_filter_types" not in after, "the listener is still filtered to conversations"
assert listener.is_file() and "class VerdureListener" in listener.read_text()

# 6. Background work (knowledge/11, "Mobile non-negotiables"): a foreground
#    service the Rust core starts while a reply is being written, a model is
#    downloading or a dictation is running (background.rs), so Android neither
#    freezes nor kills the process when the user switches apps. It does no
#    work itself; it holds the process's foreground standing and shows one
#    quiet notification naming the work. Reached by an explicit Intent with
#    the class name, so the Rust side needs no app class from a native thread.
#    Permissions: FOREGROUND_SERVICE plus the per-type ones Android 14 requires
#    (dataSync for a reply or download, microphone for dictation), and
#    POST_NOTIFICATIONS so the notification can be shown on 13+ if the user
#    allows it — the service runs either way.
work = activity.parent / "VerdureWork.kt"
work.write_text(
    r"""package ai.verdure.porter

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder

/**
 * Keeps Verdure's work alive when the user switches apps (knowledge/11,
 * "Mobile non-negotiables"): a reply being written, a model downloading, a
 * dictation running. Started and stopped by the Rust core (background.rs)
 * with the kind of work in the Intent's "kind" extra; shows one quiet
 * notification naming it. Does no work itself.
 */
class VerdureWork : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onCreate() {
    super.onCreate()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(CHANNEL, "Working in the background", NotificationManager.IMPORTANCE_LOW)
      channel.description = "Shown while Verdure finishes something you started."
      channel.setShowBadge(false)
      getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val kind = intent?.getStringExtra("kind") ?: "working"
    val text = when (kind) {
      "listening" -> "Listening…"
      "thinking" -> "Thinking…"
      else -> "Working…"
    }
    val open = packageManager.getLaunchIntentForPackage(packageName)
    val tap = if (open != null) {
      PendingIntent.getActivity(this, 0, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    } else null
    @Suppress("DEPRECATION")
    val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) Notification.Builder(this, CHANNEL) else Notification.Builder(this)
    val notification = builder
      .setSmallIcon(applicationInfo.icon)
      .setContentTitle("Verdure")
      .setContentText(text)
      .setOngoing(true)
      .setContentIntent(tap)
      .build()
    val type = if (kind == "listening") ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
      else ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) startForeground(ID, notification, type)
    else startForeground(ID, notification)
    return START_NOT_STICKY
  }

  override fun onDestroy() {
    stopForeground(STOP_FOREGROUND_REMOVE)
    super.onDestroy()
  }

  companion object {
    private const val CHANNEL = "verdure.work"
    private const val ID = 7
  }
}
"""
)
print(f"wrote {work}")

xml = manifest.read_text()
for perm in (
    "android.permission.FOREGROUND_SERVICE",
    "android.permission.FOREGROUND_SERVICE_DATA_SYNC",
    "android.permission.FOREGROUND_SERVICE_MICROPHONE",
    "android.permission.POST_NOTIFICATIONS",
):
    if perm not in xml:
        xml = xml.replace("<application", f'<uses-permission android:name="{perm}" />\n    <application', 1)
if "VerdureWork" not in xml:
    service = (
        '        <service\n'
        '            android:name=".VerdureWork"\n'
        '            android:exported="false"\n'
        '            android:foregroundServiceType="dataSync|microphone" />\n'
        '    </application>'
    )
    assert xml.count("</application>") == 1, "unexpected manifest layout (application close tag)"
    xml = xml.replace("    </application>", service, 1) if "    </application>" in xml else xml.replace("</application>", service, 1)
manifest.write_text(xml)
print(f"patched {manifest}: VerdureWork service + foreground-service permissions")

after = manifest.read_text()
for needle in (
    "android.permission.FOREGROUND_SERVICE\"",
    "android.permission.FOREGROUND_SERVICE_DATA_SYNC",
    "android.permission.FOREGROUND_SERVICE_MICROPHONE",
    "android.permission.POST_NOTIFICATIONS",
    'android:name=".VerdureWork"',
    'android:foregroundServiceType="dataSync|microphone"',
):
    assert needle in after, f"{needle} missing after patching:\n{after}"
assert work.is_file() and "class VerdureWork" in work.read_text()
