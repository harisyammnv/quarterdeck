//! The command gateway: the only way this crate runs anything.
//!
//! It holds an allow-list of Firstmate entry points and runs each one with an argument vector,
//! never through a shell, with `FM_HOME` set explicitly, stdin closed and a timeout.
//! Nothing here can spawn, steer, merge or clean up work; those scripts are not on the list.

use std::ffi::OsString;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::home::FmHome;

/// The allow-listed entry points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// `bin/fm-bearings-snapshot.sh --json --all-in-flight`: read-only fleet projection.
    BearingsSnapshot,
    /// `bin/fm-inbox.sh note --request-id <id> --json -- <text>`: the one write path.
    InboxNote { request_id: String, text: String },
}

impl Entry {
    fn script(&self) -> &'static str {
        match self {
            Self::BearingsSnapshot => "fm-bearings-snapshot.sh",
            Self::InboxNote { .. } => "fm-inbox.sh",
        }
    }

    fn args(&self) -> Vec<OsString> {
        match self {
            Self::BearingsSnapshot => vec!["--json".into(), "--all-in-flight".into()],
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
            Self::BearingsSnapshot => Duration::from_secs(90),
            Self::InboxNote { .. } => Duration::from_secs(30),
        }
    }
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

/// Directories appended to `PATH` when missing, because an app launched from Finder gets a
/// minimal `PATH` and Firstmate's scripts need `python3` and Homebrew tools.
const EXTRA_PATH: [&str; 3] = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];

fn augmented_path() -> OsString {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let mut parts: Vec<_> = std::env::split_paths(&current).collect();
    for extra in EXTRA_PATH {
        if !parts.iter().any(|p| p.as_os_str() == extra) {
            parts.push(extra.into());
        }
    }
    std::env::join_paths(parts).unwrap_or(current)
}

/// Runs one allow-listed entry point against `home`.
pub async fn run(home: &FmHome, entry: &Entry, timeout: Duration) -> Result<Output, GatewayError> {
    let script = entry.script();
    let mut cmd = Command::new(home.script(script));
    cmd.args(entry.args())
        .current_dir(home.root())
        .env("FM_HOME", home.root())
        .env("PATH", augmented_path())
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
    fn path_gains_homebrew() {
        let p = augmented_path();
        assert!(std::env::split_paths(&p).any(|d| d.as_os_str() == "/opt/homebrew/bin"));
    }
}
