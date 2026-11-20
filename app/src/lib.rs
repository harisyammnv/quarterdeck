//! The Quarterdeck window: a thin Tauri shell over `quarterdeck-core`.
//!
//! Every command is listed in `capabilities/default.json`; the window has no shell, file
//! system or network permission of its own.

use std::io::Write;
use std::path::PathBuf;

use quarterdeck_core::engine::{Engine, EngineConfig, EngineHandle, LatencySample};
use quarterdeck_core::{FleetView, FmHome, NoteReceipt};
use serde::Serialize;
use specta::Type;
use tauri_specta::{collect_commands, collect_events, Event};

/// A new fleet view, pushed on every change.
#[derive(Clone, Serialize, Type, Event)]
pub struct FleetUpdated(pub FleetView);

/// Facts the UI needs once at start.
#[derive(Clone, Serialize, Type)]
pub struct AppInfo {
    /// Short label of the Firstmate home, when one was found.
    pub home: Option<String>,
    /// Why no home is shown, when none was found.
    pub home_error: Option<String>,
    /// `light` or `dark` from `QD_THEME`, to pin the theme regardless of the system.
    pub theme: Option<String>,
    pub version: String,
}

struct AppState {
    engine: Result<EngineHandle, String>,
    theme: Option<String>,
    latency_log: Option<PathBuf>,
}

impl AppState {
    fn engine(&self) -> Result<&EngineHandle, String> {
        self.engine.as_ref().map_err(Clone::clone)
    }
}

#[tauri::command]
#[specta::specta]
fn app_info(state: tauri::State<'_, AppState>) -> AppInfo {
    AppInfo {
        home: state.engine.as_ref().ok().map(|e| e.home().label()),
        home_error: state.engine.as_ref().err().cloned(),
        theme: state.theme.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// The current fleet view, or `None` when no home is configured.
#[tauri::command]
#[specta::specta]
fn fleet_view(state: tauri::State<'_, AppState>) -> Option<FleetView> {
    state.engine().ok().map(EngineHandle::current)
}

/// Asks for a fresh snapshot.
#[tauri::command]
#[specta::specta]
fn refresh_fleet(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.engine()?.refresh();
    Ok(())
}

/// Sends one note to Firstmate through `bin/fm-inbox.sh note`.
#[tauri::command]
#[specta::specta]
async fn send_note(
    state: tauri::State<'_, AppState>,
    text: String,
    request_id: String,
) -> Result<NoteReceipt, String> {
    state
        .engine()?
        .submit_note(&text, &request_id)
        .await
        .map_err(|e| e.to_string())
}

/// The UI reports that it painted view `seq`; completes a status-change latency sample.
#[tauri::command]
#[specta::specta]
fn view_rendered(state: tauri::State<'_, AppState>, seq: u32, rendered_ms: f64) {
    let Ok(engine) = state.engine() else { return };
    // Milliseconds since the epoch fit an f64 exactly.
    #[allow(clippy::cast_possible_truncation)]
    let Some(sample) = engine.rendered(seq, rendered_ms as i64) else {
        return;
    };
    log_latency(state.latency_log.as_deref(), &sample);
}

fn log_latency(path: Option<&std::path::Path>, s: &LatencySample) {
    let line = format!(
        "{{\"task\":{task:?},\"seq\":{seq},\"file_mtime_ms\":{m},\"detected_ms\":{d},\"emitted_ms\":{e},\"rendered_ms\":{r},\"total_ms\":{t}}}",
        task = s.task_id,
        seq = s.seq,
        m = s.file_mtime_ms,
        d = s.detected_ms,
        e = s.emitted_ms,
        r = s.rendered_ms,
        t = s.total_ms(),
    );
    eprintln!("quarterdeck latency {line}");
    if let Some(path) = path {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The typed IPC surface, shared by the app and the bindings export.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            app_info,
            fleet_view,
            refresh_fleet,
            send_note,
            view_rendered,
        ])
        .events(collect_events![FleetUpdated])
}

/// Writes the TypeScript bindings the UI imports.
pub fn export_bindings(path: &std::path::Path) -> Result<(), String> {
    specta_builder()
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            path,
        )
        .map_err(|e| e.to_string())
}

fn start_engine() -> Result<EngineHandle, String> {
    let home = FmHome::from_env().map_err(|e| e.to_string())?;
    tauri::async_runtime::block_on(async { Engine::start(home, EngineConfig::default()) })
        .map_err(|e| format!("could not watch the Firstmate home: {e}"))
}

pub fn run() {
    let builder = specta_builder();
    let theme = std::env::var("QD_THEME")
        .ok()
        .filter(|t| t == "light" || t == "dark");
    let latency_log = std::env::var_os("QD_LATENCY_LOG").map(PathBuf::from);

    // The engine starts before the window exists and its state is managed before `run`, so
    // the UI can never call a command before the state it needs is in place.
    let engine = start_engine();
    let views = engine.as_ref().ok().map(EngineHandle::subscribe);

    tauri::Builder::default()
        .manage(AppState {
            engine,
            theme,
            latency_log,
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            if let Some(mut views) = views {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    while views.changed().await.is_ok() {
                        let view = views.borrow_and_update().clone();
                        let _ = FleetUpdated(view).emit(&handle);
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Quarterdeck window");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed bindings must match the Rust signatures. Set `QD_WRITE_BINDINGS=1` to
    /// regenerate `ui/src/bindings.ts`.
    #[test]
    fn bindings_are_current() {
        let committed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts");
        if std::env::var_os("QD_WRITE_BINDINGS").is_some() {
            export_bindings(&committed).unwrap();
        }
        let fresh = std::env::temp_dir().join(format!("qd-bindings-{}.ts", std::process::id()));
        export_bindings(&fresh).unwrap();
        let want = std::fs::read_to_string(&fresh).unwrap();
        let _ = std::fs::remove_file(&fresh);
        let have = std::fs::read_to_string(&committed).unwrap_or_default();
        assert!(have == want, "ui/src/bindings.ts is stale: run QD_WRITE_BINDINGS=1 cargo test -p quarterdeck bindings");
    }
}
