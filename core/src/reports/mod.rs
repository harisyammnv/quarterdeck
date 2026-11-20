//! Worker reports and status history, read on demand.
//!
//! A report is `data/<task>/report.md`: untrusted text, read through [`safe_read`] so it can
//! never leave the home's `data` directory, bounded in size, and rendered by the UI through
//! the shared sanitizing Markdown component. The status history comes from the engine's own
//! bounded reader, [`status::history`].

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::engine::HomeState;
use crate::gateway::{TaskId, TaskIdError};
use crate::home::FmHome;
use crate::safe_read::{self, ReadError};
use crate::snapshot::Snapshot;
use crate::status::{self, StatusEvent, HISTORY_MAX_EVENTS};

/// The most bytes of a report shown; a longer one is cut at a line and marked.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// A report as read from disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReportDoc {
    pub task_id: String,
    /// The Markdown source, untrusted. Render it only through the shared Markdown component.
    pub markdown: String,
    /// True when the file was longer than [`MAX_BYTES`] and only its start is here.
    pub truncated: bool,
    /// Last modified, in Unix seconds, when the file system says.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub modified: Option<i64>,
}

/// A task's recent status events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReportHistory {
    pub task_id: String,
    /// Newest first, at most [`HISTORY_MAX_EVENTS`].
    pub events: Vec<StatusEvent>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("not a task id")]
    BadId,
    #[error("no task {0} in the current fleet")]
    UnknownTask(String),
    #[error("the report is a link, which is not followed")]
    Link,
    #[error("the report is outside the home's data directory")]
    Outside,
    #[error("the report is not a regular file")]
    NotFile,
    #[error("the report could not be read: {0}")]
    Read(String),
    #[error("the status record could not be read: {0}")]
    History(String),
}

/// Checks that `id` is a safe id the current snapshot knows, as a task or a backlog record.
/// A finished task can leave `tasks[]` while its record and report remain.
fn known(state: &HomeState, id: &str) -> Result<TaskId, ReportError> {
    let snapshot = state.snapshot.as_deref();
    TaskId::known(id, |id| {
        snapshot.is_some_and(|s| s.task(id).is_some() || s.record(id).is_some())
    })
    .map_err(|e| match e {
        TaskIdError::Unknown(id) => ReportError::UnknownTask(id),
        TaskIdError::BadId => ReportError::BadId,
    })
}

/// Where the report of `id` lives, relative to the home's `data` directory.
fn report_rel(id: &TaskId) -> PathBuf {
    Path::new(id.as_str()).join("report.md")
}

/// Reads the report of task `id`. `Ok(None)` means the task has no report yet.
pub async fn read(
    home: &FmHome,
    state: &HomeState,
    id: &str,
) -> Result<Option<ReportDoc>, ReportError> {
    let id = known(state, id)?;
    read_report(&home.root().join("data"), &id).await
}

/// Reads `<data>/<id>/report.md`.
pub async fn read_report(data: &Path, id: &TaskId) -> Result<Option<ReportDoc>, ReportError> {
    let rel = report_rel(id);
    // safe_read keeps the read under `data` after links resolve; a report should never be a
    // link at all, so refuse one outright, whether the task folder or the file.
    for p in [data.join(id.as_str()), data.join(&rel)] {
        match tokio::fs::symlink_metadata(&p).await {
            Ok(m) if m.file_type().is_symlink() => return Err(ReportError::Link),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ReportError::Read(e.kind().to_string())),
        }
    }
    // safe_read proves the file stays under `data`, then reads it from a no-follow handle it
    // checks is the very file it resolved, so a swap after the check is refused.
    let capped = match safe_read::read_text_capped(data, &rel, MAX_BYTES).await {
        Ok(c) => c,
        Err(ReadError::NotFound) => return Ok(None),
        Err(ReadError::Escapes) => return Err(ReportError::Outside),
        Err(ReadError::NotAFile) => return Err(ReportError::NotFile),
        Err(e) => return Err(ReportError::Read(e.to_string())),
    };
    let truncated = capped.truncated;
    let mut markdown = capped.content;
    if truncated {
        // Cut on a line so a half-read table row or list item is not shown.
        if let Some(nl) = markdown.rfind('\n') {
            markdown.truncate(nl + 1);
        }
    }
    // For the "Updated" label only; the content above came from the verified handle.
    let modified = tokio::fs::metadata(data.join(&rel))
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok());
    Ok(Some(ReportDoc {
        task_id: id.to_string(),
        markdown,
        truncated,
        modified,
    }))
}

/// The most reports [`list`] returns, newest first.
pub const MAX_LISTED: usize = 200;

/// One report in the cross-task list. Ids and labels only; no path reaches the UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReportItem {
    pub task_id: String,
    /// The task's title from the backlog or the task row, when the snapshot has one.
    pub title: Option<String>,
    /// The report's kind from the snapshot, such as `scout`.
    pub kind: Option<String>,
    /// Last modified, in Unix seconds, when the file system says.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub modified: Option<i64>,
}

/// The reports the snapshot lists, newest first, at most [`MAX_LISTED`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReportsList {
    pub items: Vec<ReportItem>,
    /// True when more reports exist than are listed.
    pub truncated: bool,
}

/// The snapshot's `scout_reports`, as items without times: safe ids only, each id once,
/// titled from the backlog record or the task row. Pure, so it is tested without a home.
pub fn candidates(snapshot: &Snapshot) -> Vec<ReportItem> {
    let mut seen = std::collections::HashSet::new();
    snapshot
        .scout_reports
        .iter()
        .filter(|r| TaskId::parse(&r.id).is_ok() && seen.insert(r.id.clone()))
        .map(|r| ReportItem {
            task_id: r.id.clone(),
            title: snapshot
                .record(&r.id)
                .and_then(|b| b.title.clone())
                .or_else(|| snapshot.task(&r.id).and_then(|t| t.title.clone())),
            kind: r.kind.clone(),
            modified: None,
        })
        .collect()
}

