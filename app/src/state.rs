//! The app's managed state: the running engine for the current home, which can be swapped
//! at runtime when a new home is chosen.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use quarterdeck_core::engine::{Engine, EngineConfig, EngineHandle};
use quarterdeck_core::{AppConfig, FmHome, HomeSource};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::commands::fleet::FleetUpdated;
use crate::commands::shell::AppInfo;
use crate::commands::Projection;

/// Must equal `identifier` in `tauri.conf.json` (checked by a test).
pub const APP_IDENTIFIER: &str = "dev.quarterdeck.desktop";

struct Active {
    engine: Result<EngineHandle, String>,
    source: Option<HomeSource>,
}

pub struct AppState {
    active: RwLock<Active>,
    projections: Vec<Projection>,
    theme: Option<String>,
    latency_log: Option<PathBuf>,
    config_dir: PathBuf,
}

/// The app's own configuration directory, the same one Tauri's `app_config_dir` names:
/// `~/Library/Application Support/<identifier>` on macOS, `$XDG_CONFIG_HOME/<identifier>`
/// elsewhere. `QD_CONFIG_DIR` overrides it for development and tests.
pub fn config_dir_from(lookup: impl Fn(&str) -> Option<std::ffi::OsString>) -> PathBuf {
    if let Some(dir) = lookup("QD_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    let home = lookup("HOME").map(PathBuf::from).unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else {
        lookup("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map_or_else(|| home.join(".config"), PathBuf::from)
    };
    base.join(APP_IDENTIFIER)
}

fn engine_config(config_dir: &Path) -> EngineConfig {
    EngineConfig {
        cache_dir: Some(config_dir.join("cache")),
        ..EngineConfig::default()
    }
}

fn start(home: FmHome, config_dir: &Path) -> Result<EngineHandle, String> {
    let config = engine_config(config_dir);
    tauri::async_runtime::block_on(async { Engine::start(home, config) })
        .map_err(|e| format!("could not watch the Firstmate home: {e}"))
}

impl AppState {
    /// Resolves the home from the environment or the saved choice and starts its engine.
    /// `projections` are the snapshot-driven area views, attached to every engine.
    pub fn new(projections: Vec<Projection>) -> Self {
        let config_dir = config_dir_from(|k| std::env::var_os(k));
        let config = AppConfig::load(&config_dir);
        let (engine, source) = match config {
            Err(e) => (Err(e.to_string()), None),
            Ok(config) => {
                match quarterdeck_core::config::resolve_home(|k| std::env::var_os(k), &config) {
                    Ok((home, source)) => (start(home, &config_dir), Some(source)),
                    Err(e) => (Err(e.to_string()), None),
                }
            }
        };
        Self {
            active: RwLock::new(Active { engine, source }),
            projections,
            theme: std::env::var("QD_THEME")
                .ok()
                .filter(|t| t == "light" || t == "dark"),
            latency_log: std::env::var_os("QD_LATENCY_LOG").map(PathBuf::from),
            config_dir,
        }
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    pub fn latency_log(&self) -> Option<&Path> {
        self.latency_log.as_deref()
    }

    /// Runs `f` with the current engine, or returns why there is none.
    pub fn with_engine<T>(&self, f: impl FnOnce(&EngineHandle) -> T) -> Result<T, String> {
        let active = self
            .active
            .read()
            .map_err(|_| "app state is unavailable".to_string())?;
        active.engine.as_ref().map(f).map_err(Clone::clone)
    }

    pub fn info(&self) -> AppInfo {
        let (home, home_error, home_source) = match self.active.read() {
            Ok(a) => (
                a.engine.as_ref().ok().map(|e| e.home().label()),
                a.engine.as_ref().err().cloned(),
                a.source,
            ),
            Err(_) => (None, Some("app state is unavailable".into()), None),
        };
        AppInfo {
            home,
            home_source,
            home_error,
            theme: self.theme.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Starts forwarding the current engine's fleet views and area projections to the window.
    pub fn forward_views(&self, app: &AppHandle) {
        let Ok((mut views, states)) = self.with_engine(|e| (e.subscribe(), e.subscribe_state()))
        else {
            return;
        };
        for p in &self.projections {
            p(app, states.clone());
        }
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            // Ends when the engine stops, which happens when its handle is replaced.
            while views.changed().await.is_ok() {
                let view = views.borrow_and_update().clone();
                let _ = FleetUpdated(view).emit(&handle);
            }
        });
    }

    /// Replaces the running engine with one for `home`. The old engine stops when its handle
    /// is dropped here, which also ends its view forwarding.
    pub fn switch_home(
        &self,
        app: &AppHandle,
        home: FmHome,
        source: HomeSource,
    ) -> Result<(), String> {
        let engine = Engine::start(home, engine_config(&self.config_dir))
            .map_err(|e| format!("could not watch the Firstmate home: {e}"))?;
        {
            let mut active = self
                .active
                .write()
                .map_err(|_| "app state is unavailable".to_string())?;
            *active = Active {
                engine: Ok(engine),
                source: Some(source),
            };
        }
        self.forward_views(app);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_matches_tauri_conf() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], APP_IDENTIFIER);
    }

    #[test]
    fn config_dir_override_and_platform_default() {
        let dir = config_dir_from(|k| (k == "QD_CONFIG_DIR").then(|| "/tmp/qd".into()));
        assert_eq!(dir, PathBuf::from("/tmp/qd"));
        let dir = config_dir_from(|k| (k == "HOME").then(|| "/p/user".into()));
        assert!(dir.ends_with(APP_IDENTIFIER));
        assert!(dir.starts_with("/p/user"));
    }
}
