//! The fleet snapshot: `bin/fm-fleet-snapshot.sh --json`, schema `fm-fleet-snapshot.v1`.
//!
//! This is the engine's "true" read and the one source of structured truth: tasks, the
//! backlog with each decision's bucket and `captain_actionable`, scout reports and second
//! mates. It is parsed once here, in Rust, and never crosses IPC whole; each area sends its
//! own compact projection of it.
//!
//! The parser is tolerant: every field is optional or defaulted, unknown fields are ignored,
//! and only the schema string is checked, so Firstmate can grow the snapshot without breaking
//! the app. A slice that needs another field adds one `#[serde(default)]` line to [`Task`] or
//! [`BacklogRecord`] (and its raw counterpart below).

use std::path::PathBuf;

use serde::Deserialize;

use crate::timefmt::rfc3339_to_epoch;

/// The schema this parser was written against.
pub const SCHEMA: &str = "fm-fleet-snapshot.v1";

/// The home's directories as the snapshot names them. Never sent to the UI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    pub state: Option<PathBuf>,
    pub data: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub projects: Option<PathBuf>,
}

/// One task row from `tasks[]`, flattened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub kind: Option<String>,
    pub mode: Option<String>,
    pub harness: Option<String>,
    pub backend: Option<String>,
    pub branch: Option<String>,
    /// The project clone, an absolute path. Never sent to the UI; send its basename.
    pub project: Option<PathBuf>,
    /// `current_state.state`, such as `working` or `unknown`.
    pub state: Option<String>,
    /// `current_state.detail`, a one-line summary such as "harness busy".
    pub state_detail: Option<String>,
    /// `current_state.source`, such as `pane` or `status`.
    pub state_source: Option<String>,
    /// `paths.worktree.path`. Never sent to the UI.
    pub worktree: Option<PathBuf>,
    /// `paths.report.path`, only when the snapshot says it is present. Never sent to the UI.
    pub report: Option<PathBuf>,
    /// `endpoint.target`, the runtime handle of the worker terminal.
    pub endpoint_target: Option<String>,
    /// `pr.url`, the recorded pull request.
    pub pr_url: Option<String>,
    /// `backlog.title`, the task's name.
    pub title: Option<String>,
    /// `remote` is set: the task runs on another host, so its terminal is not read from here.
    pub remote: bool,
}

impl Task {
    /// The project's directory name, the label the UI shows.
    pub fn project_name(&self) -> Option<String> {
        self.project
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
    }
}

/// One record from `backlog.records[]`. Free-form backlog lines arrive with `id: null`; they
/// are read as an empty id and dropped, so one such line never costs the whole snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct BacklogRecord {
    #[serde(default, deserialize_with = "null_as_empty")]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    /// `queued`, `in_flight` or `done`.
    #[serde(default)]
    pub state: Option<String>,
    /// The decision is waiting on a human now. "Needs your call" is exactly these.
    #[serde(default)]
    pub captain_actionable: bool,
    /// `live`, `dated`, `aged` or `blocked`, decided by Firstmate from structured fields.
    #[serde(default)]
    pub hold_bucket: Option<String>,
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub hold_until: Option<String>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub pr_url: Option<String>,
    /// Relative to the home (`data/<id>/report.md`) or absolute. Never sent to the UI.
    #[serde(default)]
    pub report_path: Option<PathBuf>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub order: Option<i64>,
}

/// One entry from `scout_reports[]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ScoutReport {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    /// Absolute path of the report. Never sent to the UI.
    #[serde(default)]
    pub path: Option<PathBuf>,
}

/// One registered second mate from `secondmate_current.records[]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Secondmate {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub doing: Option<String>,
}

/// A typed, tolerant read of one fleet snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// `generated` as Unix seconds.
    pub generated: Option<i64>,
    pub roots: Roots,
    pub tasks: Vec<Task>,
    pub backlog: Vec<BacklogRecord>,
    pub scout_reports: Vec<ScoutReport>,
    pub secondmates: Vec<Secondmate>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected snapshot schema {found:?}, expected {SCHEMA}")]
    Schema { found: String },
}

