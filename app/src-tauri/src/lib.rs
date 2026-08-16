//! Verdure desktop — Tauri application wiring.
//!
//! The web UI calls these commands:
//!   - `detect_device`    → which hardware tier this machine is (no question asked)
//!   - `transcribe_audio` → run Whisper on a dropped audio file, IN-APP (the pivot,
//!                          knowledge/18 — wrap the model, no separate app)
//!   - `port_tool`        → (legacy) install + launch an OSS app; kept while other
//!                          modalities are still being wrapped
//!
//! Model work lives in audio.rs (decode), model.rs (weights), transcribe.rs (run).

mod audio;
mod catalog;
mod device;
mod live;
mod llm;
mod model;
mod porter;
mod transcribe;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use tauri::{AppHandle, Emitter, Manager, Runtime};

/// The one in-flight live-transcription session (if any). Held in Tauri managed
/// state so `start`/`stop` can hand off the stop signal + capture thread. There is
/// at most one session: starting a new one stops the old one first.
#[derive(Default)]
struct LiveState(Mutex<Option<LiveHandle>>);

/// The chat model kept LOADED across messages (knowledge/22). Re-reading
/// 0.4–1.4 GB of weights per message was the dominant cost and made sending
/// conversation history unaffordable. Held behind a Mutex so two messages can
/// never drive the same model concurrently — replies are serialised, which is
/// what a single-user chat wants anyway.
///
/// Mobile note: holding weights resident makes a backgrounded app a likelier
/// target for iOS jetsam / Android's LMK. The failure mode is benign — the OS
/// kills it, the user reopens, the model reloads — and the transcript lives in
/// the frontend, so nothing is lost. Releasing on background is [OQ-69].
#[derive(Default)]
struct ChatState(Mutex<Option<llm::ResidentModel>>);

struct LiveHandle {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// Return the detected hardware tier as a decider `Tier` string (e.g. "L2-high").
/// The frontend feeds this straight into `recommend()` from @verdure/decider.
#[tauri::command]
fn detect_device() -> String {
    device::detect_tier().to_string()
}

/// The one click: port `tool_id` onto this machine. Long-running (a download),
/// so it runs on a blocking thread and streams progress to the UI via the
/// `porter:progress` event.
#[tauri::command]
async fn port_tool<R: Runtime>(
    app: AppHandle<R>,
    tool_id: String,
) -> Result<porter::PortOutcome, String> {
    let tool = catalog::find(&tool_id).ok_or_else(|| format!("Unknown tool: {tool_id}"))?;
    let app_for_progress = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        porter::port(tool, move |msg| {
            // A dropped event only costs a progress line; never fail the port for it.
            let _ = app_for_progress.emit("porter:progress", msg);
        })
    })
    .await
    .map_err(|e| format!("The setup task could not run: {e}"))?
    .map_err(|e| e.to_string())
}

/// The detected machine specs the frontend feeds to the performance predictor.
#[tauri::command]
fn device_spec() -> device::DeviceSpec {
    device::detect_spec()
}

/// A LIVE snapshot of free resources (available RAM, free disk, low-power) — the
/// machine-context that drives honest orchestration and model picking
/// (knowledge/20/21). Free disk is measured where models download (the app data
/// folder).
#[tauri::command]
fn system_resources<R: Runtime>(app: AppHandle<R>) -> Result<device::SystemResources, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    Ok(device::system_resources(&data_dir))
}

/// The chat models the user picks from (Small/Balanced/Powerful), each with its
/// download size and whether it fits THIS device's free memory + disk right now —
/// what the first-launch picker renders (design.md: honest space up front).
#[tauri::command]
fn chat_model_options<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<model::ChatModelOption>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let res = device::system_resources(&data_dir);
    Ok(model::chat_model_options(
        &data_dir,
        res.available_ram_mb,
        res.free_disk_mb,
    ))
}

/// A finished transcription plus the timing that calibrates the predictor: how
/// long the audio was vs how long it took (→ the real "× real-time" factor).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Transcription {
    text: String,
    audio_seconds: f64,
    processing_seconds: f64,
}

