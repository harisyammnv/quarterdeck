//! Projects commands: the sidebar view, project icons and "Open in Orca".

use quarterdeck_core::external::{self, External};
use quarterdeck_core::projects::{self, IconCache, ProjectsView};
use serde::Serialize;
use specta::Type;

use crate::state::AppState;

/// A new projects view, pushed only when it differs from the last one.
#[derive(Clone, Serialize, Type, tauri_specta::Event)]
pub struct ProjectsUpdated(pub ProjectsView);

/// The area's own state: resolved project icons.
#[derive(Default)]
pub struct ProjectsState {
    icons: IconCache,
}

/// The current projects view, or `None` when no home is configured.
#[tauri::command]
#[specta::specta]
pub fn projects_view(state: tauri::State<'_, AppState>) -> Option<ProjectsView> {
    state.with_engine(|e| projects::view(&e.state())).ok()
}

/// A project's icon as a size-capped `data:` URL, or `None` for a monogram. The window sends
/// the project's name; the core finds its clone or worktrees in the snapshot.
#[tauri::command]
#[specta::specta]
pub async fn project_icon(
    state: tauri::State<'_, AppState>,
    projects: tauri::State<'_, ProjectsState>,
    name: String,
) -> Result<Option<String>, String> {
    let home = state.with_engine(|e| e.state())?;
    projects
        .icons
        .for_project(&home, &name)
        .await
        .map_err(|e| e.to_string())
}

/// Brings a task's terminal (or, without one, its worktree) forward in Orca. The window sends
/// the task id; the handle or path comes from the snapshot.
#[tauri::command]
#[specta::specta]
pub async fn projects_focus_orca(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    let home = state.with_engine(|e| e.state())?;
    let target = projects::orca::resolve(&home, &task_id).map_err(|e| e.to_string())?;
    external::run(&External::OrcaFocus(target))
        .await
        .map_err(|e| e.to_string())
}