impl Snapshot {
    /// Parses snapshot JSON and checks the schema.
    pub fn parse(json: &str) -> Result<Self, SnapshotError> {
        let raw: raw::Snapshot = serde_json::from_str(json)?;
        if raw.schema != SCHEMA {
            return Err(SnapshotError::Schema { found: raw.schema });
        }
        Ok(raw.into())
    }

    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn record(&self, id: &str) -> Option<&BacklogRecord> {
        self.backlog.iter().find(|r| r.id == id)
    }

    /// Decisions waiting on a human now: backlog records marked `captain_actionable`.
    pub fn actionable_count(&self) -> usize {
        self.backlog.iter().filter(|r| r.captain_actionable).count()
    }
}

/// The wire shapes, as Firstmate writes them. Kept private so the nesting never leaks out.
mod raw {
    use std::path::PathBuf;

    use serde::Deserialize;

    use super::{BacklogRecord, ScoutReport, Secondmate};

    #[derive(Deserialize)]
    pub struct Snapshot {
        pub schema: String,
        #[serde(default)]
        pub generated: Option<String>,
        #[serde(default)]
        pub roots: Roots,
        #[serde(default)]
        pub tasks: Vec<Task>,
        #[serde(default)]
        pub backlog: Backlog,
        #[serde(default)]
        pub scout_reports: Vec<ScoutReport>,
        #[serde(default)]
        pub secondmate_current: SecondmateCurrent,
    }

    #[derive(Default, Deserialize)]
    pub struct Roots {
        #[serde(default)]
        pub state: Option<PathBuf>,
        #[serde(default)]
        pub data: Option<PathBuf>,
        #[serde(default)]
        pub config: Option<PathBuf>,
        #[serde(default)]
        pub projects: Option<PathBuf>,
    }

    #[derive(Default, Deserialize)]
    pub struct Backlog {
        #[serde(default)]
        pub records: Vec<BacklogRecord>,
    }

    #[derive(Default, Deserialize)]
    pub struct SecondmateCurrent {
        #[serde(default)]
        pub records: Vec<Secondmate>,
    }

    #[derive(Deserialize)]
    pub struct Task {
        pub id: String,
        #[serde(default)]
        pub kind: Option<String>,
        #[serde(default)]
        pub mode: Option<String>,
        #[serde(default)]
        pub harness: Option<String>,
        #[serde(default)]
        pub backend: Option<String>,
        #[serde(default)]
        pub branch: Option<String>,
        #[serde(default)]
        pub project: Option<PathBuf>,
        #[serde(default)]
        pub current_state: CurrentState,
        #[serde(default)]
        pub paths: Paths,
        #[serde(default)]
        pub endpoint: Endpoint,
        #[serde(default)]
        pub pr: Pr,
        #[serde(default)]
        pub backlog: TaskBacklog,
        #[serde(default)]
        pub remote: Option<serde_json::Value>,
    }

    #[derive(Default, Deserialize)]
    pub struct CurrentState {
        #[serde(default)]
        pub state: Option<String>,
        #[serde(default)]
        pub detail: Option<String>,
        #[serde(default)]
        pub source: Option<String>,
    }

    #[derive(Default, Deserialize)]
    pub struct Paths {
        #[serde(default)]
        pub worktree: PathEntry,
        #[serde(default)]
        pub report: PathEntry,
    }

    #[derive(Default, Deserialize)]
    pub struct PathEntry {
        #[serde(default)]
        pub path: Option<PathBuf>,
        #[serde(default)]
        pub present: bool,
    }

    #[derive(Default, Deserialize)]
    pub struct Endpoint {
        #[serde(default)]
        pub target: Option<String>,
    }

    #[derive(Default, Deserialize)]
    pub struct Pr {
        #[serde(default)]
        pub url: Option<String>,
    }

    #[derive(Default, Deserialize)]
    pub struct TaskBacklog {
        #[serde(default)]
        pub title: Option<String>,
    }
}

/// Reads a string that may be `null`, as an empty string.
fn null_as_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

