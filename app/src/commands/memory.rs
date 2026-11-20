//! Memory commands: list, read and search Firstmate's memory documents, read-only.
//!
//! The UI sends ids, never paths. Each list call rescans and keeps the index, so a read or a
//! search resolves ids against exactly what the list showed, for the home that is current.

use std::path::{Path, PathBuf};

use quarterdeck_core::memory::{
    self, search, MemoryDoc, MemoryIndex, MemorySearch, MemoryView, ProjectRoot,
};
use quarterdeck_core::skills::SkillIndex;
use tokio::sync::Mutex;

use crate::state::AppState;

/// Matching lines returned by one search.
const SEARCH_LIMIT: usize = 100;

/// The last memory and skills scans, each tagged with the home it was made for.
#[derive(Default)]
pub struct MemoryState {
    memory: Mutex<Option<(PathBuf, MemoryIndex)>>,
    pub(crate) skills: Mutex<Option<(PathBuf, SkillIndex)>>,
}

/// The current home's root and the project checkouts from its latest snapshot.
pub(crate) fn sources(state: &AppState) -> Result<(PathBuf, Vec<ProjectRoot>), String> {
    state.with_engine(|e| {
        (
            e.home().root().to_path_buf(),
            memory::project_roots(&e.state()),
        )
    })
}

async fn scan(root: &Path, projects: Vec<ProjectRoot>) -> Result<MemoryIndex, String> {
    let data = root.join("data");
    tokio::task::spawn_blocking(move || MemoryIndex::scan(&data, &projects))
        .await
        .map_err(|e| format!("the memory scan stopped: {e}"))
}

/// The index for the current home: a fresh scan when `fresh` or when none matches the home.
async fn index(state: &AppState, mem: &MemoryState, fresh: bool) -> Result<MemoryIndex, String> {
    let (root, projects) = sources(state)?;
    let mut cached = mem.memory.lock().await;
    if !fresh {
        if let Some((home, idx)) = cached.as_ref() {
            if *home == root {
                return Ok(idx.clone());
            }
        }
    }
    let idx = scan(&root, projects).await?;
    *cached = Some((root, idx.clone()));
    Ok(idx)
}

/// Lists the memory documents: preferences, shared preferences, learnings, project guides and
/// briefs. Rescans the home on every call.
#[tauri::command]
#[specta::specta]
pub async fn memory_list(
    state: tauri::State<'_, AppState>,
    mem: tauri::State<'_, MemoryState>,
) -> Result<MemoryView, String> {
    Ok(index(&state, &mem, true).await?.view().clone())
}

/// Reads one memory document by the id `memory_list` gave it (at most 1 MiB).
#[tauri::command]
#[specta::specta]
pub async fn memory_read(
    state: tauri::State<'_, AppState>,
    mem: tauri::State<'_, MemoryState>,
    id: String,
) -> Result<MemoryDoc, String> {
    index(&state, &mem, false)
        .await?
        .read(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Finds lines containing `query` across every memory document, case-insensitively.
#[tauri::command]
#[specta::specta]
pub async fn memory_search(
    state: tauri::State<'_, AppState>,
    mem: tauri::State<'_, MemoryState>,
    query: String,
) -> Result<MemorySearch, String> {
    let idx = index(&state, &mem, false).await?;
    Ok(search::search(&idx, &query, SEARCH_LIMIT).await)
}