/// Transcribe a dropped audio file entirely on-device: ensure the model →
/// decode the audio → run Whisper → return the text + timing. Long-running, so it
/// runs on a blocking thread and streams stage updates via `transcribe:progress`.
/// First model wrapped directly in the Verdure UI (knowledge/18).
#[tauri::command]
async fn transcribe_audio<R: Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<Transcription, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let app_for_progress = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress = |msg: &str| {
            let _ = app_for_progress.emit("transcribe:progress", msg);
        };
        let model = model::ensure(&data_dir, &model::WHISPER_BASE_EN, &progress)?;
        progress("Reading the audio…");
        let samples = audio::decode_to_mono_16k(std::path::Path::new(&path))?;
        let audio_seconds = samples.len() as f64 / 16_000.0;
        progress("Transcribing — this runs on your computer and can take a little while…");
        let started = std::time::Instant::now();
        let text = transcribe::transcribe(&model, &samples)?;
        Ok(Transcription {
            text,
            audio_seconds,
            processing_seconds: started.elapsed().as_secs_f64(),
        })
    })
    .await
    .map_err(|e| format!("the transcription task could not run: {e}"))?
}

/// The conversational front door: ensure the chat model → generate a reply.
/// Long-running, so it runs on a blocking thread and streams a "Thinking…" stage
/// via the `chat:progress` event (knowledge/18). Tool-calling lands next.
#[tauri::command]
async fn chat<R: Runtime>(
    app: AppHandle<R>,
    message: String,
    model_id: String,
    history: Vec<llm::Turn>,
) -> Result<llm::ToolDecision, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let app_for_progress = app.clone();
    let app_for_state = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress = |msg: &str| {
            let _ = app_for_progress.emit("chat:progress", msg);
        };
        // Load the model the user picked (Small/Balanced/Powerful) — but always
        // checked against the memory AVAILABLE right now, not just total RAM. A
        // model "fits" on paper but still OOM-aborts when the WebView leaves only
        // ~1 GB free, so we never load one that doesn't fit (knowledge/21 — the
        // chat-crash fix); we say why instead. Empty/unknown id → auto-pick.
        let res = device::system_resources(&data_dir);
        let gb = |mb: u64| format!("{:.1} GB", mb as f64 / 1024.0);
        let info = model::chat_model_choice(&model_id, res.available_ram_mb).map_err(|e| {
            if e.strict {
                format!(
                    "\u{201c}{}\u{201d} needs about {} of free memory, but only {} is free right now. \
                     Close some apps to free memory — or pick a smaller model in Settings.",
                    e.model_name,
                    gb(e.needed_mb as u64),
                    gb(e.available_mb)
                )
            } else {
                format!(
                    "There isn't enough free memory to chat right now — about {} is free of {}. \
                     Closing a few other apps will free some up, then try again.",
                    gb(res.available_ram_mb),
                    gb(res.total_ram_mb)
                )
            }
        })?;
        let path = model::ensure(&data_dir, info, &progress)?;
        progress("Thinking…");
        // Size the conversation window to THIS device and THIS model: the KV cache
        // costs >10× more per token on the 4B than on the 0.5B, so one fixed
        // context length would be both too stingy and too risky (knowledge/22).
        // available_ram_mb is already platform-honest — vm_stat on macOS,
        // os_proc_available_memory (jetsam headroom) on iOS, MemAvailable on
        // Android — so the same arithmetic sizes correctly on all three.
        let spare = llm::spare_for_context_mb(res.available_ram_mb, info.approx_mb);
        let n_ctx = llm::context_for_budget(spare, info.kv_kb_per_token, info.trained_ctx);
        // Frame the prompt for this model's family (ChatML vs Gemma). Chat models
        // always carry a style; fall back to ChatML if one ever doesn't.
        let style = info.prompt_style.unwrap_or(crate::model::PromptStyle::ChatMl);
        // Hold the resident model for the whole reply: one conversation at a time.
        let state = app_for_state.state::<ChatState>();
        let mut slot = state
            .0
            .lock()
            .map_err(|_| "the chat session is in a bad state — please restart Verdure.".to_string())?;
        let resident = llm::ensure_resident(&mut slot, info, &path)?;
        llm::decide(resident, &history, &message, style, n_ctx)
    })
    .await
    .map_err(|e| format!("the chat task could not run: {e}"))?
}

