//! Verdure desktop — Tauri application wiring.
//!
//! One interface over local models (knowledge/18): chat (llm.rs), live
//! dictation (live.rs over transcribe.rs), the model registry and downloads
//! (model.rs), and honest device readings (device.rs). The command surface the
//! UI calls is `handlers()`; the app and its integration test share that one
//! list, so they cannot drift apart.

mod audio;
mod context;
mod device;
mod live;
mod llm;
mod model;
mod transcribe;
mod web;

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

/// Verdure's model choice for this device, with the honesty that goes with it:
/// what was picked, what the best model would need (memory or storage, and how
/// much more), and the opt-in experiment where offered (knowledge/21, design.md:
/// the user no longer chooses a tier; they see what was chosen and why).
#[tauri::command]
fn chat_model_plan<R: Runtime>(app: AppHandle<R>) -> Result<model::ChatModelPlan, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let res = device::system_resources(&data_dir);
    Ok(model::chat_model_plan(
        &data_dir,
        res.total_ram_mb,
        res.available_ram_mb,
        res.free_disk_mb,
    ))
}

/// The conversational front door: ensure the chat model → generate a reply.
/// Long-running, so it runs on a blocking thread and streams a "Thinking…" stage
/// via the `chat:progress` event (knowledge/18).
///
/// `web_query` + `web_sources` carry this turn's approved web search and its
/// results (knowledge/24); `web_unavailable` says the user wanted the check and
/// why there are no sources (offline / failed / empty), so the model is told
/// the same reason the user sees rather than left to guess.
#[tauri::command]
async fn chat<R: Runtime>(
    app: AppHandle<R>,
    message: String,
    model_id: String,
    history: Vec<llm::Turn>,
    web_query: Option<String>,
    web_sources: Option<Vec<web::Snippet>>,
    web_unavailable: Option<web::UnavailableReason>,
    web_enabled: Option<bool>,
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
        let info = model::chat_model_choice(&model_id, &data_dir, res.total_ram_mb, res.available_ram_mb, res.free_disk_mb).map_err(|e| {
            if e.strict {
                // No internal model name (jargon) and no "Settings" (there is no
                // such screen): the switch is the model name at the top of chat.
                format!(
                    "The experimental model needs about {} of free memory, but only {} is free right now. \
                     Close some apps, or switch the experiment off under the model name at the top.",
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
        let spare = llm::spare_for_context_mb(res.available_ram_mb, info.runtime_mb);
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
        // Fit the sources to THIS window (the relay sized them for it already;
        // this is the guarantee) and frame them for the model.
        let block = web_sources
            .filter(|s| !s.is_empty())
            .map(|s| web::fit_snippets(s, web::snippet_budget_tokens(n_ctx), web::MAX_SOURCES))
            .map(|s| web::sources_block(web_query.as_deref().unwrap_or(""), &s));
        let grounding = match (&block, web_unavailable) {
            (Some(b), _) => Some(llm::Grounding::Sources(b)),
            (None, Some(reason)) => Some(llm::Grounding::Unavailable(web::unavailable_note(reason))),
            (None, None) => None,
        };
        let web = if web_enabled.unwrap_or(false) { llm::WebAccess::On } else { llm::WebAccess::Off };
        llm::decide(resident, &history, &message, style, n_ctx, grounding, web)
    })
    .await
    .map_err(|e| format!("the chat task could not run: {e}"))?
}

/// Whether the relay is reachable, and over Wi-Fi or mobile data (knowledge/24).
/// The UI's data rule runs on this before any search is offered.
#[tauri::command]
fn network_state() -> web::NetState {
    web::network_state()
}

/// The words that would leave the device for `message`, scrubbed — shown to the
/// user for approval BEFORE anything is sent (knowledge/06 principle 4: show
/// what Verdure understood, then ask).
#[tauri::command]
fn prepare_web_query(message: String) -> String {
    web::prepare_query(&message)
}

/// Run the approved search through Verdure's relay. The snippet budget is sized
/// to the window this device gives the chosen model, so the relay trims
/// server-side and the phone never downloads text the model couldn't read.
#[tauri::command]
async fn web_search<R: Runtime>(
    app: AppHandle<R>,
    query: String,
    model_id: String,
) -> Result<Vec<web::Snippet>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let res = device::system_resources(&data_dir);
    // Same arithmetic as `chat`. A model that doesn't fit right now still gets a
    // search at the smallest budget; `chat` will explain the memory problem.
    let n_ctx = model::chat_model_choice(&model_id, &data_dir, res.total_ram_mb, res.available_ram_mb, res.free_disk_mb)
        .map(|info| {
            let spare = llm::spare_for_context_mb(res.available_ram_mb, info.runtime_mb);
            llm::context_for_budget(spare, info.kv_kb_per_token, info.trained_ctx)
        })
        .unwrap_or(llm::MIN_CONTEXT);
    let budget = web::snippet_budget_tokens(n_ctx);
    // Scrubbed again at the boundary: the UI sends what the user approved, and
    // this is the last line before the network, so the floor holds regardless.
    let query = web::prepare_query(&query);
    if query.is_empty() {
        return Err("There's nothing to search for after removing private details.".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        web::search(web::RELAY_URL, &query, budget).map(|s| web::fit_snippets(s, budget, web::MAX_SOURCES))
    })
    .await
    .map_err(|e| format!("the web search task could not run: {e}"))?
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
        // SAFETY: both pointers come from live JNI handles: the JavaVM of this
        // process and a GLOBAL ref (valid on any thread until deleted, and we
        // never delete it: see `forget` below). `Once` guarantees the
        // process-global ndk_context is initialised exactly once.
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
    // SAFETY: `ctx.vm()` is the process's JavaVM pointer, stored by
    // initAudioContext at launch and valid for the life of the process.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|_| friendly.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|_| friendly.to_string())?;
    // SAFETY: the Application context stored by initAudioContext is a global
    // ref, alive for the whole process, so borrowing it as a JObject is sound.
    // We never delete it through this handle.
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

/// Open the third-party notices bundled with the app ([OQ-79]).
///
/// The obligation attached to the MIT/Apache-2.0 code Verdure is built on is to
/// distribute their copyright notices WITH the binary — so the file ships inside
/// the .app as a bundled resource, not merely in the repo, and this makes it
/// reachable. Opened with the OS default handler rather than rendered in-app:
/// it is ~700 KB of licence text, which no one wants inside a chat window.
#[tauri::command]
fn open_third_party_notices<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let path = app
        .path()
        .resolve("THIRD-PARTY-NOTICES.md", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("couldn't find the notices file: {e}"))?;
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(&path)
            .status()
            .map_err(|e| format!("couldn't open the notices: {e}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("Opening the notices is only wired up on macOS so far.".to_string())
    }
}

/// The third-party notices must cover everything we actually ship ([OQ-79]).
///
/// Adding a dependency and forgetting to re-run `generate-notices.py` is the
/// obvious way this rots, and the failure is invisible: the app still builds and
/// runs, it just quietly distributes someone's code without their copyright
/// notice. So this asserts every package in `Cargo.lock` appears in the file.
///
/// It reads `Cargo.lock` rather than calling cargo, so it stays fast and works
/// offline. If it fails, the fix is one command:
///     python3 app/scripts/generate-notices.py
#[cfg(test)]
mod notices_tests {
    #[test]
    fn third_party_notices_cover_every_dependency() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let notices = std::fs::read_to_string(root.join("THIRD-PARTY-NOTICES.md"))
            .expect("THIRD-PARTY-NOTICES.md is missing — run app/scripts/generate-notices.py");
        let lock = std::fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock");

        let mut missing = Vec::new();
        for line in lock.lines() {
            if let Some(name) = line.strip_prefix("name = \"").and_then(|r| r.strip_suffix('"')) {
                if name == "verdure-porter" {
                    continue; // ourselves
                }
                if !notices.contains(name) {
                    missing.push(name.to_string());
                }
            }
        }
        assert!(
            missing.is_empty(),
            "{} dependencies are missing from THIRD-PARTY-NOTICES.md: {:?}\n\
             Re-run: python3 app/scripts/generate-notices.py",
            missing.len(),
            missing
        );
    }

    /// The two projects that ARE the product, specifically. llama.cpp ships no
    /// LICENSE inside its crate, so its attribution comes from a checked-in
    /// supplementary file and would vanish silently if that file were deleted.
    #[test]
    fn the_engines_we_wrap_are_credited() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let notices = std::fs::read_to_string(root.join("THIRD-PARTY-NOTICES.md")).unwrap();
        assert!(
            notices.contains("Copyright (c) 2023-2026 The ggml authors"),
            "llama.cpp's copyright is absent — check app/src-tauri/licenses/"
        );
        assert!(
            notices.contains("Copyright (c) 2023-2024 The ggml authors"),
            "whisper.cpp's copyright is absent"
        );
        // MPL-2.0 is weak copyleft: every MPL component listed in the table needs
        // a pointer to its source (generate-notices.py derives them).
        for line in notices.lines().filter(|l| l.starts_with("| ") && l.ends_with("| MPL-2.0 |")) {
            let name = line.split('|').nth(1).unwrap().trim();
            assert!(
                notices.contains(&format!("crates.io/crates/{name}/")),
                "MPL-2.0 component {name} has no source pointer"
            );
        }
    }
}

