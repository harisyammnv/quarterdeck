//! Report commands: a task's `report.md` and its status history, read on demand.

use quarterdeck_core::reports::{self, ReportDoc, ReportHistory, ReportsList};

use crate::state::AppState;

/// Task `task_id`'s report, or `None` when it has not written one. The Markdown is
/// untrusted; the UI renders it through its sanitizing Markdown component.
#[tauri::command]
#[specta::specta]
pub async fn report_read(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<Option<ReportDoc>, String> {
    let (home, home_state) = state.with_engine(|e| (e.home().clone(), e.state()))?;
    reports::read(&home, &home_state, &task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Task `task_id`'s recent status events, newest first.
#[tauri::command]
#[specta::specta]
pub async fn report_history(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<ReportHistory, String> {
    let (home, home_state) = state.with_engine(|e| (e.home().clone(), e.state()))?;
    reports::history(&home, &home_state, &task_id)
        .await
        .map_err(|e| e.to_string())
}

/// The reports the fleet snapshot lists, newest first and bounded: task ids, titles, kinds and
/// times only, never paths. Open one with `report_read`.
#[tauri::command]
#[specta::specta]
pub async fn reports_list(state: tauri::State<'_, AppState>) -> Result<ReportsList, String> {
    let (home, home_state) = state.with_engine(|e| (e.home().clone(), e.state()))?;
    Ok(reports::list(&home, &home_state).await)
}
