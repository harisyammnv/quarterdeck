fn main() {
    // Declaring the app's commands generates one `allow-<command>` permission per command,
    // so capabilities/default.json grants each one individually. One string per command;
    // `commands::tests::registry_is_consistent` checks this list against the registered
    // commands and the capability.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_info",
            "board_view",
            "fleet_view",
            "home_pick",
            "memory_list",
            "memory_read",
            "memory_search",
            "orca_focus",
            "project_icon",
            "projects_view",
            "refresh_fleet",
            "report_history",
            "report_read",
            "reports_list",
            "send_note",
            "skill_read",
            "skills_list",
            "task_pr_open",
            "terminal_tail",
            "url_open",
            "view_rendered",
        ]),
    ))
    .expect("failed to run tauri-build");
}
