//! "Open in Orca": bring a task's terminal forward, and nothing else.
//!
//! Orca's `orca://` scheme has no terminal route, so Orca's own CLI is used, with exactly one
//! command: `orca terminal switch --terminal <handle> --json`, which only changes which terminal
//! the Orca window shows. A task without a terminal handle on record has no button.
//!
//! The handle comes from the task's snapshot record (`endpoint.target`), never from the window,
//! must match `^term_[A-Za-z0-9_-]{1,64}$`, and is used only when the task's backend is `orca`.
//! No `--environment` or pairing code is ever passed, so only the local Orca is addressed. The
//! launch goes through the core's external allow-list ([`crate::external::External::OrcaFocus`]).

use crate::engine::HomeState;
use crate::snapshot::Task;

/// Where Orca installs its CLI.
pub const ORCA_BIN: &str = "/Applications/Orca.app/Contents/Resources/bin/orca";

/// A checked Orca terminal handle. It can only be made from a well-formed value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalHandle(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrcaError {
    #[error("{0:?} is not a task in the current snapshot")]
    UnknownTask(String),
    #[error("task {0} has no Orca terminal on record")]
    NotInOrca(String),
}

/// `term_` followed by 1 to 64 letters, digits, `-` or `_`.
pub fn valid_terminal(s: &str) -> bool {
    s.strip_prefix("term_").is_some_and(|rest| {
        (1..=64).contains(&rest.len())
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    })
}

impl TerminalHandle {
    pub fn new(s: &str) -> Option<Self> {
        valid_terminal(s).then(|| Self(s.to_string()))
    }

    /// The argument vector for the Orca CLI.
    pub fn args(&self) -> Vec<String> {
        vec![
            "terminal".into(),
            "switch".into(),
            "--terminal".into(),
            self.0.clone(),
            "--json".into(),
        ]
    }
}

/// The terminal handle for `task`, when Orca runs it and a well-formed handle is on record.
pub fn handle_for(task: &Task) -> Option<TerminalHandle> {
    if task.backend.as_deref() != Some("orca") {
        return None;
    }
    task.endpoint_target
        .as_deref()
        .and_then(TerminalHandle::new)
}

/// Resolves task `id` in `state` to its terminal handle.
pub fn resolve(state: &HomeState, id: &str) -> Result<TerminalHandle, OrcaError> {
    let task = super::find_task(state, id).ok_or_else(|| OrcaError::UnknownTask(id.to_string()))?;
    handle_for(task).ok_or_else(|| OrcaError::NotInOrca(id.to_string()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::snapshot::Snapshot;

    fn task(backend: &str, target: Option<&str>) -> Task {
        Task {
            id: "t1".into(),
            backend: Some(backend.into()),
            endpoint_target: target.map(Into::into),
            worktree: Some(PathBuf::from("/p/wt/t1")),
            ..Task::default()
        }
    }

    #[test]
    fn switches_the_terminal_and_nothing_else() {
        let h = handle_for(&task("orca", Some("term_ab-12_c"))).unwrap();
        let args = h.args();
        assert_eq!(
            args,
            ["terminal", "switch", "--terminal", "term_ab-12_c", "--json"]
        );
        assert!(!args
            .iter()
            .any(|a| a.contains("environment") || a.contains("pairing") || a.contains("worktree")));
    }

    #[test]
    fn no_handle_means_no_target() {
        // A worktree alone is not enough: there is no worktree fallback.
        assert_eq!(handle_for(&task("orca", None)), None);
        assert_eq!(handle_for(&task("orca", Some("--environment"))), None);
        assert_eq!(handle_for(&task("tmux", Some("term_a"))), None);
    }

    #[test]
    fn handle_shape() {
        assert!(valid_terminal("term_a"));
        assert!(valid_terminal(&format!("term_{}", "a".repeat(64))));
        for bad in [
            "term_",
            "term_a b",
            "term_a;rm",
            "xterm_a",
            &format!("term_{}", "a".repeat(65)),
        ] {
            assert!(!valid_terminal(bad), "{bad}");
            assert_eq!(TerminalHandle::new(bad), None);
        }
    }

    #[test]
    fn resolves_only_tasks_in_the_snapshot() {
        let s = HomeState {
            snapshot: Some(Arc::new(Snapshot {
                tasks: vec![
                    task("orca", Some("term_x")),
                    Task {
                        id: "plain".into(),
                        ..Task::default()
                    },
                ],
                ..Snapshot::default()
            })),
            ..HomeState::default()
        };
        assert_eq!(resolve(&s, "t1"), Ok(TerminalHandle("term_x".into())));
        assert_eq!(
            resolve(&s, "nope"),
            Err(OrcaError::UnknownTask("nope".into()))
        );
        assert_eq!(
            resolve(&s, "../t1"),
            Err(OrcaError::UnknownTask("../t1".into()))
        );
        assert_eq!(
            resolve(&s, "plain"),
            Err(OrcaError::NotInOrca("plain".into()))
        );
    }
}