/// Lists the reports the current snapshot names. Each must exist as a regular file at
/// `data/<id>/report.md` inside the home's `data` directory; one that does not, or that
/// leaves the directory, is left out. Nothing is read but the file's metadata.
pub async fn list(home: &FmHome, state: &HomeState) -> ReportsList {
    let Some(snapshot) = state.snapshot.as_deref() else {
        return ReportsList::default();
    };
    let data = home.root().join("data");
    let mut items = Vec::new();
    for mut item in candidates(snapshot) {
        let rel = Path::new(&item.task_id).join("report.md");
        let Ok(real) = safe_read::resolve_under(&data, &rel).await else {
            continue;
        };
        item.modified = tokio::fs::metadata(&real)
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_secs()).ok());
        items.push(item);
    }
    items.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    let truncated = items.len() > MAX_LISTED;
    items.truncate(MAX_LISTED);
    ReportsList { items, truncated }
}

/// The status history of task `id`, newest first.
pub async fn history(
    home: &FmHome,
    state: &HomeState,
    id: &str,
) -> Result<ReportHistory, ReportError> {
    let id = known(state, id)?;
    let mut events = status::history(home, id.as_str(), HISTORY_MAX_EVENTS)
        .await
        .map_err(|e| ReportError::History(e.to_string()))?;
    events.reverse();
    Ok(ReportHistory {
        task_id: id.to_string(),
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn id() -> TaskId {
        TaskId::parse("app-login-fix").unwrap()
    }

    fn task_dir(d: &tempfile::TempDir) -> PathBuf {
        let p = d.path().join("app-login-fix");
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn none_when_absent() {
        let d = data();
        assert!(read_report(d.path(), &id()).await.unwrap().is_none());
        task_dir(&d);
        assert!(read_report(d.path(), &id()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reads_a_report() {
        let d = data();
        std::fs::write(task_dir(&d).join("report.md"), "# Done\n\nAll green.\n").unwrap();
        let r = read_report(d.path(), &id()).await.unwrap().unwrap();
        assert_eq!(r.markdown, "# Done\n\nAll green.\n");
        assert_eq!(r.task_id, "app-login-fix");
        assert!(!r.truncated);
        assert!(r.modified.is_some());
    }

    #[tokio::test]
    async fn cuts_a_long_report_on_a_line() {
        let d = data();
        let line = format!("{}\n", "y".repeat(99));
        std::fs::write(task_dir(&d).join("report.md"), line.repeat(20_000)).unwrap();
        let r = read_report(d.path(), &id()).await.unwrap().unwrap();
        assert!(r.truncated);
        assert!(r.markdown.len() as u64 <= MAX_BYTES);
        assert!(r.markdown.ends_with('\n'));
    }

    #[tokio::test]
    async fn refuses_links_even_inside_the_data_directory() {
        let d = data();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("report.md"), "secret").unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join("app-login-fix")).unwrap();
        assert!(matches!(
            read_report(d.path(), &id()).await,
            Err(ReportError::Link)
        ));
        std::fs::remove_file(d.path().join("app-login-fix")).unwrap();
        std::fs::create_dir_all(d.path().join("other")).unwrap();
        std::fs::write(d.path().join("other/report.md"), "x").unwrap();
        std::os::unix::fs::symlink(
            d.path().join("other/report.md"),
            task_dir(&d).join("report.md"),
        )
        .unwrap();
        assert!(matches!(
            read_report(d.path(), &id()).await,
            Err(ReportError::Link)
        ));
    }

    #[tokio::test]
    async fn refuses_a_fifo_without_blocking() {
        let d = data();
        let fifo = task_dir(&d).join("report.md");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(made.success());
        assert!(matches!(
            read_report(d.path(), &id()).await,
            Err(ReportError::NotFile)
        ));
    }

    #[test]
    fn candidates_keep_safe_unique_ids_with_titles() {
        let json = serde_json::json!({
            "schema": "fm-fleet-snapshot.v1",
            "tasks": [{ "id": "task-b", "backlog": { "title": "From the task" } }],
            "backlog": { "records": [{ "id": "task-a", "title": "From the backlog" }] },
            "scout_reports": [
                { "id": "task-a", "kind": "scout", "path": "/p/home/data/task-a/report.md" },
                { "id": "task-b", "kind": "ship" },
                { "id": "task-a", "kind": "scout" },
                { "id": "../state", "kind": "scout" },
                { "id": "-x", "kind": "scout" },
            ],
        });
        let s = Snapshot::parse(&json.to_string()).unwrap();
        let c = candidates(&s);
        let ids: Vec<_> = c.iter().map(|i| i.task_id.as_str()).collect();
        assert_eq!(ids, ["task-a", "task-b"]);
        assert_eq!(c[0].title.as_deref(), Some("From the backlog"));
        assert_eq!(c[1].title.as_deref(), Some("From the task"));
        assert_eq!(c[1].kind.as_deref(), Some("ship"));
        assert_eq!(c[0].modified, None);
    }

    #[tokio::test]
    async fn refuses_a_directory() {
        let d = data();
        std::fs::create_dir_all(task_dir(&d).join("report.md")).unwrap();
        assert!(matches!(
            read_report(d.path(), &id()).await,
            Err(ReportError::NotFile)
        ));
    }
}
