fn main() {
    // Declaring the app's commands generates one `allow-<command>` permission per command,
    // so capabilities/default.json grants each one individually.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_info",
            "fleet_view",
            "refresh_fleet",
            "send_note",
            "view_rendered",
        ]),
    ))
    .expect("failed to run tauri-build");
}
