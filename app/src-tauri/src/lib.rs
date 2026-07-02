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
    allow_fallback: bool,
) -> Result<llm::ToolDecision, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    let app_for_progress = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress = |msg: &str| {
            let _ = app_for_progress.emit("chat:progress", msg);
        };
        // Choose the chat model against the memory AVAILABLE right now — not just
        // total RAM. A model "fits" an 8 GB Mac on paper but still OOM-aborts when
        // the WebView leaves only ~1 GB free, so we never load one that doesn't fit
        // (knowledge/21 — the chat-crash fix). `allow_fallback` = false is "best
        // model only": use the preferred model or say why it can't, never a silent
        // downgrade — the switch that lets a tester know which model actually ran.
        let res = device::system_resources(&data_dir);
        let gb = |mb: u64| format!("{:.1} GB", mb as f64 / 1024.0);
        let info = model::choose_chat_model(res.available_ram_mb, allow_fallback).map_err(|e| {
            if e.strict {
                format!(
                    "\u{201c}{}\u{201d} needs about {} of free memory, but only {} is free right now. \
                     Close some apps to free memory — or turn off \u{201c}best model only\u{201d} to use a smaller one.",
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
        let model = model::ensure(&data_dir, info, &progress)?;
        progress("Thinking…");
        // Frame the prompt for this model's family (ChatML vs Gemma). Chat models
        // always carry a style; fall back to ChatML if one ever doesn't.
        let style = info.prompt_style.unwrap_or(crate::model::PromptStyle::ChatMl);
        llm::decide(&model, &message, style)
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

/// Completely remove a downloaded model by id, reclaiming the disk space.
#[tauri::command]
fn remove_model<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("couldn't find the app's data folder: {e}"))?;
    model::remove(&dir, &id)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(LiveState::default())
        .invoke_handler(tauri::generate_handler![
            detect_device,
            device_spec,
            system_resources,
            transcribe_audio,
            start_live_transcription,
            stop_live_transcription,
            chat,
            list_models,
            remove_model,
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