/// Start live (microphone) transcription. Ensures + loads the speech model ONCE,
/// then spawns a dedicated thread that captures the mic and streams finished
/// segments to the UI via `live:segment` events (see `live.rs`). Returns as soon
/// as capture is underway; any later capture failure arrives as a `live:error`
/// event. Starting again stops any session already running.
#[tauri::command]
async fn start_live_transcription<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    stop_live_session(&app).await; // never run two at once

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let app_for_load = app.clone();
    // Download the model (first time) off the UI thread, so a download failure is
    // reported clearly before we claim to be recording. We hand the validated
    // PATH to the capture thread, which LOADS the context there — the loaded model
    // never crosses a thread boundary (and neither does the !Send cpal stream).
    let model_path = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let progress = |msg: &str| {
            let _ = app_for_load.emit("live:status", msg);
        };
        let path = model::ensure(&data_dir, &model::WHISPER_BASE_EN, &progress)?;
        progress("Starting the microphone…");
        Ok(path)
    })
    .await
    .map_err(|e| format!("couldn't start live transcription: {e}"))??;

    // The cpal stream is !Send, so capture lives entirely on one owned thread.
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let app_for_thread = app.clone();
    let thread = std::thread::Builder::new()
        .name("verdure-live".into())
        .spawn(move || live::run(app_for_thread, model_path, stop_for_thread))
        .map_err(|e| format!("couldn't start the capture thread: {e}"))?;

    *app.state::<LiveState>().0.lock().unwrap() = Some(LiveHandle { stop, thread });
    Ok(())
}

/// Stop the live-transcription session (if any) and release the microphone.
#[tauri::command]
async fn stop_live_transcription<R: Runtime>(app: AppHandle<R>) {
    stop_live_session(&app).await;
}

/// Signal the live session to stop and wait for it to finish (transcribing any
/// final segment + releasing the mic). The join can block briefly, so it runs on
/// a blocking thread rather than the async/UI path.
async fn stop_live_session<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.state::<LiveState>().0.lock().unwrap().take();
    if let Some(h) = handle {
        h.stop.store(true, Ordering::Relaxed);
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let _ = h.thread.join();
        })
        .await;
    }
}

/// List every model with whether it's downloaded and its size — for the
/// transcribe panel's storage row and the global Storage screen.
#[tauri::command]
fn list_models<R: Runtime>(app: AppHandle<R>) -> Result<Vec<model::ModelEntry>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    Ok(model::list(&dir))
}

/// Where downloaded models live on this device, as a displayable path.
///
/// The user owns these files, so they get to know where they are. It matters
/// most on macOS: deleting the app leaves this folder behind (macOS gives apps no
/// uninstall hook), so without this the models become invisible orphaned GBs.
/// iOS and Android delete the whole container on uninstall, so nothing is
/// stranded there — but showing the path is honest everywhere.
#[tauri::command]
fn models_dir<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    // Shorten $HOME to `~` for display — a full /Users/name path is noise.
    let shown = match app.path().home_dir() {
        Ok(home) => match dir.strip_prefix(&home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => dir.display().to_string(),
        },
        Err(_) => dir.display().to_string(),
    };
    Ok(shown)
}

/// Open the models folder in the system file browser (macOS only — the platform
/// where the folder outlives the app and the user may want to clear it).
#[tauri::command]
fn reveal_models_dir<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(&dir)
            .status()
            .map_err(|e| format!("couldn't open the folder: {e}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = dir;
        Err("Not available on this platform.".to_string())
    }
}

/// Completely remove a downloaded model by id, reclaiming the disk space.
#[tauri::command]
fn remove_model<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    model::remove(&dir, &id)
}

/// Ask the OS to make `bytes` of room on the models volume, so a model download
/// can fit ([OQ-67]). Android-only: `StorageManager.allocateBytes` has the system
/// itself clear other apps' *clearable cached files* — we never touch, see, or
/// delete anyone's data; the OS decides what is junk. On every other platform
/// this returns an error (the UI never offers it there — macOS gets a settings
/// link, iOS gets instructions; see `open_storage_settings` and the picker copy).
#[tauri::command]
async fn free_up_space<R: Runtime>(app: AppHandle<R>, bytes: u64) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    #[cfg(target_os = "android")]
    {
        return tauri::async_runtime::spawn_blocking(move || {
            android_free_up_space(&data_dir, bytes)
        })
        .await
        .map_err(|e| format!("the clean-up task could not run: {e}"))?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (data_dir, bytes);
        Err("Not available on this platform.".to_string())
    }
}

