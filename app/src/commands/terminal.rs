//! Terminal commands: the read-only tail of a worker's terminal. Nothing here sends input.

use quarterdeck_core::terminal::{self, TerminalTail};

use crate::state::AppState;

/// The last lines of task `task_id`'s terminal, read through `bin/fm-peek.sh`, sanitized and
/// styled in the core. The id must name a task in the current snapshot; a remote task is
/// refused rather than read over the network.
#[tauri::command]
#[specta::specta]
pub async fn terminal_tail(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<TerminalTail, String> {
    let (home, home_state) = state.with_engine(|e| (e.home().clone(), e.state()))?;
    terminal::read(&home, &home_state, &task_id)
        .await
        .map_err(|e| e.to_string())
}
