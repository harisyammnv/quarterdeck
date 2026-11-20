//! The command gateway: the only way this crate runs anything.
//!
//! It holds an allow-list of Firstmate entry points and runs each one with an argument vector,
//! never through a shell, with `FM_HOME` set explicitly, stdin closed and a timeout.
//! Nothing here can spawn, steer, merge or clean up work; those scripts are not on the list.

use std::ffi::OsString;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::process::Command;

use crate::home::FmHome;

/// The allow-listed entry points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `bin/fm-fleet-snapshot.sh --json`: the read-only structured fleet snapshot.
    FleetSnapshot,
    /// `bin/fm-inbox.sh note --request-id <id> --json -- <text>`: the one write path.
    InboxNote { request_id: String, text: String },
}

impl Entry {
    fn script(&self) -> &'static str {
        match self {
            Self::FleetSnapshot => "fm-fleet-snapshot.sh",
            Self::InboxNote { .. } => "fm-inbox.sh",
        }
    }

    fn args(&self) -> Vec<OsString> {
        match self {
            Self::FleetSnapshot => vec!["--json".into()],
            Self::InboxNote { request_id, text } => vec![
                "note".into(),
                "--request-id".into(),
                request_id.into(),
                "--json".into(),
                // Everything after `--` is body text, so a note starting with dashes is safe.
                "--".into(),
                text.into(),
            ],
        }
    }

    /// Default time budget for each entry point.
    pub fn default_timeout(&self) -> Duration {
        match self {
            Self::FleetSnapshot => Duration::from_secs(90),
            Self::InboxNote { .. } => Duration::from_secs(30),
        }
    }
}

/// A task id the app will pass on: `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$`. Callers also check
/// that the id names a task in the current snapshot.
pub fn valid_task_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id.len() <= 128
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// "exit 3", or "killed by a signal" when there is no exit code.
pub fn exit_label(code: Option<i32>) -> String {
    code.map_or_else(|| "killed by a signal".to_string(), |c| format!("exit {c}"))
}

/// What a finished command produced.
#[derive(Clone, Debug)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("could not start {script}: {source}")]
    Spawn {
        script: &'static str,
        source: std::io::Error,
    },
    #[error("{script} did not finish within {secs} s")]
    Timeout { script: &'static str, secs: u64 },
}

/// Variables passed through to Firstmate's scripts. Everything else is dropped, so a variable
/// set in whatever shell launched the app (another task's ids, a tool's shims) cannot change
/// how Firstmate behaves.
const PASSED_ENV: [&str; 9] = [
    "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TERM",
];

/// Directories appended to the script `PATH` when missing: Homebrew for `python3` and other
/// tools, and Orca's bundled CLI, which Firstmate uses to read worker endpoints.
const EXTRA_PATH: [&str; 4] = [
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/Applications/Orca.app/Contents/Resources/bin",
];

const PATH_MARKER: &str = "__QD_PATH__=";

/// The `PATH` of the user's login shell, read once.
///
/// An app opened from Finder inherits a minimal `PATH`, and an app started from an agent's
/// shell can inherit shims that break Firstmate's scripts. Asking the login shell gives the
/// same `PATH` a new terminal would have.
fn login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut cmd = std::process::Command::new(shell);
    cmd.args([
        "-l",
        "-i",
        "-c",
        &format!("printf '\\n{PATH_MARKER}%s\\n' \"$PATH\""),
    ])
    .env_clear()
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    for key in PASSED_ENV {
        if let Some(v) = std::env::var_os(key) {
            cmd.env(key, v);
        }
    }
    let mut child = cmd.spawn().ok()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    std::io::Read::read_to_string(child.stdout.as_mut()?, &mut out).ok()?;
    parse_marked_path(&out)
}

fn parse_marked_path(out: &str) -> Option<String> {
    out.lines()
        .rev()
        .find_map(|l| l.strip_prefix(PATH_MARKER))
        .map(str::to_string)
        .filter(|p| !p.is_empty())
}

