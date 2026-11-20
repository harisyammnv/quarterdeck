//! A worker's terminal, read-only: the bounded tail of its pane through Firstmate's
//! `bin/fm-peek.sh`.
//!
//! The peek script already dispatches to the right runtime backend, so the app never talks to
//! a backend itself and never sends input. The result is a recent capture, not a byte-accurate
//! stream; it is sanitized and turned into styled plain-text spans before the webview sees it.

pub mod ansi;

use std::hash::{Hash, Hasher};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::Semaphore;

use crate::engine::HomeState;
pub use crate::gateway::TaskId;
use crate::gateway::{self, Entry, TaskIdError};
use crate::home::FmHome;
use ansi::{sanitize, spans, Span};

/// Lines asked for when the caller does not say, and the most one read returns.
pub const MAX_LINES: u16 = 200;
/// The most bytes of peek output kept; anything before is dropped.
pub const MAX_BYTES: usize = 64 * 1024;
/// How long one peek may take.
pub const PEEK_TIMEOUT: Duration = Duration::from_secs(20);
/// At most this many peeks run at once, whatever the UI asks for.
const MAX_CONCURRENT: usize = 2;

/// What the core knows about the task whose terminal is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeekTarget {
    pub id: TaskId,
    /// The task runs on another host. Its pane is not read: that would be a network hop,
    /// which this read-only companion does not make.
    pub remote: bool,
}

/// A captured tail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TerminalTail {
    pub task_id: String,
    /// The captured lines, oldest first, as styled plain-text spans.
    pub lines: Vec<Vec<Span>>,
    /// The line count that was asked for.
    pub requested: u32,
    /// True when the capture was longer than the byte or line bound and its start was cut.
    pub truncated: bool,
    /// A fingerprint of the content, so the UI can skip repainting an unchanged tail.
    pub digest: String,
    /// When the capture finished, in Unix seconds.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub captured_at: i64,
}

/// Why no tail could be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TerminalError {
    #[error("not a task id")]
    BadId,
    #[error("no task {0} in the current fleet")]
    UnknownTask(String),
    #[error("this task runs on another host; its terminal is not read from here")]
    Remote,
    #[error("the terminal could not be read: {0}")]
    PeekFailed(String),
    #[error("the terminal did not answer within {} s", PEEK_TIMEOUT.as_secs())]
    TimedOut,
}

/// Reads the tail of task `id`'s terminal. The id must name a task in the current snapshot;
/// a task the snapshot marks as remote is refused without running anything.
pub async fn read(
    home: &FmHome,
    state: &HomeState,
    id: &str,
) -> Result<TerminalTail, TerminalError> {
    let snapshot = state.snapshot.as_deref();
    let task = TaskId::parse(id)
        .ok()
        .and_then(|_| snapshot.and_then(|s| s.task(id)));
    let id = TaskId::known(id, |_| task.is_some())?;
    let target = PeekTarget {
        id,
        remote: task.is_some_and(|t| t.remote),
    };
    tail(home, &target, MAX_LINES).await
}

impl From<TaskIdError> for TerminalError {
    fn from(e: TaskIdError) -> Self {
        match e {
            TaskIdError::BadId => Self::BadId,
            TaskIdError::Unknown(id) => Self::UnknownTask(id),
        }
    }
}

/// Clamps a requested line count to `1..=MAX_LINES`, with 0 meaning the most.
pub fn clamp_lines(lines: u16) -> u16 {
    match lines {
        0 => MAX_LINES,
        n => n.min(MAX_LINES),
    }
}

/// Reads the last `lines` lines of the task's terminal.
pub async fn tail(
    home: &FmHome,
    target: &PeekTarget,
    lines: u16,
) -> Result<TerminalTail, TerminalError> {
    static GATE: Semaphore = Semaphore::const_new(MAX_CONCURRENT);
    if target.remote {
        return Err(TerminalError::Remote);
    }
    let lines = clamp_lines(lines);
    let _permit = GATE.acquire().await.map_err(|_| TerminalError::TimedOut)?;
    let entry = Entry::Peek {
        task_id: target.id.clone(),
        lines,
    };
    match gateway::run(home, &entry, PEEK_TIMEOUT).await {
        Ok(out) if out.code == Some(0) => Ok(shape(&target.id, &out.stdout, lines)),
        Ok(out) => Err(TerminalError::PeekFailed(failure_message(
            &out.stderr,
            out.code,
        ))),
        Err(gateway::GatewayError::Timeout { .. }) => Err(TerminalError::TimedOut),
        Err(e) => Err(TerminalError::PeekFailed(e.to_string())),
    }
}

