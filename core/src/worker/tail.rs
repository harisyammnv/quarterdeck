//! The live tail of a worker's terminal, read through Firstmate's `bin/fm-peek.sh`.
//!
//! The peek script already dispatches to the right runtime backend, so the app never calls a
//! backend itself and never sends input. The result is a bounded text capture of recent
//! output, not a byte-accurate stream, and it is sanitized before the webview sees it.

use std::hash::{Hash, Hasher};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::Semaphore;

use super::sanitize::sanitize;
use super::TaskId;
use crate::gateway::{self, Entry};
use crate::home::FmHome;

/// Lines asked for when the caller does not say.
pub const DEFAULT_LINES: u16 = 200;
/// The most lines one read returns.
pub const MAX_LINES: u16 = 500;
/// The most bytes of peek output kept; anything before is dropped.
pub const MAX_BYTES: usize = 256 * 1024;
/// How long one peek may take.
pub const PEEK_TIMEOUT: Duration = Duration::from_secs(10);
/// At most this many peeks run at once, whatever the UI asks for.
const MAX_CONCURRENT: usize = 2;

/// A captured tail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TerminalTail {
    pub task_id: String,
    /// Sanitized text: printable characters, newlines, tabs and colour sequences only.
    pub text: String,
    /// Lines in `text`.
    pub lines: u32,
    /// The line count that was asked for.
    pub requested: u32,
    /// True when the capture was longer than the byte or line bound and its start was cut.
    pub truncated: bool,
    /// A fingerprint of `text`, so the UI can skip repainting an unchanged tail.
    pub digest: String,
    /// When the capture finished, in milliseconds since the epoch.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub captured_ms: i64,
}

/// Why no tail could be read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum TailUnavailable {
    /// The task runs on another host. Reading it would mean a network hop, which this
    /// read-only companion does not make.
    Remote,
    /// The peek script failed, for example because the terminal is gone.
    Failed { message: String },
    /// The peek script did not answer in time.
    TimedOut,
}

/// The outcome of one tail read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TailRead {
    Tail(TerminalTail),
    Unavailable(TailUnavailable),
}

/// Clamps a requested line count to `1..=MAX_LINES`, with 0 meaning the default.
pub fn clamp_lines(lines: u16) -> u16 {
    match lines {
        0 => DEFAULT_LINES,
        n => n.min(MAX_LINES),
    }
}

/// Whether the task's meta record names a remote host.
fn is_remote(home: &FmHome, id: &TaskId) -> bool {
    let meta = home.state_dir().join(format!("{id}.meta"));
    std::fs::read_to_string(meta).is_ok_and(|m| {
        m.lines().any(|l| {
            l.split_once('=')
                .is_some_and(|(k, v)| k.trim() == "remote_host" && !v.trim().is_empty())
        })
    })
}

/// Reads the last `lines` lines of the task's terminal.
pub async fn read(home: &FmHome, id: &TaskId, lines: u16) -> TailRead {
    static GATE: Semaphore = Semaphore::const_new(MAX_CONCURRENT);
    let lines = clamp_lines(lines);
    if is_remote(home, id) {
        return TailRead::Unavailable(TailUnavailable::Remote);
    }
    let Ok(_permit) = GATE.acquire().await else {
        return TailRead::Unavailable(TailUnavailable::TimedOut);
    };
    let entry = Entry::Peek {
        id: id.clone(),
        lines,
    };
    match gateway::run(home, &entry, PEEK_TIMEOUT).await {
        Ok(out) if out.code == Some(0) => TailRead::Tail(shape(id, &out.stdout, lines)),
        Ok(out) => TailRead::Unavailable(TailUnavailable::Failed {
            message: failure_message(&out.stderr, out.code),
        }),
        Err(gateway::GatewayError::Timeout { .. }) => {
            TailRead::Unavailable(TailUnavailable::TimedOut)
        }
        Err(e) => TailRead::Unavailable(TailUnavailable::Failed {
            message: e.to_string(),
        }),
    }
}

/// The first meaningful stderr line, sanitized and bounded, or the exit label.
fn failure_message(stderr: &str, code: Option<i32>) -> String {
    let line = sanitize(stderr)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(240).collect::<String>());
    line.unwrap_or_else(|| format!("fm-peek.sh ended with {}", gateway::exit_label(code)))
}

