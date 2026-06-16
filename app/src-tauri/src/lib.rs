//! Verdure desktop porter — Tauri application wiring.
//!
//! The web UI (the reused decider + intent cards) calls two commands:
//!   - `detect_device` → which hardware tier this machine is (no question asked)
//!   - `port_tool`     → install (if needed) + launch the chosen OSS tool
//!
//! Everything tool-specific lives in catalog.rs (data) and porter.rs (execution).

mod catalog;
mod device;
mod porter;

use tauri::{AppHandle, Emitter};

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
async fn port_tool(app: AppHandle, tool_id: String) -> Result<porter::PortOutcome, String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![detect_device, port_tool])
        .run(tauri::generate_context!())
        .expect("error while running the Verdure porter");
}