/// What to show when the peek failed: the last `error:` line of its stderr, else its last
/// non-empty line, sanitized and bounded, else the exit label. `fm-peek.sh` runs Firstmate's
/// guard first, which can print a supervision banner on stderr ahead of the real error.
fn failure_message(stderr: &str, code: Option<i32>) -> String {
    let clean = sanitize(stderr);
    let lines: Vec<&str> = clean
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines
        .iter()
        .rev()
        .find(|l| l.starts_with("error:"))
        .or_else(|| lines.last())
        .map(|l| {
            l.replace('\u{1b}', "")
                .chars()
                .take(240)
                .collect::<String>()
        })
        .unwrap_or_else(|| format!("fm-peek.sh ended with {}", gateway::exit_label(code)))
}

/// Bounds, sanitizes and styles raw peek output.
pub fn shape(id: &TaskId, raw: &str, lines: u16) -> TerminalTail {
    let mut truncated = false;
    let mut start = 0;
    if raw.len() > MAX_BYTES {
        truncated = true;
        start = raw.len() - MAX_BYTES;
        while !raw.is_char_boundary(start) {
            start += 1;
        }
        // The cut lands mid-line; start after the next newline when there is one.
        if let Some(nl) = raw[start..].find('\n') {
            start += nl + 1;
        }
    }
    let clean = sanitize(&raw[start..]);
    let body = clean.trim_end_matches('\n');
    let mut keep: Vec<&str> = if body.is_empty() {
        Vec::new()
    } else {
        body.split('\n').collect()
    };
    if keep.len() > usize::from(lines) {
        truncated = true;
        keep.drain(..keep.len() - usize::from(lines));
    }
    let text = keep.join("\n");
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    TerminalTail {
        task_id: id.to_string(),
        lines: if text.is_empty() {
            Vec::new()
        } else {
            spans(&text)
        },
        requested: u32::from(lines),
        truncated,
        digest: format!("{:016x}", h.finish()),
        captured_at: crate::now_ms() / 1000,
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::ansi::Tone;
    use super::*;

    fn script(path: &std::path::Path, body: &str) {
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn fake_home() -> (tempfile::TempDir, FmHome) {
        let dir = tempfile::tempdir().unwrap();
        for d in ["state", "bin", "data"] {
            std::fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        std::fs::write(dir.path().join("bin/fm-fleet-snapshot.sh"), "").unwrap();
        std::fs::write(dir.path().join("bin/fm-inbox.sh"), "").unwrap();
        let home = FmHome::at(dir.path()).unwrap();
        (dir, home)
    }

    fn target() -> PeekTarget {
        PeekTarget {
            id: TaskId::parse("app-login-fix").unwrap(),
            remote: false,
        }
    }

    fn text(t: &TerminalTail) -> String {
        t.lines
            .iter()
            .map(|l| l.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn ids_accept_plain_names() {
        for ok in ["a", "app-login-fix", "qd.terminal_2", "9lives"] {
            assert!(TaskId::parse(ok).is_ok(), "{ok}");
        }
        assert!(TaskId::parse(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn ids_reject_paths_options_and_targets() {
        for bad in [
            "",
            "-n",
            "--all",
            "../x",
            "a/b",
            "a..b",
            ".hidden",
            "a b",
            "a:b",
            "%1",
            "a\n",
            "tmux:main:1",
            "é",
        ] {
            assert_eq!(TaskId::parse(bad), Err(TaskIdError::BadId), "{bad:?}");
        }
        assert!(TaskId::parse(&"a".repeat(129)).is_err());
    }

    #[test]
    fn ids_must_be_in_the_fleet() {
        assert!(TaskId::known("app-login-fix", |id| id == "app-login-fix").is_ok());
        assert_eq!(
            TaskId::known("other", |_| false),
            Err(TaskIdError::Unknown("other".into()))
        );
        assert_eq!(TaskId::known("../x", |_| true), Err(TaskIdError::BadId));
    }

    #[test]
    fn clamps_lines() {
        assert_eq!(clamp_lines(0), MAX_LINES);
        assert_eq!(clamp_lines(7), 7);
        assert_eq!(clamp_lines(u16::MAX), MAX_LINES);
    }

    #[test]
    fn shape_keeps_the_last_lines() {
        let id = target().id;
        let t = shape(&id, "1\n2\n3\n4\n", 2);
        assert_eq!(text(&t), "3\n4");
        assert!(t.truncated);
        let t = shape(&id, "1\n2\n3\n4\n", 10);
        assert_eq!(t.lines.len(), 4);
        assert!(!t.truncated);
        assert!(shape(&id, "", 10).lines.is_empty());
        assert!(shape(&id, "\n\n", 10).lines.is_empty());
    }

    #[test]
    fn shape_bounds_bytes_and_cuts_on_a_line() {
        let line = "x".repeat(1000);
        let raw = format!("é{}\n", vec![line.as_str(); 100].join("\n"));
        let t = shape(&target().id, &raw, MAX_LINES);
        assert!(t.truncated);
        let body = text(&t);
        assert!(body.len() <= MAX_BYTES);
        assert!(body.split('\n').all(|l| l.len() == 1000));
    }

    #[test]
    fn shape_sanitizes_styles_and_digests() {
        let id = target().id;
        let a = shape(&id, "\u{1b}]0;title\u{7}hi\r\n", 5);
        assert_eq!(text(&a), "hi");
        let b = shape(&id, "hi\n", 5);
        assert_eq!(a.digest, b.digest);
        assert_ne!(a.digest, shape(&id, "ho", 5).digest);
        let c = shape(&id, "\u{1b}[31mred\u{1b}[0m ok", 5);
        assert_eq!(c.lines[0][0].tone, Tone::Red);
        assert_eq!(text(&c), "red ok");
    }

    #[tokio::test]
    async fn tail_runs_peek_with_the_id_and_line_count() {
        let (dir, home) = fake_home();
        script(
            &dir.path().join("bin/fm-peek.sh"),
            r#"printf '%s\n' "$@" > "$FM_HOME/peek-args"; printf 'line one\n\033[31mred\033[0m\n'"#,
        );
        let t = tail(&home, &target(), 0).await.unwrap();
        assert_eq!(text(&t), "line one\nred");
        assert_eq!(t.lines[1][0].tone, Tone::Red);
        assert_eq!(t.requested, u32::from(MAX_LINES));
        let args = std::fs::read_to_string(dir.path().join("peek-args")).unwrap();
        assert_eq!(args, "app-login-fix\n200\n");
    }

    #[test]
    fn failure_prefers_the_error_line_over_a_guard_banner() {
        let stderr = "=== WARNING: supervision watcher is down ===\nrun fm-arm to restart it\nerror: no live endpoint for app-login-fix\n";
        assert_eq!(
            failure_message(stderr, Some(1)),
            "error: no live endpoint for app-login-fix"
        );
        assert_eq!(
            failure_message("banner\nlast words\n\n", Some(1)),
            "last words"
        );
        assert_eq!(failure_message("", Some(2)), "fm-peek.sh ended with exit 2");
    }

    #[tokio::test]
    async fn tail_reports_failure_from_stderr() {
        let (dir, home) = fake_home();
        let peek = dir.path().join("bin/fm-peek.sh");
        script(
            &peek,
            "printf '\\033]0;x\\007error: no such pane\\n' >&2; exit 3",
        );
        assert_eq!(
            tail(&home, &target(), 10).await,
            Err(TerminalError::PeekFailed("error: no such pane".into()))
        );
        script(&peek, "exit 4");
        assert_eq!(
            tail(&home, &target(), 10).await,
            Err(TerminalError::PeekFailed(
                "fm-peek.sh ended with exit 4".into()
            ))
        );
    }

    #[tokio::test]
    async fn tail_refuses_remote_tasks_without_running_peek() {
        let (dir, home) = fake_home();
        script(
            &dir.path().join("bin/fm-peek.sh"),
            r#"touch "$FM_HOME/peek-ran""#,
        );
        let remote = PeekTarget {
            remote: true,
            ..target()
        };
        assert_eq!(tail(&home, &remote, 10).await, Err(TerminalError::Remote));
        assert!(!dir.path().join("peek-ran").exists());
    }
}
