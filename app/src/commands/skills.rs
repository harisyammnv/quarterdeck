//! Skills commands: list and read the installed skills, read-only.

use std::path::PathBuf;

use quarterdeck_core::skills::{SkillDoc, SkillIndex, SkillSources, SkillsView};

use crate::commands::memory::{sources, MemoryState};
use crate::state::AppState;

/// The user's home directory, when it is an absolute path.
fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

async fn index(state: &AppState, mem: &MemoryState, fresh: bool) -> Result<SkillIndex, String> {
    let (root, projects) = sources(state)?;
    let mut cached = mem.skills.lock().await;
    if !fresh {
        if let Some((home, idx)) = cached.as_ref() {
            if *home == root {
                return Ok(idx.clone());
            }
        }
    }
    let idx = SkillIndex::scan(&SkillSources {
        firstmate: Some(root.clone()),
        user_home: user_home(),
        projects,
    })
    .await;
    *cached = Some((root, idx.clone()));
    Ok(idx)
}

/// Lists every skill in Firstmate's, the user's and the projects' skill folders. Rescans on
/// every call.
#[tauri::command]
#[specta::specta]
pub async fn skills_list(
    state: tauri::State<'_, AppState>,
    mem: tauri::State<'_, MemoryState>,
) -> Result<SkillsView, String> {
    Ok(index(&state, &mem, true).await?.view().clone())
}

/// Reads one skill's body by the id `skills_list` gave it (at most 1 MiB).
#[tauri::command]
#[specta::specta]
pub async fn skill_read(
    state: tauri::State<'_, AppState>,
    mem: tauri::State<'_, MemoryState>,
    id: String,
) -> Result<SkillDoc, String> {
    index(&state, &mem, false)
        .await?
        .read(&id)
        .await
        .map_err(|e| e.to_string())
}