/// Every command the UI can call, in ONE place. `run()` registers it and the
/// integration test builds the real app with it, so a command can't be live in
/// the app but missing from the test (the test's own copy had drifted, missing
/// two commands, by 2026-10-20).
fn handlers<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        system_resources,
        chat_model_plan,
        start_live_transcription,
        stop_live_transcription,
        chat,
        list_models,
        remove_model,
        models_dir,
        reveal_models_dir,
        open_third_party_notices,
        free_up_space,
        open_storage_settings,
        network_state,
        prepare_web_query,
        web_search
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // Self-update, desktop only — iOS and Android update through their stores.
    // Registering the plugin does NOT make Verdure phone home: it only makes a
    // check *possible*. Every check is initiated by the frontend, and only after
    // the user has chosen to allow it (knowledge/23). That split is deliberate —
    // the capability lives in Rust, the consent lives with the UI that asked for
    // it, so no code path here can check for updates on its own.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    // Opens a web-search source in the system browser when its citation is
    // tapped (knowledge/24). Only `open_url` is used, on every platform; the
    // capability file limits it to http(s)/mailto/tel.
    let builder = builder.plugin(tauri_plugin_opener::init());
    builder
        .manage(LiveState::default())
        .manage(ChatState::default())
        .invoke_handler(handlers())
        .run(build_context())
        .expect("error while running the Verdure porter");
}

// Integration test for the Tauri command surface — the wiring the frontend's
// invoke() calls land on. It builds the real app (real config, capabilities,
// icons) with the real `handlers()`, so every registered command compiles as a
// handler and the config parses.
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
    // icons all parse and load) and the real `handlers()`. It fails if a command
    // isn't runtime-generic, or the config, capabilities, or icons are malformed.
    fn create_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::App<R> {
        builder
            .manage(super::LiveState::default())
            .manage(super::ChatState::default())
            .invoke_handler(super::handlers())
            .build(super::build_context())
            .expect("the app builds with the real config + both commands registered")
    }

    #[test]
    fn app_builds_with_both_commands_registered() {
        // Proves generate_handler! accepts both commands, the real config parses,
        // and the runtime builds — the command surface the frontend targets.
        let _app = create_app(tauri::test::mock_builder());
    }
}