fn with_extras(base: &OsString) -> OsString {
    let mut parts: Vec<_> = std::env::split_paths(base).collect();
    for extra in EXTRA_PATH {
        if !parts.iter().any(|p| p.as_os_str() == extra) {
            parts.push(extra.into());
        }
    }
    std::env::join_paths(parts).unwrap_or_else(|_| base.clone())
}

/// The `PATH` scripts run with: the login shell's, or the inherited one if that fails, plus
/// [`EXTRA_PATH`].
fn script_path() -> &'static OsString {
    static PATH: OnceLock<OsString> = OnceLock::new();
    PATH.get_or_init(|| {
        let base = login_shell_path()
            .map(OsString::from)
            .unwrap_or_else(|| std::env::var_os("PATH").unwrap_or_default());
        with_extras(&base)
    })
}

/// Runs one allow-listed entry point against `home`.
pub async fn run(home: &FmHome, entry: &Entry, timeout: Duration) -> Result<Output, GatewayError> {
    let script = entry.script();
    let path = tokio::task::spawn_blocking(script_path)
        .await
        .map_err(|e| GatewayError::Spawn {
            script,
            source: std::io::Error::other(e),
        })?;
    let mut cmd = Command::new(home.script(script));
    cmd.env_clear();
    for key in PASSED_ENV {
        if let Some(v) = std::env::var_os(key) {
            cmd.env(key, v);
        }
    }
    cmd.args(entry.args())
        .current_dir(home.root())
        .env("FM_HOME", home.root())
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = cmd
        .spawn()
        .map_err(|source| GatewayError::Spawn { script, source })?;
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => Ok(Output {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
        Ok(Err(source)) => Err(GatewayError::Spawn { script, source }),
        Err(_) => Err(GatewayError::Timeout {
            script,
            secs: timeout.as_secs(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_text_follows_double_dash() {
        let e = Entry::InboxNote {
            request_id: "qd-1".into(),
            text: "--all is body text".into(),
        };
        let args = e.args();
        let dd = args.iter().position(|a| a == "--").unwrap();
        assert_eq!(args[dd + 1], "--all is body text");
        assert_eq!(args.len(), dd + 2);
    }

    #[test]
    fn task_ids() {
        for ok in ["a", "qd-phase1", "A.b_c-9", &"x".repeat(128)] {
            assert!(valid_task_id(ok), "{ok}");
        }
        for bad in [
            "",
            "-a",
            ".a",
            "_a",
            "a/b",
            "../a",
            "a b",
            "a;b",
            &"x".repeat(129),
            "é",
        ] {
            assert!(!valid_task_id(bad), "{bad}");
        }
    }

    #[test]
    fn snapshot_argv_is_fixed() {
        let e = Entry::FleetSnapshot;
        assert_eq!(e.script(), "fm-fleet-snapshot.sh");
        assert_eq!(e.args(), vec![OsString::from("--json")]);
    }

    #[test]
    fn path_gains_extras_once() {
        let p = with_extras(&OsString::from("/opt/homebrew/bin:/x"));
        let dirs: Vec<_> = std::env::split_paths(&p).collect();
        assert_eq!(
            dirs.iter()
                .filter(|d| d.as_os_str() == "/opt/homebrew/bin")
                .count(),
            1
        );
        assert_eq!(dirs[1].as_os_str(), "/x");
        assert!(dirs.iter().any(|d| d.as_os_str() == "/usr/bin"));
    }

    #[test]
    fn marked_path_ignores_shell_noise() {
        let out = "welcome banner\n__QD_PATH__=/a:/b\n";
        assert_eq!(parse_marked_path(out).as_deref(), Some("/a:/b"));
        assert_eq!(parse_marked_path("no marker"), None);
        assert_eq!(parse_marked_path("__QD_PATH__=\n"), None);
    }
}