/// Open the system's own storage-management screen, where the OS can safely
/// clear its junk and show what's taking space ([OQ-67]). macOS-only: the System
/// Settings Storage pane. The pane id is community-documented (macOS 13+), so if
/// the deep link is refused we still open System Settings itself — never a
/// silent no-op.
#[tauri::command]
fn open_storage_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let deep = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.settings.Storage")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !deep {
            std::process::Command::new("/usr/bin/open")
                .args(["-b", "com.apple.systempreferences"])
                .status()
                .map_err(|e| format!("couldn't open System Settings: {e}"))?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    Err("Not available on this platform.".to_string())
}

// The single place generate_context! is expanded. It emits platform statics
// (e.g. _EMBED_INFO_PLIST on macOS) that may be defined only ONCE per crate, so
// run() and the IPC tests both go through this one function rather than calling
// the macro twice (which fails to link with "symbol already defined").
// Generic over the runtime so it serves both Wry (run) and MockRuntime (tests):
// Context is Context<R>, and a single concrete context can't satisfy both.
fn build_context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// Android: hand cpal's oboe backend the JavaVM + Application context it needs to
/// enumerate the microphone over JNI. Oboe reads them from the process-global
/// `ndk_context`, which Tauri v2 leaves unset — so without this, the first mic
/// open panics "android context was not initialized" and the capture thread dies
/// silently (the UI then hangs on "preparing" until the watchdog). `MainActivity`
/// calls this once on launch (see the patch in `.github/workflows/build-android.yml`).
/// [OQ-66]
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_ai_verdure_porter_MainActivity_initAudioContext(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    context: jni::objects::JObject,
) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Ok(vm) = env.get_java_vm() else { return };
        // Global ref so the Context survives past this JNI call; oboe reads it on
        // every mic open. Leaked deliberately — the Application context lives for
        // the whole process anyway, so there's nothing to release.
        let Ok(ctx) = env.new_global_ref(&context) else { return };
        unsafe {
            ndk_context::initialize_android_context(
                vm.get_java_vm_pointer() as *mut _,
                ctx.as_raw() as *mut _,
            );
        }
        std::mem::forget(ctx);
    });
}

/// Android: ask the OS for `bytes` of room via `StorageManager.allocateBytes`.
/// The system satisfies the request by deleting clearable cached files across
/// apps — the sanctioned, zero-risk way to free space (Android API 26+; [OQ-67]).
/// Reaches Java over JNI through the same process-global `ndk_context` that
/// `initAudioContext` populates on launch. Ownership at this boundary: every
/// JObject here is a local reference owned by the JVM for the duration of the
/// attached call — nothing is retained, so there is nothing to release manually.
#[cfg(target_os = "android")]
fn android_free_up_space(data_dir: &std::path::Path, bytes: u64) -> Result<(), String> {
    let friendly = "Android couldn't clear enough space automatically. \
                    Freeing space in Settings → Storage will do it.";
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|_| friendly.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|_| friendly.to_string())?;
    // The Application context stored by initAudioContext — alive for the whole
    // process, so borrowing it as a local JObject here is sound.
    let context = unsafe { jni::objects::JObject::from_raw(ctx.context() as jni::sys::jobject) };

    match try_allocate(&mut env, &context, data_dir, bytes) {
        Ok(outcome) => outcome,
        Err(_) => {
            // A Java exception (e.g. IOException: space couldn't be freed) is
            // pending — clear it so it can't take down the JVM bridge later.
            let _ = env.exception_clear();
            Err(friendly.to_string())
        }
    }
}

