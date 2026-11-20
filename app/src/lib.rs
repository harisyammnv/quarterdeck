//! The Quarterdeck window: a thin Tauri shell over `quarterdeck-core`.
//!
//! Every command is listed in `capabilities/default.json`; the window has no shell, file
//! system, dialog or network permission of its own. The folder picker for the home runs from
//! Rust, so the webview never supplies a path.
//!
//! Areas register here, one line per item, in alphabetical order by area: commands in
//! `collect_commands![]`, events in `collect_events![]`, and snapshot-driven views in
//! `projections()`. See `commands/mod.rs`.

pub mod commands;
mod state;

use commands::{board, fleet, reports, shell, terminal, Projection};
use tauri_specta::{collect_commands, collect_events};

/// The typed IPC surface, shared by the app and the bindings export.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            board::board_view,
            fleet::fleet_view,
            fleet::refresh_fleet,
            fleet::send_note,
            fleet::view_rendered,
            reports::report_history,
            reports::report_read,
            reports::reports_list,
            shell::app_info,
            shell::home_pick,
            shell::task_pr_open,
            shell::url_open,
            terminal::terminal_tail,
        ])
        .events(collect_events![
            // one line per event
            board::BoardUpdated,
            fleet::FleetUpdated,
            shell::HomeChanged,
        ])
}

/// Snapshot-driven area views, one line per area:
/// `commands::projection(quarterdeck_core::<area>::view, <area>::<Area>Updated),`.
fn projections() -> Vec<Projection> {
    vec![commands::projection(
        quarterdeck_core::board::view,
        board::BoardUpdated,
    )]
}

/// Writes the TypeScript bindings the UI imports.
pub fn export_bindings(path: &std::path::Path) -> Result<(), String> {
    specta_builder()
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            path,
        )
        .map_err(|e| e.to_string())
}

pub fn run() {
    let builder = specta_builder();

    // The state (and the engine for the home, when one is known) exists before the window,
    // so the UI can never call a command before the state it needs is in place.
    let state = state::AppState::new(projections());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let state = tauri::Manager::state::<state::AppState>(app);
            state.forward_views(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Quarterdeck window");
}
