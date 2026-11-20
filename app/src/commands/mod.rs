//! The typed IPC surface: one file per area, each command registered in `lib.rs`.
//!
//! To add an area: `pub mod <area>;` here, its commands in `collect_commands![]` and its
//! events in `collect_events![]` in `lib.rs`, one string per command in `build.rs`, one
//! `allow-<command>` per command in `capabilities/default.json`, and, for a snapshot-driven
//! view, one `projection(...)` line in `lib.rs`. The `registry_is_consistent` test keeps the
//! lists in step.

pub mod fleet;
pub mod memory;
pub mod shell;
pub mod skills;

use std::sync::Arc;

use quarterdeck_core::HomeState;
use serde::Serialize;
use tauri::AppHandle;
use tokio::sync::watch;

/// A snapshot-driven area view, attached to every engine the app starts.
pub type Projection = Arc<dyn Fn(&AppHandle, watch::Receiver<Arc<HomeState>>) + Send + Sync>;

/// Registers a pure area projection: on every published [`HomeState`] the app runs `view`,
/// and emits `event(view)` only when the result differs from the last one emitted, so a
/// change elsewhere in the home costs that area nothing on the wire.
pub fn projection<V, E>(view: fn(&HomeState) -> V, event: fn(V) -> E) -> Projection
where
    V: PartialEq + Clone + Send + Sync + 'static,
    E: tauri_specta::Event + Serialize + Clone + 'static,
{
    Arc::new(
        move |app: &AppHandle, rx: watch::Receiver<Arc<HomeState>>| {
            spawn_projection(app.clone(), rx, view, event);
        },
    )
}

/// Runs one projection until its engine stops (the home changed or the app is closing).
pub fn spawn_projection<V, E>(
    app: AppHandle,
    mut rx: watch::Receiver<Arc<HomeState>>,
    view: fn(&HomeState) -> V,
    event: fn(V) -> E,
) where
    V: PartialEq + Clone + Send + Sync + 'static,
    E: tauri_specta::Event + Serialize + Clone + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut last: Option<V> = None;
        loop {
            let state = rx.borrow_and_update().clone();
            let next = view(&state);
            if last.as_ref() != Some(&next) {
                let _ = event(next.clone()).emit(&app);
                last = Some(next);
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn manifest(rel: &str) -> String {
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
    }

    /// The committed bindings must match the Rust signatures. Set `QD_WRITE_BINDINGS=1` to
    /// regenerate `ui/src/bindings.ts`.
    #[test]
    fn bindings_are_current() {
        let committed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts");
        if std::env::var_os("QD_WRITE_BINDINGS").is_some() {
            crate::export_bindings(&committed).unwrap();
        }
        let fresh = std::env::temp_dir().join(format!("qd-bindings-{}.ts", std::process::id()));
        crate::export_bindings(&fresh).unwrap();
        let want = std::fs::read_to_string(&fresh).unwrap();
        let _ = std::fs::remove_file(&fresh);
        let have = std::fs::read_to_string(&committed).unwrap_or_default();
        assert!(have == want, "ui/src/bindings.ts is stale: run QD_WRITE_BINDINGS=1 cargo test -p quarterdeck bindings");
    }

    /// `build.rs`, the registered commands and the window capability name the same commands.
    #[test]
    fn registry_is_consistent() {
        let build = manifest("build.rs");
        let list = &build[build.find("commands(&[").unwrap()..build.find("])").unwrap()];
        let mut declared: Vec<String> = list
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect();
        declared.sort();

        let fresh = std::env::temp_dir().join(format!("qd-registry-{}.ts", std::process::id()));
        crate::export_bindings(&fresh).unwrap();
        let bindings = std::fs::read_to_string(&fresh).unwrap();
        let _ = std::fs::remove_file(&fresh);
        let mut registered: Vec<String> = bindings
            .split("__TAURI_INVOKE<")
            .skip(1)
            .filter_map(|s| s.split("(\"").nth(1)?.split('"').next().map(str::to_string))
            .collect();
        registered.sort();
        assert_eq!(
            declared, registered,
            "build.rs and collect_commands! disagree"
        );

        let cap: serde_json::Value =
            serde_json::from_str(&manifest("capabilities/default.json")).unwrap();
        let mut granted: Vec<String> = cap["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| {
                p.as_str()?
                    .strip_prefix("allow-")
                    .map(|c| c.replace('-', "_"))
            })
            .collect();
        granted.sort();
        assert_eq!(
            declared, granted,
            "capabilities/default.json and build.rs disagree"
        );
        for p in cap["permissions"].as_array().unwrap() {
            let p = p.as_str().unwrap();
            assert!(
                p.starts_with("allow-") || p.starts_with("core:event:") || p == "core:window:allow-start-dragging",
                "unexpected permission {p}: the window gets no plugin, shell, fs, http or opener permission"
            );
        }
    }
}
