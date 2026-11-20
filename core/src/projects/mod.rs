//! The projects sidebar and the inspector beside it (design 5.1).
//!
//! - [`view`] projects the published home state into projects with their worktrees: a pure
//!   function with no I/O and no clock, re-run on every state change.
//! - [`icon`] finds a project's icon among image files its clone already contains.
//! - [`orca`] builds the Orca CLI call behind "Open in Orca" from the task's own record.
//!
//! The window sends task ids and project names only; paths, terminal handles and argv stay in
//! the core.

pub mod icon;
pub mod orca;

use std::collections::BTreeMap;

use serde::Serialize;

use crate::engine::HomeState;
use crate::fleet::StateSource;
use crate::snapshot::Task;
use crate::status::{StatusEvent, TaskState};
pub use icon::{IconCache, IconError};
pub use orca::{OrcaError, TerminalHandle};

/// Everything the sidebar and the inspector's task facts need.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProjectsView {
    /// A snapshot has been read; until then the list is empty because it is not known yet.
    pub ready: bool,
    /// Projects by name, then the tasks with no project as a last group with no name.
    pub projects: Vec<ProjectGroup>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProjectGroup {
    /// The project's directory name, never its path. `None` groups tasks without a project.
    pub name: Option<String>,
    /// The project's worktrees (tasks), by id.
    pub tasks: Vec<ProjectRow>,
}

/// One worktree row, and the facts the inspector shows when it is selected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProjectRow {
    pub id: String,
    pub title: Option<String>,
    /// `ship`, `scout` or `secondmate` as written.
    pub kind: Option<String>,
    pub state: TaskState,
    /// The state word as written, so unknown words are still shown honestly.
    pub state_word: String,
    pub source: StateSource,
    /// The agent harness, such as `claude` or `codex`.
    pub harness: Option<String>,
    pub branch: Option<String>,
    /// The delivery mode as written, such as `direct-PR`.
    pub mode: Option<String>,
    /// When the worker was started, Unix seconds, from the spawn stamp.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub started_at: Option<i64>,
    pub latest_event: Option<StatusEvent>,
    /// "Open in Orca" can bring this task forward.
    pub in_orca: bool,
}

/// Projects the home state into the sidebar view. Pure: no I/O, no clock.
pub fn view(state: &HomeState) -> ProjectsView {
    let Some(snap) = state.snapshot.as_deref() else {
        return ProjectsView::default();
    };
    let mut groups: BTreeMap<Option<String>, Vec<ProjectRow>> = BTreeMap::new();
    for task in &snap.tasks {
        let row = row(task, state.events.get(&task.id), snap.generated);
        groups.entry(task.project_name()).or_default().push(row);
    }
    let mut projects: Vec<ProjectGroup> = groups
        .into_iter()
        .map(|(name, mut tasks)| {
            tasks.sort_by(|a, b| a.id.cmp(&b.id));
            ProjectGroup { name, tasks }
        })
        .collect();
    // BTreeMap puts `None` first; the unnamed group belongs last.
    projects.sort_by_key(|g| g.name.is_none());
    ProjectsView {
        ready: true,
        projects,
    }
}

/// Main's rule (`fleet.rs`): an event wins when it is newer than the snapshot, or when the
/// snapshot could not tell.
fn row(task: &Task, event: Option<&StatusEvent>, generated: Option<i64>) -> ProjectRow {
    let word = task
        .state
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let snapshot_knows = TaskState::from_word(&word) != TaskState::Unknown;
    let newer = |e: &StatusEvent| match (generated, e.at) {
        (Some(g), Some(at)) => at > g,
        (None, _) => true,
        (Some(_), None) => false,
    };
    let (state_word, source) = match event {
        Some(e) if newer(e) || !snapshot_knows => (e.word.clone(), StateSource::Event),
        _ => (word, StateSource::Snapshot),
    };
    ProjectRow {
        id: task.id.clone(),
        title: task.title.clone().filter(|t| !t.is_empty()),
        kind: task.kind.clone(),
        state: TaskState::from_word(&state_word),
        state_word,
        source,
        harness: task.harness.clone(),
        branch: task.branch.clone(),
        mode: task.mode.clone(),
        started_at: task.spawn_gen.as_deref().and_then(spawn_epoch),
        latest_event: event.cloned(),
        in_orca: orca::handle_for(task).is_some(),
    }
}

