//! End-to-end checks of the engine against a fake Firstmate home with stub scripts.
//!
//! Set `QD_TEST_TMPDIR` to run the fake home on a specific volume, for example the one the
//! real home lives on, since file-event latency depends on the file system.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use quarterdeck_core::engine::{Engine, EngineConfig};
use quarterdeck_core::{FleetView, FmHome, NoteOutcome, StateSource, TaskState};

const FIXTURE: &str = include_str!("fixtures/bearings.json");

fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_home() -> tempfile::TempDir {
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
        &root.join("bin/fm-bearings-snapshot.sh"),
        r#"[ "$1" = "--json" ] || exit 9; cat "$FM_HOME/snapshot.json""#,
    );
    script(
        &root.join("bin/fm-inbox.sh"),
        r#"printf '%s\n' "$@" > "$FM_HOME/inbox-args"
printf '%s' "${QD_LEAK_PROBE:-absent}" > "$FM_HOME/inbox-env"
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

async fn wait_for(
    rx: &mut tokio::sync::watch::Receiver<FleetView>,
    limit: Duration,
    pred: impl Fn(&FleetView) -> bool,
) -> Option<FleetView> {
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

fn append(path: &Path, line: &str) {
    let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(f, "{line}").unwrap();
}

fn epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstraps_then_reconciles_with_snapshot() {
    let dir = fake_home();
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    let mut rx = handle.subscribe();
    let v = wait_for(&mut rx, Duration::from_secs(10), |v| {
        v.snapshot.generated_at.is_some()
    })
    .await
    .expect("snapshot applied");
    assert_eq!(v.rows.len(), 6);
    assert!(!v.snapshot.running);
    assert_eq!(v.snapshot.error, None);
    let row = v.rows.iter().find(|r| r.id == "app-login-fix").unwrap();
    // The snapshot could not tell this task's state, so the record's latest event stands in.
    assert_eq!(
        (row.state, row.source),
        (TaskState::Working, StateSource::Event)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn status_change_reaches_the_view_within_a_second() {
    let dir = fake_home();
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    let mut rx = handle.subscribe();
    wait_for(&mut rx, Duration::from_secs(10), |v| {
        v.snapshot.generated_at.is_some()
    })
    .await
    .expect("snapshot applied");
    // Let FSEvents settle after the setup writes.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let status = dir.path().join("state/app-login-fix.status");
    let mut worst = Duration::ZERO;
    for (i, word) in ["blocked", "working", "needs-decision", "done", "working"]
        .iter()
        .enumerate()
    {
        let started = Instant::now();
        append(&status, &format!("{word} [at={}]: probe {i}", epoch()));
        let want = TaskState::from_word(word);
        let v = wait_for(&mut rx, Duration::from_secs(1), |v| {
            v.rows.iter().any(|r| {
                r.id == "app-login-fix"
                    && r.state == want
                    && r.latest_event
                        .as_ref()
                        .is_some_and(|e| e.text == format!("probe {i}"))
            })
        })
        .await
        .unwrap_or_else(|| panic!("{word} did not appear within 1 s"));
        let took = started.elapsed();
        worst = worst.max(took);
        let cause = v.cause.expect("status change carries a cause");
        assert_eq!(cause.task_id, "app-login-fix");
        eprintln!(
            "probe {i} {word}: {} ms (file to core {} ms)",
            took.as_millis(),
            cause.detected_ms - cause.file_mtime_ms
        );
    }
    eprintln!("worst status-change latency: {} ms", worst.as_millis());
    assert!(worst < Duration::from_secs(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_failure_is_reported_not_fatal() {
    let dir = fake_home();
    script(
        &dir.path().join("bin/fm-bearings-snapshot.sh"),
        "echo 'lock busy' >&2; exit 4",
    );
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    let mut rx = handle.subscribe();
    let v = wait_for(&mut rx, Duration::from_secs(10), |v| {
        v.snapshot.error.is_some()
    })
    .await
    .expect("error surfaced");
    assert!(v.snapshot.error.unwrap().contains("lock busy"));
    // Bootstrap rows from the records are still shown.
    assert_eq!(v.rows.len(), 1);
    assert_eq!(v.rows[0].source, StateSource::Bootstrap);
}

#[tokio::test(flavor = "multi_thread")]
async fn note_goes_through_the_inbox_script_as_argv() {
    let dir = fake_home();
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    // A variable from the launching shell must not reach Firstmate's scripts.
    std::env::set_var("QD_LEAK_PROBE", "leaked");
    let text = "--all; rm -rf / $(whoami) is just text";
    let receipt = handle.submit_note(text, "qd-test-1").await.unwrap();
    assert_eq!(receipt.outcome, NoteOutcome::Created);
    assert_eq!(receipt.id, "n-7");
    assert!(receipt.announced);
    let args = std::fs::read_to_string(dir.path().join("inbox-args")).unwrap();
    let env = std::fs::read_to_string(dir.path().join("inbox-env")).unwrap();
    assert_eq!(env, "absent");
    assert_eq!(
        args,
        format!("note\n--request-id\nqd-test-1\n--json\n--\n{text}\n")
    );
}