impl From<raw::Snapshot> for Snapshot {
    fn from(r: raw::Snapshot) -> Self {
        Self {
            generated: r.generated.as_deref().and_then(rfc3339_to_epoch),
            roots: Roots {
                state: r.roots.state,
                data: r.roots.data,
                config: r.roots.config,
                projects: r.roots.projects,
            },
            tasks: r
                .tasks
                .into_iter()
                .map(|t| Task {
                    id: t.id,
                    kind: non_empty(t.kind),
                    mode: non_empty(t.mode),
                    harness: non_empty(t.harness),
                    backend: non_empty(t.backend),
                    branch: non_empty(t.branch),
                    project: t.project,
                    state: non_empty(t.current_state.state),
                    state_detail: non_empty(t.current_state.detail),
                    state_source: non_empty(t.current_state.source),
                    worktree: t.paths.worktree.path.filter(|_| t.paths.worktree.present),
                    report: t.paths.report.path.filter(|_| t.paths.report.present),
                    endpoint_target: non_empty(t.endpoint.target),
                    pr_url: non_empty(t.pr.url),
                    title: non_empty(t.backlog.title),
                    remote: t.remote.is_some_and(|r| !r.is_null()),
                })
                .collect(),
            backlog: r
                .backlog
                .records
                .into_iter()
                .filter(|b| !b.id.trim().is_empty())
                .collect(),
            scout_reports: r.scout_reports,
            secondmates: r.secondmate_current.records,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/fleet-snapshot.json");

    #[test]
    fn parses_the_documented_shape() {
        let s = Snapshot::parse(FIXTURE).unwrap();
        assert_eq!(s.generated, Some(1_795_168_775));
        assert_eq!(s.tasks.len(), 6);
        let t = s.task("app-login-fix").unwrap();
        assert_eq!(t.kind.as_deref(), Some("ship"));
        assert_eq!(t.state.as_deref(), Some("unknown"));
        assert_eq!(t.project_name().as_deref(), Some("example-app"));
        assert_eq!(t.backend.as_deref(), Some("orca"));
        assert_eq!(
            t.state_detail.as_deref(),
            Some("backend unreachable (orca endpoint state: unreadable)")
        );
        assert!(t.worktree.is_some());
        assert_eq!(
            t.report, None,
            "a report the snapshot marks absent is not offered"
        );
        assert_eq!(
            s.task("perf-scout").unwrap().report.as_deref(),
            Some(std::path::Path::new("/p/home/data/perf-scout/report.md"))
        );
        assert_eq!(
            s.task("web-pin-update").unwrap().pr_url.as_deref(),
            Some("https://forge.example.invalid/acme/example-web/pull/37")
        );
        assert_eq!(s.backlog.len(), 9);
        assert_eq!(s.actionable_count(), 1);
        let dated = s.record("cache-plan").unwrap();
        assert_eq!(
            (dated.hold_bucket.as_deref(), dated.hold_until.as_deref()),
            (Some("dated"), Some("2026-12-02"))
        );
        assert_eq!(s.scout_reports.len(), 2);
        assert_eq!(s.secondmates[0].id, "mate-web");
        assert_eq!(
            s.roots.data.as_deref(),
            Some(std::path::Path::new("/p/home/data"))
        );
    }

    #[test]
    fn rejects_other_schema() {
        let err = Snapshot::parse(r#"{"schema":"fm-bearings.v1"}"#).unwrap_err();
        assert!(matches!(err, SnapshotError::Schema { .. }));
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let s = Snapshot::parse(r#"{"schema":"fm-fleet-snapshot.v1","tasks":[{"id":"a","kind":""}],"backlog":{"records":[{"id":"b"}]}}"#).unwrap();
        let t = &s.tasks[0];
        assert_eq!(
            (t.kind.clone(), t.state.clone(), t.project_name()),
            (None, None, None)
        );
        assert!(!s.backlog[0].captain_actionable);
        assert_eq!(s.generated, None);
        assert!(s.scout_reports.is_empty() && s.secondmates.is_empty());
    }

    #[test]
    fn free_form_backlog_lines_with_null_ids_are_dropped_not_fatal() {
        let s = Snapshot::parse(
            r#"{"schema":"fm-fleet-snapshot.v1","backlog":{"records":[{"id":null,"structured":false,"state":"queued"},{"state":"queued"},{"id":"a","captain_actionable":true}]}}"#,
        )
        .unwrap();
        assert_eq!(s.backlog.len(), 1);
        assert_eq!(s.backlog[0].id, "a");
        assert_eq!(s.actionable_count(), 1);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(matches!(
            Snapshot::parse("{\"schema\":"),
            Err(SnapshotError::Json(_))
        ));
        assert!(matches!(
            Snapshot::parse(r#"{"schema":"fm-fleet-snapshot.v1","tasks":[{"kind":"ship"}]}"#),
            Err(SnapshotError::Json(_))
        ));
    }
}