/// `s1795176247.90164.19766` -> 1795176247.
fn spawn_epoch(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('s')?.split('.').next()?;
    digits.parse::<i64>().ok().filter(|&t| t > 0)
}

/// The task with `id` in the current snapshot, after checking the id's shape.
pub fn find_task<'a>(state: &'a HomeState, id: &str) -> Option<&'a Task> {
    if !crate::gateway::valid_task_id(id) {
        return None;
    }
    state.snapshot.as_deref()?.tasks.iter().find(|t| t.id == id)
}

/// Whether `s` is safe as one file-name component (a project name).
pub fn is_plain_component(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s != "."
        && s != ".."
        && !s.chars().any(|c| c == '/' || c == '\\' || c.is_control())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::snapshot::Snapshot;
    use crate::status::parse_line;

    const GEN: i64 = 1_000_000;

    pub(crate) fn task(id: &str, project: Option<&str>, state: &str) -> Task {
        Task {
            id: id.into(),
            kind: Some("ship".into()),
            mode: Some("direct-PR".into()),
            harness: Some("claude".into()),
            backend: Some("orca".into()),
            branch: Some(format!("fm/{id}")),
            project: project.map(|p| PathBuf::from(format!("/p/{p}"))),
            state: Some(state.into()),
            worktree: Some(PathBuf::from(format!("/p/wt/{id}"))),
            endpoint_target: Some(format!("term_{id}")),
            title: Some(format!("Title of {id}")),
            spawn_gen: Some("s999000.1.2".into()),
            ..Task::default()
        }
    }

    fn state(tasks: Vec<Task>, events: &[(&str, &str)]) -> HomeState {
        HomeState {
            seq: 1,
            snapshot: Some(Arc::new(Snapshot {
                generated: Some(GEN),
                tasks,
                ..Snapshot::default()
            })),
            info: Default::default(),
            events: events
                .iter()
                .map(|(id, line)| ((*id).to_string(), parse_line(line).unwrap()))
                .collect(),
        }
    }

    #[test]
    fn groups_by_project_name_sorted_with_unassigned_last() {
        let s = state(
            vec![
                task("b-two", Some("portal-web"), "working"),
                task("loose", None, "working"),
                task("a-one", Some("portal-web"), "blocked"),
                task("pin", Some("example-app"), "done"),
            ],
            &[],
        );
        let v = view(&s);
        assert!(v.ready);
        let names: Vec<_> = v.projects.iter().map(|g| g.name.clone()).collect();
        assert_eq!(
            names,
            [Some("example-app".into()), Some("portal-web".into()), None]
        );
        let ids: Vec<_> = v.projects[1].tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["a-one", "b-two"]);
        let r = &v.projects[1].tasks[0];
        assert_eq!(r.state, TaskState::Blocked);
        assert_eq!(r.started_at, Some(999_000));
        assert_eq!(r.branch.as_deref(), Some("fm/a-one"));
        assert!(r.in_orca);
    }

    #[test]
    fn view_of_the_snapshot_fixture() {
        let snap =
            Snapshot::parse(include_str!("../../tests/fixtures/fleet-snapshot.json")).unwrap();
        let s = HomeState {
            snapshot: Some(Arc::new(snap)),
            ..HomeState::default()
        };
        let v = view(&s);
        assert!(v.ready);
        let rows: Vec<_> = v.projects.iter().flat_map(|g| &g.tasks).collect();
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.started_at == Some(1_795_160_000)));
        // Only directory names cross IPC: no group name is a path.
        assert!(v.projects.iter().flatten_names().all(|n| !n.contains('/')));
    }

    trait Names<'a> {
        fn flatten_names(self) -> Box<dyn Iterator<Item = &'a String> + 'a>;
    }
    impl<'a, I: Iterator<Item = &'a ProjectGroup> + 'a> Names<'a> for I {
        fn flatten_names(self) -> Box<dyn Iterator<Item = &'a String> + 'a> {
            Box::new(self.filter_map(|g| g.name.as_ref()))
        }
    }

    #[test]
    fn no_snapshot_is_not_ready() {
        assert_eq!(view(&HomeState::default()), ProjectsView::default());
    }

    #[test]
    fn event_merge_follows_mains_rule() {
        let s = state(
            vec![
                task("old", Some("x"), "working"),
                task("new", Some("x"), "working"),
                task("unk", Some("x"), "unknown"),
            ],
            &[
                ("old", "blocked [at=999999]: stale"),
                ("new", "needs-decision [at=1000001]: A or B"),
                ("unk", "paused [at=5]: waiting"),
            ],
        );
        let v = view(&s);
        let by_id = |id: &str| {
            v.projects[0]
                .tasks
                .iter()
                .find(|t| t.id == id)
                .unwrap()
                .clone()
        };
        let old = by_id("old");
        assert_eq!(
            (old.state, old.source),
            (TaskState::Working, StateSource::Snapshot)
        );
        assert_eq!(old.latest_event.unwrap().text, "stale");
        let new = by_id("new");
        assert_eq!(
            (new.state, new.source),
            (TaskState::NeedsDecision, StateSource::Event)
        );
        let unk = by_id("unk");
        assert_eq!(
            (unk.state, unk.source),
            (TaskState::Paused, StateSource::Event)
        );
    }

    #[test]
    fn missing_optional_fields_are_tolerated() {
        let bare = Task {
            id: "bare".into(),
            ..Task::default()
        };
        let v = view(&state(vec![bare], &[]));
        let r = &v.projects[0].tasks[0];
        assert_eq!(v.projects[0].name, None);
        assert_eq!(
            (r.state, r.state_word.as_str()),
            (TaskState::Unknown, "unknown")
        );
        assert_eq!(
            (r.title.clone(), r.started_at, r.in_orca),
            (None, None, false)
        );
    }

    #[test]
    fn spawn_stamps() {
        assert_eq!(spawn_epoch("s1795176247.90164.19766"), Some(1_795_176_247));
        assert_eq!(spawn_epoch("g12.3"), None);
        assert_eq!(spawn_epoch("s.1"), None);
    }

    #[test]
    fn ids_and_names() {
        for ok in ["example-app", "erp dashboard"] {
            assert!(is_plain_component(ok));
        }
        for bad in ["", ".", "..", "a/b", "a\\b", "x\ny"] {
            assert!(!is_plain_component(bad), "{bad:?}");
        }
        let s = state(vec![task("t1", Some("x"), "working")], &[]);
        assert!(find_task(&s, "t1").is_some());
        assert!(find_task(&s, "t2").is_none());
        assert!(find_task(&s, "../t1").is_none());
    }

    #[test]
    fn view_is_fast_for_a_large_fleet() {
        let tasks: Vec<_> = (0..500)
            .map(|i| task(&format!("t{i}"), Some(&format!("p{}", i % 40)), "working"))
            .collect();
        let events: Vec<(String, String)> = (0..500)
            .map(|i| (format!("t{i}"), format!("working [at={}]: e{i}", GEN + 1)))
            .collect();
        let refs: Vec<(&str, &str)> = events
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let s = state(tasks, &refs);
        let start = std::time::Instant::now();
        let v = view(&s);
        let took = start.elapsed();
        assert_eq!(v.projects.len(), 40);
        // About 5 ms in a release build; the contract's loose 250 ms bound keeps debug CI from
        // flaking and still catches quadratic work.
        assert!(took.as_millis() < 250, "{took:?}");
    }
}
