//! End-to-end checks of the engine against a fake Firstmate home with stub scripts.
//!
//! Set `QD_TEST_TMPDIR` to run the fake home on a specific volume, for example the one the
//! real home lives on, since file-event latency depends on the file system.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{append, epoch, fake_home, script, wait_for};
use quarterdeck_core::engine::{Engine, EngineConfig};
use quarterdeck_core::{FmHome, HomeState, NoteOutcome, StateSource, TaskState};

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
        &dir.path().join("bin/fm-fleet-snapshot.sh"),
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
    // Cargo sets CARGO_MANIFEST_DIR for this test; as a variable from the launching
    // environment it must not reach Firstmate's scripts.
    assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some());
    let text = "--all; rm -rf / $(whoami) is just text";
    let receipt = handle.submit_note(text, "qd-test-1").await.unwrap();
    assert_eq!(receipt.outcome, NoteOutcome::Created);
    assert_eq!(receipt.id, "n-7");
    assert!(receipt.announced);
    let args = std::fs::read_to_string(dir.path().join("inbox-args")).unwrap();
    let env = std::fs::read_to_string(dir.path().join("inbox-env")).unwrap();
    assert_eq!(env, "absent");
    let path = std::fs::read_to_string(dir.path().join("inbox-path")).unwrap();
    eprintln!("script PATH: {path}");
    assert!(path.split(':').any(|d| d == "/usr/bin"));
    assert_eq!(
        args,
        format!("note\n--request-id\nqd-test-1\n--json\n--\n{text}\n")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_handle_stops_the_engine() {
    let dir = fake_home();
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    let mut rx = handle.subscribe();
    wait_for(&mut rx, Duration::from_secs(10), |v| {
        v.snapshot.generated_at.is_some()
    })
    .await
    .expect("snapshot applied");
    drop(handle);
    // The loop exits and drops its sender, so the stream ends instead of idling forever.
    let ended = tokio::time::timeout(Duration::from_secs(5), async {
        while rx.changed().await.is_ok() {}
    })
    .await;
    assert!(
        ended.is_ok(),
        "the engine kept running after its handle was dropped"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn home_state_carries_the_snapshot_and_events() {
    let dir = fake_home();
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), EngineConfig::default()).unwrap();
    let mut rx = handle.subscribe_state();
    let s: Arc<HomeState> = wait_for(&mut rx, Duration::from_secs(10), |s: &Arc<HomeState>| {
        s.snapshot.is_some()
    })
    .await
    .expect("snapshot published in the home state");
    let snap = s.snapshot.as_ref().unwrap();
    assert_eq!(snap.actionable_count(), 1);
    assert!(s.events.contains_key("app-login-fix"));
    assert_eq!(s.info.generated_at, snap.generated);
    assert_eq!(handle.state().seq, handle.current().seq);
}

#[tokio::test(flavor = "multi_thread")]
async fn second_start_paints_from_the_disk_cache() {
    let dir = fake_home();
    let cache = tempfile::tempdir().unwrap();
    let config = EngineConfig {
        cache_dir: Some(cache.path().to_path_buf()),
        ..EngineConfig::default()
    };
    {
        let handle = Engine::start(FmHome::at(dir.path()).unwrap(), config.clone()).unwrap();
        let mut rx = handle.subscribe();
        wait_for(&mut rx, Duration::from_secs(10), |v| {
            v.snapshot.generated_at.is_some() && !v.snapshot.cached
        })
        .await
        .expect("first run");
        // The cache write runs in the background; wait for the file.
        let deadline = Instant::now() + Duration::from_secs(5);
        let written = || {
            std::fs::read_dir(cache.path())
                .unwrap()
                .filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().ends_with(".json"))
        };
        while !written() {
            assert!(Instant::now() < deadline, "cache was never written");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    // Make the next run slow, so the first view can only come from the cache.
    common::script(
        &dir.path().join("bin/fm-fleet-snapshot.sh"),
        r#"sleep 3; cat "$FM_HOME/snapshot.json""#,
    );
    let handle = Engine::start(FmHome::at(dir.path()).unwrap(), config).unwrap();
    let mut rx = handle.subscribe();
    let v = wait_for(&mut rx, Duration::from_secs(2), |v| {
        v.snapshot.cached && v.rows.len() == 6
    })
    .await
    .expect("cached snapshot shown before the run finishes");
    // The slow run takes 3 s, so a cached view within 2 s can only have come from the disk.
    assert_eq!(v.snapshot.generated_at, Some(1_795_168_775));
}
