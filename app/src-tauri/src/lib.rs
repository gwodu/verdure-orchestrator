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

use tauri::{AppHandle, Emitter, Runtime};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![detect_device, port_tool])
        .run(tauri::generate_context!())
        .expect("error while running the Verdure porter");
}

// Integration tests for the Tauri command surface — the wire the frontend's
// invoke() calls land on. The unit tests in catalog/device/porter cover the
// logic; these prove the commands are actually REGISTERED and REACHABLE through
// the real runtime (the gap that isolated unit tests + a simulated browser
// preview leave open). Uses Tauri's MockRuntime so it runs headlessly in CI.
//
// Only `detect_device` is invoked (it's pure + side-effect-free). `port_tool` is
// deliberately NOT invoked here — it would run a real `brew install` and launch
// Buzz; its registration is proven by the app building with it in the handler,
// and its install recipe is verified separately by the CI `brew install` step.
#[cfg(test)]
mod ipc_tests {
    // Build the mock app with the REAL context (generate_context!) rather than a
    // bare mock_context — so the actual capabilities/default.json loads and the
    // "main" window is authorized for IPC, exactly as in the shipped app. This
    // also validates that tauri.conf.json, the capabilities, and the icons parse.
    fn create_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::App<R> {
        builder
            .invoke_handler(tauri::generate_handler![super::detect_device, super::port_tool])
            .build(tauri::generate_context!())
            .expect("the app builds with the real config + both commands registered")
    }

    #[test]
    fn app_builds_with_both_commands_registered() {
        // Proves generate_handler! accepts both commands, the real config parses,
        // and the runtime builds.
        let _app = create_app(tauri::test::mock_builder());
    }

    #[test]
    fn detect_device_is_invokable_over_ipc() {
        use tauri::Manager;
        let app = create_app(tauri::test::mock_builder());
        // Reuse the configured "main" window if the build created it; otherwise
        // make one with the same label so the capability (windows: ["main"])
        // authorizes it.
        let webview = app.get_webview_window("main").unwrap_or_else(|| {
            tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
                .build()
                .expect("mock webview builds")
        });

        let res = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "detect_device".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::default(),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.to_string(),
            },
        );

        // Ok means: the command is registered, the window is authorized, the call
        // routed to the handler, ran, and serialized a return value — the exact
        // path the frontend's invoke('detect_device') exercises.
        assert!(
            res.is_ok(),
            "detect_device should be reachable via IPC and return Ok, got: {res:?}"
        );
    }
}