/// Bounds and sanitizes raw peek output.
pub fn shape(id: &TaskId, raw: &str, lines: u16) -> TerminalTail {
    let mut truncated = false;
    let mut start = 0;
    if raw.len() > MAX_BYTES {
        truncated = true;
        start = raw.len() - MAX_BYTES;
        while !raw.is_char_boundary(start) {
            start += 1;
        }
    }
    let clean = sanitize(&raw[start..]);
    let body = clean.trim_end_matches('\n');
    let all: Vec<&str> = body.split('\n').collect();
    let mut keep = all.as_slice();
    if keep.len() > usize::from(lines) {
        truncated = true;
        keep = &keep[keep.len() - usize::from(lines)..];
    }
    // A byte cut lands mid-line; drop that partial first line.
    if start > 0 && keep.len() == all.len() && keep.len() > 1 {
        keep = &keep[1..];
    }
    let text = keep.join("\n");
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    TerminalTail {
        task_id: id.to_string(),
        lines: u32::try_from(if text.is_empty() { 0 } else { keep.len() }).unwrap_or(u32::MAX),
        requested: u32::from(lines),
        truncated,
        digest: format!("{:016x}", h.finish()),
        text,
        captured_ms: crate::now_ms(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testutil::{fake_home, script};
    use super::*;

    fn id() -> TaskId {
        TaskId::parse("app-login-fix").unwrap()
    }

    #[test]
    fn clamps_lines() {
        assert_eq!(clamp_lines(0), DEFAULT_LINES);
        assert_eq!(clamp_lines(7), 7);
        assert_eq!(clamp_lines(u16::MAX), MAX_LINES);
    }

    #[test]
    fn shape_keeps_the_last_lines() {
        let raw = "1\n2\n3\n4\n";
        let t = shape(&id(), raw, 2);
        assert_eq!(t.text, "3\n4");
        assert_eq!(t.lines, 2);
        assert!(t.truncated);
        let t = shape(&id(), raw, 10);
        assert_eq!(t.text, "1\n2\n3\n4");
        assert!(!t.truncated);
        assert_eq!(shape(&id(), "", 10).lines, 0);
    }

    #[test]
    fn shape_bounds_bytes_and_cuts_on_a_line() {
        let line = "x".repeat(1000);
        let raw = format!("é{}\n", vec![line.as_str(); 400].join("\n"));
        let t = shape(&id(), &raw, MAX_LINES);
        assert!(t.truncated);
        assert!(t.text.len() <= MAX_BYTES);
        assert!(t.text.split('\n').all(|l| l.len() == 1000));
    }

    #[test]
    fn shape_sanitizes_and_digests() {
        let a = shape(&id(), "\u{1b}]0;title\u{7}hi\r\n", 5);
        assert_eq!(a.text, "hi");
        let b = shape(&id(), "hi\n", 5);
        assert_eq!(a.digest, b.digest);
        assert_ne!(a.digest, shape(&id(), "ho", 5).digest);
    }

    #[tokio::test]
    async fn read_runs_peek_with_the_id_and_line_count() {
        let (dir, home) = fake_home();
        script(
            &dir.path().join("bin/fm-peek.sh"),
            r#"printf '%s\n' "$@" > "$FM_HOME/peek-args"; printf 'line one\n\033[31mred\033[0m\n'"#,
        );
        let TailRead::Tail(t) = read(&home, &id(), 0).await else {
            panic!("expected a tail");
        };
        assert_eq!(t.text, "line one\n\u{1b}[31mred\u{1b}[0m");
        assert_eq!(t.requested, u32::from(DEFAULT_LINES));
        let args = std::fs::read_to_string(dir.path().join("peek-args")).unwrap();
        assert_eq!(args, "app-login-fix\n200\n");
    }

    #[tokio::test]
    async fn read_reports_failure_from_stderr() {
        let (dir, home) = fake_home();
        script(
            &dir.path().join("bin/fm-peek.sh"),
            "echo 'error: no such pane' >&2; exit 3",
        );
        assert_eq!(
            read(&home, &id(), 10).await,
            TailRead::Unavailable(TailUnavailable::Failed {
                message: "error: no such pane".into()
            })
        );
        script(&dir.path().join("bin/fm-peek.sh"), "exit 4");
        assert_eq!(
            read(&home, &id(), 10).await,
            TailRead::Unavailable(TailUnavailable::Failed {
                message: "fm-peek.sh ended with exit 4".into()
            })
        );
    }

    #[tokio::test]
    async fn read_refuses_remote_tasks_without_running_peek() {
        let (dir, home) = fake_home();
        std::fs::write(
            dir.path().join("state/app-login-fix.meta"),
            "kind=ship\nremote_host=example-host\n",
        )
        .unwrap();
        script(
            &dir.path().join("bin/fm-peek.sh"),
            r#"touch "$FM_HOME/peek-ran""#,
        );
        assert_eq!(
            read(&home, &id(), 10).await,
            TailRead::Unavailable(TailUnavailable::Remote)
        );
        assert!(!dir.path().join("peek-ran").exists());
    }
}
