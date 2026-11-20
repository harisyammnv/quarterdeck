//! Shared helpers for the core's end-to-end tests: a fake Firstmate home with stub scripts.
//!
//! Each test file includes this with `mod common;`. Set `QD_TEST_TMPDIR` to run the fake
//! home on a specific volume, since file-event latency depends on the file system.
#![allow(dead_code)]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use quarterdeck_core::FleetView;

pub const FIXTURE: &str = include_str!("../fixtures/fleet-snapshot.json");

/// Writes an executable `#!/bin/sh` script.
pub fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A temporary home with `state/`, `bin/` and `data/`, a snapshot stub that prints
/// [`FIXTURE`], an inbox stub that records its argv, environment and PATH, and one task's
/// meta and status records.
pub fn fake_home() -> tempfile::TempDir {
    let dir = match std::env::var_os("QD_TEST_TMPDIR") {
        Some(base) => tempfile::tempdir_in(base).unwrap(),
        None => tempfile::tempdir().unwrap(),
    };
    let root = dir.path();
    for d in ["state", "bin", "data"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("snapshot.json"), FIXTURE).unwrap();
    script(
        &root.join("bin/fm-fleet-snapshot.sh"),
        r#"[ "$1" = "--json" ] && [ "$#" = 1 ] || exit 9; cat "$FM_HOME/snapshot.json""#,
    );
    script(
        &root.join("bin/fm-inbox.sh"),
        r#"printf '%s\n' "$@" > "$FM_HOME/inbox-args"
printf '%s' "${CARGO_MANIFEST_DIR:-absent}" > "$FM_HOME/inbox-env"
printf '%s' "$PATH" > "$FM_HOME/inbox-path"
printf '{"schema":"fm-inbox-note.v1","outcome":"created","id":"n-7","request_id":"%s","saved":true,"announced":true,"acknowledged":false,"path":"x"}\n' "$3""#,
    );
    std::fs::write(
        root.join("state/app-login-fix.meta"),
        "kind=ship\nproject=/p/example-app\n",
    )
    .unwrap();
    std::fs::write(
        root.join("state/app-login-fix.status"),
        "working [at=1795163364]: started\n",
    )
    .unwrap();
    dir
}

/// Waits until a published value satisfies `pred`, or `limit` passes.
pub async fn wait_for<T: Clone>(
    rx: &mut tokio::sync::watch::Receiver<T>,
    limit: Duration,
    pred: impl Fn(&T) -> bool,
) -> Option<T> {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        {
            let v = rx.borrow_and_update();
            if pred(&v) {
                return Some(v.clone());
            }
        }
        if tokio::time::timeout_at(deadline, rx.changed())
            .await
            .is_err()
        {
            return None;
        }
    }
}

/// Waits for a fleet view (kept for the latency tests' readability).
pub async fn wait_view(
    rx: &mut tokio::sync::watch::Receiver<FleetView>,
    limit: Duration,
    pred: impl Fn(&FleetView) -> bool,
) -> Option<FleetView> {
    wait_for(rx, limit, pred).await
}

/// Appends one line to a record.
pub fn append(path: &Path, line: &str) {
    let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(f, "{line}").unwrap();
}

/// Now, in Unix seconds.
pub fn epoch() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}
