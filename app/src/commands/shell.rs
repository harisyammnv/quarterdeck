//! Shell commands: app facts, choosing the Firstmate home, and opening links.

use quarterdeck_core::external::{self, External};
use quarterdeck_core::gateway::valid_task_id;
use quarterdeck_core::{AppConfig, FmHome, HomeSource};
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::state::AppState;

/// Facts the UI needs at start and after the home changes.
#[derive(Clone, Serialize, Type)]
pub struct AppInfo {
    /// Short label of the Firstmate home, when one was found.
    pub home: Option<String>,
    /// Where the home came from, when one was found.
    pub home_source: Option<HomeSource>,
    /// Why no home is shown, when none was found.
    pub home_error: Option<String>,
    /// `light` or `dark` from `QD_THEME`, to pin the theme regardless of the system.
    pub theme: Option<String>,
    pub version: String,
}

/// The home changed: a new home was chosen in the window.
#[derive(Clone, Serialize, Type, tauri_specta::Event)]
pub struct HomeChanged(pub AppInfo);

#[tauri::command]
#[specta::specta]
pub fn app_info(state: tauri::State<'_, AppState>) -> AppInfo {
    state.info()
}

/// Opens the system folder picker, and when a valid Firstmate home is picked, remembers it
/// in the app's own configuration and switches the window to it.
///
/// The path never comes from the webview: the picker runs here, in the core process.
/// Cancelling the picker changes nothing and returns the current facts.
#[tauri::command]
#[specta::specta]
pub async fn home_pick(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<AppInfo, String> {
    use tauri_plugin_dialog::DialogExt;
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Choose the Firstmate home folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| format!("the folder picker failed: {e}"))?;
    let Some(picked) = picked else {
        return Ok(state.info());
    };
    let root = picked
        .into_path()
        .map_err(|e| format!("the picked folder is not a local path: {e}"))?;
    let home = FmHome::at(root).map_err(|e| sentence(&e.to_string()))?;
    let config_dir = state.config_dir();
    let mut config = AppConfig::load(&config_dir).unwrap_or_default();
    config.home = Some(home.root().to_path_buf());
    config.save(&config_dir).map_err(|e| e.to_string())?;
    state.switch_home(&app, home, HomeSource::Chosen)?;
    let info = state.info();
    let _ = HomeChanged(info.clone()).emit(&app);
    Ok(info)
}

/// Opens an `https` link in the default browser. Used only for links inside rendered
/// documents, after the window has shown the full address and the person confirmed it.
#[tauri::command]
#[specta::specta]
pub async fn url_open(url: String) -> Result<(), String> {
    external::run(&External::OpenUrl(url))
        .await
        .map_err(|e| e.to_string())
}

/// Opens a task's recorded pull request in the default browser. The address comes from the
/// fleet snapshot (the task row, else its backlog record), never from the window.
#[tauri::command]
#[specta::specta]
pub async fn task_pr_open(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    if !valid_task_id(&task_id) {
        return Err("invalid task id".into());
    }
    let snapshot = state
        .with_engine(|e| e.state().snapshot.clone())?
        .ok_or("the fleet snapshot has not been read yet")?;
    let url = snapshot
        .task(&task_id)
        .and_then(|t| t.pr_url.clone())
        .or_else(|| snapshot.record(&task_id).and_then(|r| r.pr_url.clone()))
        .ok_or_else(|| format!("no pull request is recorded for {task_id}"))?;
    external::run(&External::OpenUrl(url))
        .await
        .map_err(|e| e.to_string())
}

fn sentence(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
