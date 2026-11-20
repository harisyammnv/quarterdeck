//! "Open in Orca": the Orca CLI call that brings a task forward, and nothing else.
//!
//! Orca's `orca://` scheme has no worktree or terminal route, so Orca's own CLI is used. Both
//! calls only change what the Orca window shows:
//! - `orca terminal switch --terminal <handle>` brings the task's terminal tab to the front;
//! - `orca worktree activate --worktree path:<worktree>` opens its worktree, when no terminal
//!   handle is on record.
//!
//! The handle and path come from the task's snapshot record, never from the window, are checked
//! for shape, and are used only when the task's backend is `orca`. No `--environment` or
//! pairing code is ever passed, so only the local Orca is addressed. The launch itself goes
//! through the core's external-launch allow-list ([`crate::external::External::OrcaFocus`]).

use std::path::Path;

use crate::engine::HomeState;
use crate::snapshot::Task;

/// Where Orca installs its CLI.
pub const ORCA_BIN: &str = "/Applications/Orca.app/Contents/Resources/bin/orca";

/// What "Open in Orca" brings forward for a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrcaTarget {
    Terminal(String),
    Worktree(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrcaError {
    #[error("{0:?} is not a task in the current snapshot")]
    UnknownTask(String),
    #[error("task {0} cannot be opened in Orca")]
    NotInOrca(String),
}

/// `term_` followed by letters, digits, `-` or `_`.
pub fn valid_terminal(s: &str) -> bool {
    s.strip_prefix("term_").is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 128
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    })
}

/// An absolute, printable path of bounded length with no `..` component.
fn valid_worktree(p: &Path) -> Option<String> {
    let s = p.to_str()?;
    let ok = p.is_absolute()
        && s.len() <= 1024
        && !s.chars().any(char::is_control)
        && !p
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
    ok.then(|| s.to_string())
}

/// The target for `task`: its terminal when one is on record, else its worktree.
pub fn target_for(task: &Task) -> Option<OrcaTarget> {
    if task.backend.as_deref() != Some("orca") {
        return None;
    }
    if let Some(t) = task
        .endpoint_target
        .as_deref()
        .filter(|t| valid_terminal(t))
    {
        return Some(OrcaTarget::Terminal(t.to_string()));
    }
    task.worktree
        .as_deref()
        .and_then(valid_worktree)
        .map(OrcaTarget::Worktree)
}

impl OrcaTarget {
    /// The argument vector for the Orca CLI, after checking the target's shape again (the
    /// enum is public, so a value may not have come from [`target_for`]).
    pub fn args(&self) -> Option<Vec<String>> {
        let ok = match self {
            Self::Terminal(h) => valid_terminal(h),
            Self::Worktree(p) => valid_worktree(Path::new(p)).is_some(),
        };
        ok.then(|| match self {
            Self::Terminal(handle) => vec![
                "terminal".into(),
                "switch".into(),
                "--terminal".into(),
                handle.clone(),
                "--json".into(),
            ],
            Self::Worktree(path) => vec![
                "worktree".into(),
                "activate".into(),
                "--worktree".into(),
                format!("path:{path}"),
                "--json".into(),
            ],
        })
    }
}

/// Resolves task `id` in `state` to an Orca target.
pub fn resolve(state: &HomeState, id: &str) -> Result<OrcaTarget, OrcaError> {
    let task = super::find_task(state, id).ok_or_else(|| OrcaError::UnknownTask(id.to_string()))?;
    target_for(task).ok_or_else(|| OrcaError::NotInOrca(id.to_string()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::snapshot::Snapshot;

    fn task(backend: &str, target: Option<&str>, worktree: Option<&str>) -> Task {
        Task {
            id: "t1".into(),
            backend: Some(backend.into()),
            endpoint_target: target.map(Into::into),
            worktree: worktree.map(PathBuf::from),
            ..Task::default()
        }
    }

    #[test]
    fn prefers_the_terminal_and_passes_the_handle_alone() {
        let t = target_for(&task("orca", Some("term_ab-12_c"), Some("/p/wt/t1"))).unwrap();
        let args = t.args().unwrap();
        assert_eq!(
            args,
            ["terminal", "switch", "--terminal", "term_ab-12_c", "--json"]
        );
        assert!(!args
            .iter()
            .any(|a| a.contains("environment") || a.contains("pairing")));
    }

    #[test]
    fn falls_back_to_the_worktree_path_selector() {
        let t = target_for(&task("orca", None, Some("/p/wt/t1"))).unwrap();
        assert_eq!(
            t.args().unwrap(),
            [
                "worktree",
                "activate",
                "--worktree",
                "path:/p/wt/t1",
                "--json"
            ]
        );
        // A hand-built target is checked again before it becomes argv.
        assert_eq!(OrcaTarget::Terminal("--environment".into()).args(), None);
        assert_eq!(OrcaTarget::Worktree("rel/x".into()).args(), None);
        // A malformed handle is ignored, not passed.
        let t = target_for(&task("orca", Some("--environment"), Some("/p/wt/t1"))).unwrap();
        assert_eq!(t, OrcaTarget::Worktree("/p/wt/t1".into()));
    }

    #[test]
    fn refuses_other_backends_and_bad_shapes() {
        assert_eq!(
            target_for(&task("tmux", Some("term_a"), Some("/p/w"))),
            None
        );
        for bad in ["term_", "term_a b", "term_a;rm", "xterm_a"] {
            assert!(!valid_terminal(bad), "{bad}");
        }
        for bad in ["relative/wt", "/p/../etc", "/p/a\nb"] {
            assert_eq!(target_for(&task("orca", None, Some(bad))), None, "{bad}");
        }
    }

    #[test]
    fn resolves_only_tasks_in_the_snapshot() {
        let s = HomeState {
            seq: 1,
            snapshot: Some(Arc::new(Snapshot {
                tasks: vec![
                    task("orca", Some("term_x"), None),
                    Task {
                        id: "plain".into(),
                        ..Task::default()
                    },
                ],
                ..Snapshot::default()
            })),
            ..HomeState::default()
        };
        assert_eq!(resolve(&s, "t1"), Ok(OrcaTarget::Terminal("term_x".into())));
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
