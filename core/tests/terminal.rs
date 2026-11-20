//! The terminal tail end to end: a fake home with a stub `fm-peek.sh`, read through the public
//! API, as the app reads it.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use quarterdeck_core::terminal::ansi::Tone;
use quarterdeck_core::terminal::{self, PeekTarget, TaskId, TerminalError, TerminalTail};
use quarterdeck_core::FmHome;

fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_home() -> (tempfile::TempDir, FmHome) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for d in ["state", "bin", "data"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("bin/fm-bearings-snapshot.sh"), "").unwrap();
    std::fs::write(root.join("bin/fm-inbox.sh"), "").unwrap();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/terminal/pane.txt"
        ),
        root.join("pane.txt"),
    )
    .unwrap();
    // The stub records its argv and environment, then prints the invented pane.
    script(
        &root.join("bin/fm-peek.sh"),
        r#"printf '%s\n' "$@" > "$FM_HOME/peek-args"
printf '%s' "$FM_HOME" > "$FM_HOME/peek-home"
cat "$FM_HOME/pane.txt""#,
    );
    let home = FmHome::at(root).unwrap();
    (dir, home)
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
    assert_eq!(TaskId::known("--help", known), Err(TerminalError::BadId));
    assert_eq!(TaskId::known("tmux:0.1", known), Err(TerminalError::BadId));
    assert_eq!(
        TaskId::known("other-task", known),
        Err(TerminalError::UnknownTask("other-task".into()))
    );
}

#[tokio::test]
async fn a_failing_peek_is_a_typed_error() {
    let (dir, home) = fake_home();
    script(
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
