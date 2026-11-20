//! The terminal tail end to end: a fake home with a stub `fm-peek.sh`, read through the public
//! API, as the app reads it.

mod common;

use std::path::Path;
use std::sync::Arc;

use quarterdeck_core::gateway::TaskIdError;
use quarterdeck_core::snapshot::Snapshot;
use quarterdeck_core::terminal::ansi::Tone;
use quarterdeck_core::terminal::{self, PeekTarget, TaskId, TerminalError, TerminalTail};
use quarterdeck_core::{FmHome, HomeState};

fn fake_home() -> (tempfile::TempDir, FmHome) {
    let dir = common::fake_home();
    let root = dir.path();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/terminal/pane.txt"
        ),
        root.join("pane.txt"),
    )
    .unwrap();
    // The stub records its argv and environment, then prints the invented pane.
    common::script(
        &root.join("bin/fm-peek.sh"),
        r#"printf '%s\n' "$@" > "$FM_HOME/peek-args"
printf '%s' "$FM_HOME" > "$FM_HOME/peek-home"
cat "$FM_HOME/pane.txt""#,
    );
    let home = FmHome::at(root).unwrap();
    (dir, home)
}

/// The engine's state with the shared fixture snapshot, optionally marking one task remote.
fn state(remote: Option<&str>) -> HomeState {
    let mut json: serde_json::Value = serde_json::from_str(common::FIXTURE).unwrap();
    if let Some(id) = remote {
        for t in json["tasks"].as_array_mut().unwrap() {
            if t["id"] == id {
                t["remote"] = serde_json::json!({ "host": "example.invalid" });
            }
        }
    }
    HomeState {
        snapshot: Some(Arc::new(Snapshot::parse(&json.to_string()).unwrap())),
        ..HomeState::default()
    }
}

fn target(raw: &str) -> PeekTarget {
    PeekTarget {
        id: TaskId::known(raw, |id| id == "app-login-fix").unwrap(),
        remote: false,
    }
}

fn plain(t: &TerminalTail) -> Vec<String> {
    t.lines
        .iter()
        .map(|l| l.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

#[tokio::test]
async fn tail_reads_sanitizes_and_styles_a_pane() {
    let (dir, home) = fake_home();
    let t = terminal::tail(&home, &target("app-login-fix"), 50)
        .await
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(dir.path().join("peek-args")).unwrap(),
        "app-login-fix\n50\n"
    );
    assert_eq!(
        Path::new(&std::fs::read_to_string(dir.path().join("peek-home")).unwrap()),
        dir.path()
    );
    assert_eq!(
        plain(&t),
        [
            "ledger-api/login-flake $ uv run pytest -q",
            ".......s [100%]",
            "107 passed, 1 skipped in 41.8s",
            "open the pull request",
        ]
    );
    // Colour survives as tones; titles, clipboard writes, screen clears and links do not.
    assert_eq!(t.lines[1][0].tone, Tone::Green);
    assert_eq!(t.lines[1][1].tone, Tone::Yellow);
    assert_eq!(t.lines[1][3].tone, Tone::Dim);
    let all: String = plain(&t).concat();
    for leaked in ["window title", "aGVsbG8", "example.invalid", "\u{1b}", "\r"] {
        assert!(!all.contains(leaked), "{leaked:?} leaked into the tail");
    }
    assert_eq!(t.requested, 50);
    assert!(!t.truncated);
}

#[tokio::test]
async fn tail_clamps_the_line_count() {
    let (dir, home) = fake_home();
    terminal::tail(&home, &target("app-login-fix"), 5000)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("peek-args")).unwrap(),
        "app-login-fix\n200\n"
    );
}

#[test]
fn ids_outside_the_fleet_or_unsafe_never_reach_peek() {
    let known = |id: &str| id == "app-login-fix";
    assert_eq!(TaskId::known("--help", known), Err(TaskIdError::BadId));
    assert_eq!(TaskId::known("tmux:0.1", known), Err(TaskIdError::BadId));
    assert_eq!(
        TaskId::known("other-task", known),
        Err(TaskIdError::Unknown("other-task".into()))
    );
}

#[tokio::test]
async fn a_failing_peek_is_a_typed_error() {
    let (dir, home) = fake_home();
    common::script(
        &dir.path().join("bin/fm-peek.sh"),
        "echo 'error: backend target gone' >&2; exit 1",
    );
    let e = terminal::tail(&home, &target("app-login-fix"), 10)
        .await
        .unwrap_err();
    assert_eq!(
        e,
        TerminalError::PeekFailed("error: backend target gone".into())
    );
    assert_eq!(
        e.to_string(),
        "the terminal could not be read: error: backend target gone"
    );
}

#[tokio::test]
async fn read_takes_ids_from_the_snapshot_only() {
    let (dir, home) = fake_home();
    let t = terminal::read(&home, &state(None), "app-login-fix")
        .await
        .unwrap();
    assert_eq!(t.task_id, "app-login-fix");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("peek-args")).unwrap(),
        "app-login-fix\n200\n"
    );
    std::fs::remove_file(dir.path().join("peek-args")).unwrap();
    for (id, want) in [
        (
            "not-in-fleet",
            TerminalError::UnknownTask("not-in-fleet".into()),
        ),
        ("../state", TerminalError::BadId),
        ("-h", TerminalError::BadId),
    ] {
        assert_eq!(
            terminal::read(&home, &state(None), id).await.unwrap_err(),
            want
        );
    }
    // No snapshot yet: nothing is known, so nothing is read.
    assert_eq!(
        terminal::read(&home, &HomeState::default(), "app-login-fix")
            .await
            .unwrap_err(),
        TerminalError::UnknownTask("app-login-fix".into())
    );
    assert!(!dir.path().join("peek-args").exists());
}

#[tokio::test]
async fn read_refuses_a_remote_task_without_running_peek() {
    let (dir, home) = fake_home();
    assert_eq!(
        terminal::read(&home, &state(Some("app-login-fix")), "app-login-fix")
            .await
            .unwrap_err(),
        TerminalError::Remote
    );
    assert!(!dir.path().join("peek-args").exists());
}
