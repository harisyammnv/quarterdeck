//! Fleet commands: the live fleet view, refresh, the inbox note and the latency probe.

use std::io::Write;

use quarterdeck_core::engine::LatencySample;
use quarterdeck_core::{FleetView, NoteReceipt};
use serde::Serialize;
use specta::Type;

use crate::state::AppState;

/// A new fleet view, pushed on every change.
#[derive(Clone, Serialize, Type, tauri_specta::Event)]
pub struct FleetUpdated(pub FleetView);

/// The current fleet view, or `None` when no home is configured.
#[tauri::command]
#[specta::specta]
pub fn fleet_view(state: tauri::State<'_, AppState>) -> Option<FleetView> {
    state.with_engine(|e| e.current()).ok()
}

/// Asks for a fresh snapshot.
#[tauri::command]
#[specta::specta]
pub fn refresh_fleet(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.with_engine(|e| e.refresh())
}

/// Sends one note to Firstmate through `bin/fm-inbox.sh note`.
#[tauri::command]
#[specta::specta]
pub async fn send_note(
    state: tauri::State<'_, AppState>,
    text: String,
    request_id: String,
) -> Result<NoteReceipt, String> {
    let home = state.with_engine(|e| e.home().clone())?;
    quarterdeck_core::inbox::submit(&home, &text, &request_id)
        .await
        .map_err(|e| e.to_string())
}

/// The UI reports that it painted view `seq`; completes a status-change latency sample.
#[tauri::command]
#[specta::specta]
pub fn view_rendered(state: tauri::State<'_, AppState>, seq: u32, rendered_ms: f64) {
    // Milliseconds since the epoch fit an f64 exactly.
    #[allow(clippy::cast_possible_truncation)]
    let Ok(Some(sample)) = state.with_engine(|e| e.rendered(seq, rendered_ms as i64)) else {
        return;
    };
    log_latency(state.latency_log(), &sample);
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
