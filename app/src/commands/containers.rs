//! Containers commands: a read-only view of the local Docker engine (design 5.8).
//!
//! Lists, logs and stats only; nothing here starts, stops, removes or execs a container.

use quarterdeck_core::containers::{
    self, ContainerDetail, ContainerLogs, ContainerStats, Containers, ContainersView, EngineSummary,
};
use quarterdeck_core::external::{self, External};

use crate::state::AppState;

/// The containers reader, with the ids of its last listing.
pub struct ContainersState(Containers);

impl Default for ContainersState {
    fn default() -> Self {
        Self(Containers::from_env())
    }
}

/// Lists containers, images, volumes and networks of the active Docker context, with
/// containers grouped under the worktrees of the tasks in the fleet snapshot.
#[tauri::command]
#[specta::specta]
pub async fn containers_list(
    app: tauri::State<'_, AppState>,
    state: tauri::State<'_, ContainersState>,
) -> Result<ContainersView, String> {
    // Without a home there are no worktrees to group by; containers still list.
    let worktrees = app
        .with_engine(|e| containers::worktrees(&e.state()))
        .unwrap_or_default();
    Ok(state.0.list(&worktrees).await)
}

/// The Docker context and engine state for the status bar: discovery and `/version` only.
#[tauri::command]
#[specta::specta]
pub async fn containers_status(
    state: tauri::State<'_, ContainersState>,
) -> Result<EngineSummary, String> {
    Ok(state.0.summary().await)
}

/// Start and finish times and exit code of one container from the last listing, by short
/// id. The container is inspected alone; nothing else from the inspect response is sent.
#[tauri::command]
#[specta::specta]
pub async fn container_inspect(
    state: tauri::State<'_, ContainersState>,
    id: String,
) -> Result<ContainerDetail, String> {
    state.0.detail(&id).await.map_err(|e| e.to_string())
}

/// The last lines of output of a container from the last listing, by short id.
#[tauri::command]
#[specta::specta]
pub async fn container_logs(
    state: tauri::State<'_, ContainersState>,
    id: String,
) -> Result<ContainerLogs, String> {
    state.0.logs(&id).await.map_err(|e| e.to_string())
}

/// One CPU and memory sample of a container from the last listing, by short id.
#[tauri::command]
#[specta::specta]
pub async fn container_stats(
    state: tauri::State<'_, ContainersState>,
    id: String,
) -> Result<ContainerStats, String> {
    state.0.stats(&id).await.map_err(|e| e.to_string())
}

/// Opens Kiri's own window. Takes no argument: the launch is fixed.
#[tauri::command]
#[specta::specta]
pub async fn kiri_open() -> Result<(), String> {
    external::run(&External::KiriUi)
        .await
        .map_err(|e| e.to_string())
}