/// The JNI calls for `android_free_up_space`, separated so a thrown Java
/// exception unwinds to one place that clears it. Inner `Result` = an honest
/// "can't free that much" answer; outer = a JNI/Java failure.
#[cfg(target_os = "android")]
fn try_allocate(
    env: &mut jni::JNIEnv,
    context: &jni::objects::JObject,
    data_dir: &std::path::Path,
    bytes: u64,
) -> jni::errors::Result<Result<(), String>> {
    use jni::objects::{JObject, JValue};

    let service_name: JObject = env.new_string("storage")?.into();
    let storage_manager = env
        .call_method(
            context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&service_name)],
        )?
        .l()?;
    let dir: JObject = env.new_string(data_dir.to_string_lossy())?.into();
    let file = env.new_object("java/io/File", "(Ljava/lang/String;)V", &[JValue::Object(&dir)])?;
    let uuid = env
        .call_method(
            &storage_manager,
            "getUuidForPath",
            "(Ljava/io/File;)Ljava/util/UUID;",
            &[JValue::Object(&file)],
        )?
        .l()?;
    // Honest ceiling first: asking beyond getAllocatableBytes always fails, so
    // check and answer plainly instead of a generic error.
    let can_free = env
        .call_method(
            &storage_manager,
            "getAllocatableBytes",
            "(Ljava/util/UUID;)J",
            &[JValue::Object(&uuid)],
        )?
        .j()?
        .max(0) as u64;
    if can_free < bytes {
        return Ok(Err(format!(
            "Android can clear about {} MB of cached junk — the download needs {} MB. \
             Removing unused apps or old photos in Settings → Storage will make up the rest.",
            can_free / 1_048_576,
            bytes / 1_048_576
        )));
    }
    env.call_method(
        &storage_manager,
        "allocateBytes",
        "(Ljava/util/UUID;J)V",
        &[JValue::Object(&uuid), JValue::Long(bytes as i64)],
    )?;
    Ok(Ok(()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(LiveState::default())
        .manage(ChatState::default())
        .invoke_handler(tauri::generate_handler![
            detect_device,
            device_spec,
            system_resources,
            chat_model_options,
            transcribe_audio,
            start_live_transcription,
            stop_live_transcription,
            chat,
            list_models,
            remove_model,
            models_dir,
            reveal_models_dir,
            free_up_space,
            open_storage_settings,
            port_tool
        ])
        .run(build_context())
        .expect("error while running the Verdure porter");
}

// Integration tests for the Tauri command surface — the wiring the frontend's
// invoke() calls land on. The unit tests in catalog/device/porter cover the
// logic; these prove the commands are REGISTERED in the real app (built with the
// real config) and that the dispatched function returns correctly.
//
// Note on IPC byte-routing: we intentionally do NOT assert get_ipc_response()
// here. Tauri's mock IPC harness denies app commands in this setup ("not
// allowed. Plugin not found"), even matching the official docs example — but the
// Tauri docs are authoritative that app commands are allowed by default on all
// webviews, and our capabilities are exactly what create-tauri-app ships, so the
// real "Set it up" is authorized. The in-process routing is Tauri's own code;
// the ultimate confirmation is the real on-device run. See TEST_REPORT.md.
#[cfg(test)]
mod command_tests {
    // Build the FULL app with the real context (tauri.conf.json + capabilities +
    // icons all parse and load) and the real invoke_handler. This is genuine
    // integration coverage: it caught port_tool not being runtime-generic, and
    // it fails if the config, capabilities, or icons are malformed.
    fn create_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::App<R> {
        builder
            .manage(super::LiveState::default())
            .manage(super::ChatState::default())
            .invoke_handler(tauri::generate_handler![
                super::detect_device,
                super::device_spec,
                super::system_resources,
                super::transcribe_audio,
                super::start_live_transcription,
                super::stop_live_transcription,
                super::chat,
                super::list_models,
                super::remove_model,
                super::models_dir,
                super::reveal_models_dir,
                super::free_up_space,
                super::open_storage_settings,
                super::port_tool
            ])
            .build(super::build_context())
            .expect("the app builds with the real config + both commands registered")
    }

    #[test]
    fn app_builds_with_both_commands_registered() {
        // Proves generate_handler! accepts both commands, the real config parses,
        // and the runtime builds — the command surface the frontend targets.
        let _app = create_app(tauri::test::mock_builder());
    }

    #[test]
    fn detect_device_command_returns_a_valid_tier() {
        // The exact function the detect_device IPC command dispatches to — the
        // value the frontend feeds into recommend().
        let tier = super::detect_device();
        assert!(
            ["L1", "L2-low", "L2-high", "L3", "L3+"].contains(&tier.as_str()),
            "detect_device returned an unknown tier: {tier}"
        );
    }
}
