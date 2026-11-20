//! The fleet model: snapshot rows with live status events laid over them.
//!
//! Two speeds feed it:
//! - the fleet snapshot is the structured truth, but takes seconds to produce;
//! - status events arrive within milliseconds of a worker writing them.
//!
//! An event replaces a row's snapshot state only when it is newer than the snapshot, and such
//! a row is marked as coming from the latest event, because a status line is a wake event and
//! not current truth. The next snapshot reconciles it.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;

use crate::meta::MetaInfo;
use crate::snapshot::{Snapshot, Task};
use crate::status::{StatusEvent, TaskState};

/// Where a row's state came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum StateSource {
    /// The fleet snapshot.
    Snapshot,
    /// A status event newer than the snapshot.
    Event,
    /// Read from the state records before the first snapshot arrived.
    Bootstrap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FleetRow {
    pub id: String,
    /// The task title from the snapshot, or the id until the snapshot arrives.
    pub name: String,
    pub project: Option<String>,
    pub kind: Option<String>,
    pub state: TaskState,
    /// The state word as written, so unknown words are still shown honestly.
    pub state_word: String,
    pub source: StateSource,
    /// The snapshot's one-line summary of what the task is doing.
    pub doing: Option<String>,
    /// The last status event for this task, when there is one.
    pub latest_event: Option<StatusEvent>,
}

/// What caused a view update, for the status-change latency probe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UpdateCause {
    pub task_id: String,
    /// Modification time of the status file, in Unix milliseconds.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub file_mtime_ms: i64,
    /// When the file event reached the core, in Unix milliseconds.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub detected_ms: i64,
    /// When this view was built, in Unix milliseconds.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub emitted_ms: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SnapshotInfo {
    /// The snapshot's `generated` time, Unix seconds.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub generated_at: Option<i64>,
    /// A snapshot run is in progress.
    pub running: bool,
    /// The last run's error, cleared by the next good run.
    pub error: Option<String>,
    /// The snapshot shown came from the on-disk cache of an earlier run, not from this one.
    pub cached: bool,
}

/// Everything the UI renders.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FleetView {
    /// Increases with every published view.
    pub seq: u32,
    /// Short label of the home, such as `orca/firstmate`.
    pub home: String,
    pub rows: Vec<FleetRow>,
    pub snapshot: SnapshotInfo,
    pub cause: Option<UpdateCause>,
}

/// The mutable model behind [`FleetView`].
#[derive(Debug, Default)]
pub struct FleetModel {
    home: String,
    snapshot: Option<Arc<Snapshot>>,
    cached: bool,
    events: BTreeMap<String, StatusEvent>,
    metas: BTreeMap<String, MetaInfo>,
    running: bool,
    error: Option<String>,
    seq: u32,
}

impl FleetModel {
    pub fn new(home_label: impl Into<String>) -> Self {
        Self {
            home: home_label.into(),
            ..Self::default()
        }
    }

    /// Records the latest event of a task. Returns whether it changed.
    pub fn set_event(&mut self, id: &str, event: Option<StatusEvent>) -> bool {
        match event {
            Some(e) => self.events.insert(id.to_string(), e.clone()).as_ref() != Some(&e),
            None => self.events.remove(id).is_some(),
        }
    }

    /// Records a task's meta facts. Returns whether they changed.
    pub fn set_meta(&mut self, id: &str, meta: Option<MetaInfo>) -> bool {
        match meta {
            Some(m) => self.metas.insert(id.to_string(), m.clone()).as_ref() != Some(&m),
            None => self.metas.remove(id).is_some(),
        }
    }

    /// Takes a freshly run snapshot as the truth and clears the last error.
    pub fn apply_snapshot(&mut self, s: Arc<Snapshot>) {
        self.snapshot = Some(s);
        self.cached = false;
        self.error = None;
    }

    /// Shows a snapshot from the on-disk cache until the first run of this session lands.
    pub fn apply_cached(&mut self, s: Arc<Snapshot>) {
        if self.snapshot.is_none() {
            self.snapshot = Some(s);
            self.cached = true;
        }
    }

    /// The current snapshot, when one has landed.
    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        self.snapshot.clone()
    }

    /// The latest status event of every task, from the fast path.
    pub fn events(&self) -> &BTreeMap<String, StatusEvent> {
        &self.events
    }

    pub fn info(&self) -> SnapshotInfo {
        SnapshotInfo {
            generated_at: self.snapshot.as_ref().and_then(|s| s.generated),
            running: self.running,
            error: self.error.clone(),
            cached: self.cached,
        }
    }

    /// The number of the last published view.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    pub fn snapshot_failed(&mut self, error: String) {
        self.error = Some(error);
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn newer_than_snapshot(&self, event: &StatusEvent) -> bool {
        match (self.snapshot.as_ref().and_then(|s| s.generated), event.at) {
            (Some(generated), Some(at)) => at > generated,
            // Without a usable stamp on the snapshot, a live event is the better guess.
            (None, _) => true,
            (Some(_), None) => false,
        }
    }

    fn row_from_snapshot(&self, r: &Task) -> FleetRow {
        let event = self.events.get(&r.id);
        let word = r.state.clone().unwrap_or_else(|| "unknown".into());
        // An event wins when it is newer than the snapshot, or when the snapshot could not tell
        // (`unknown`, for example while a runtime backend is unreachable): then the latest
        // event is the best evidence there is, and the row says where it came from.
        let snapshot_knows = TaskState::from_word(&word) != TaskState::Unknown;
        let (state_word, source) = match event {
            Some(e) if self.newer_than_snapshot(e) || !snapshot_knows => {
                (e.word.clone(), StateSource::Event)
            }
            _ => (word, StateSource::Snapshot),
        };
        FleetRow {
            id: r.id.clone(),
            name: r.title.clone().unwrap_or_else(|| r.id.clone()),
            project: r.project_name(),
            kind: r.kind.clone(),
            state: TaskState::from_word(&state_word),
            state_word,
            source,
            doing: r.state_detail.clone(),
            latest_event: event.cloned(),
        }
    }

    fn row_from_records(&self, id: &str, source: StateSource) -> FleetRow {
        let meta = self.metas.get(id);
        let event = self.events.get(id);
        let state_word = event.map_or_else(|| "unknown".to_string(), |e| e.word.clone());
        FleetRow {
            id: id.to_string(),
            name: id.to_string(),
            project: meta.and_then(|m| m.project.clone()),
            kind: meta.and_then(|m| m.kind.clone()),
            state: TaskState::from_word(&state_word),
            state_word,
            source,
            doing: None,
            latest_event: event.cloned(),
        }
    }

    fn rows(&self) -> Vec<FleetRow> {
        let mut rows: Vec<FleetRow> = match &self.snapshot {
            Some(snap) => {
                let mut rows: Vec<_> = snap
                    .tasks
                    .iter()
                    .map(|r| self.row_from_snapshot(r))
                    .collect();
                // A task that started after the snapshot shows up from its own records.
                for (id, event) in &self.events {
                    let known = snap.tasks.iter().any(|r| &r.id == id);
                    if !known && self.metas.contains_key(id) && self.newer_than_snapshot(event) {
                        rows.push(self.row_from_records(id, StateSource::Event));
                    }
                }
                rows
            }
            None => self
                .metas
                .keys()
                .map(|id| self.row_from_records(id, StateSource::Bootstrap))
                .collect(),
        };
        // Stable order, so a status change repaints a row in place instead of moving it.
        rows.sort_by(|a, b| a.project.cmp(&b.project).then_with(|| a.id.cmp(&b.id)));
        rows
    }

    /// Builds the next view.
    pub fn view(&mut self, cause: Option<UpdateCause>) -> FleetView {
        self.seq = self.seq.wrapping_add(1);
        FleetView {
            seq: self.seq,
            home: self.home.clone(),
            rows: self.rows(),
            snapshot: self.info(),
            cause,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::parse_line;

    const FIXTURE: &str = include_str!("../../tests/fixtures/fleet-snapshot.json");
    // The fixture's `generated` time.
    const GEN: i64 = 1_795_168_775;

    fn model_with_snapshot() -> FleetModel {
        let mut m = FleetModel::new("home");
        m.apply_snapshot(Arc::new(Snapshot::parse(FIXTURE).unwrap()));
        m
    }

    fn ev(line: &str) -> Option<StatusEvent> {
        parse_line(line)
    }

    #[test]
    fn snapshot_rows_are_sorted_by_project_then_id() {
        let v = model_with_snapshot().view(None);
        let keys: Vec<_> = v
            .rows
            .iter()
            .map(|r| (r.project.clone().unwrap(), r.id.clone()))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert_eq!(v.rows.len(), 6);
        assert!(v.rows.iter().all(|r| r.source == StateSource::Snapshot));
    }

    #[test]
    fn older_event_does_not_override_a_known_snapshot_state() {
        let mut m = model_with_snapshot();
        m.set_event("qd-spike", ev(&format!("blocked [at={}]: x", GEN - 30)));
        let row = m
            .view(None)
            .rows
            .into_iter()
            .find(|r| r.id == "qd-spike")
            .unwrap();
        assert_eq!(row.state, TaskState::Working);
        assert_eq!(row.source, StateSource::Snapshot);
        assert_eq!(row.latest_event.unwrap().text, "x");
    }

    #[test]
    fn unknown_snapshot_state_falls_back_to_the_latest_event() {
        let mut m = model_with_snapshot();
        let row = m
            .view(None)
            .rows
            .into_iter()
            .find(|r| r.id == "app-login-fix")
            .unwrap();
        assert_eq!(
            (row.state, row.source),
            (TaskState::Unknown, StateSource::Snapshot)
        );
        m.set_event(
            "app-login-fix",
            ev(&format!("working [at={}]: x", GEN - 30)),
        );
        let row = m
            .view(None)
            .rows
            .into_iter()
            .find(|r| r.id == "app-login-fix")
            .unwrap();
        assert_eq!(
            (row.state, row.source),
            (TaskState::Working, StateSource::Event)
        );
    }

    #[test]
    fn newer_event_overrides_snapshot() {
        let mut m = model_with_snapshot();
        assert!(m.set_event(
            "app-login-fix",
            ev(&format!("blocked [at={}]: need creds", GEN + 1))
        ));
        let row = m
            .view(None)
            .rows
            .into_iter()
            .find(|r| r.id == "app-login-fix")
            .unwrap();
        assert_eq!(row.state, TaskState::Blocked);
        assert_eq!(row.source, StateSource::Event);
    }

    #[test]
    fn same_event_twice_is_not_a_change() {
        let mut m = model_with_snapshot();
        assert!(m.set_event("a", ev("working [at=1]: x")));
        assert!(!m.set_event("a", ev("working [at=1]: x")));
        assert!(m.set_event("a", None));
        assert!(!m.set_event("a", None));
    }

    #[test]
    fn new_task_after_snapshot_appears_from_records() {
        let mut m = model_with_snapshot();
        m.set_meta(
            "fresh",
            Some(MetaInfo {
                kind: Some("scout".into()),
                project: Some("zz".into()),
            }),
        );
        m.set_event("fresh", ev(&format!("working [at={}]: started", GEN + 5)));
        let v = m.view(None);
        let row = v.rows.iter().find(|r| r.id == "fresh").unwrap();
        assert_eq!(
            (row.kind.as_deref(), row.source),
            (Some("scout"), StateSource::Event)
        );
        // A stale leftover record does not resurrect a finished task.
        m.set_meta("stale", Some(MetaInfo::default()));
        m.set_event("stale", ev(&format!("done [at={}]: old", GEN - 999)));
        assert!(!m.view(None).rows.iter().any(|r| r.id == "stale"));
    }

    #[test]
    fn bootstrap_rows_before_snapshot() {
        let mut m = FleetModel::new("home");
        m.set_meta(
            "t1",
            Some(MetaInfo {
                kind: Some("ship".into()),
                project: Some("p".into()),
            }),
        );
        m.set_event("t1", ev("needs-decision [at=5]: A or B"));
        m.set_meta("t2", Some(MetaInfo::default()));
        let v = m.view(None);
        assert_eq!(v.rows.len(), 2);
        assert_eq!(v.rows[0].state, TaskState::Unknown); // t2 has no project, sorts first
        assert_eq!(v.rows[1].state, TaskState::NeedsDecision);
        assert!(v.rows.iter().all(|r| r.source == StateSource::Bootstrap));
    }

    #[test]
    fn rows_take_title_project_basename_and_detail() {
        let v = model_with_snapshot().view(None);
        let row = v.rows.iter().find(|r| r.id == "perf-scout").unwrap();
        assert_eq!(row.name, "Find the slow dashboard query");
        assert_eq!(row.project.as_deref(), Some("example-ops"));
        assert_eq!(row.state, TaskState::NeedsDecision);
        assert_eq!(row.doing.as_deref(), Some("report ready, awaiting a pick"));
    }

    #[test]
    fn cache_is_shown_until_the_first_run_then_replaced() {
        let mut m = FleetModel::new("home");
        let snap = Arc::new(Snapshot::parse(FIXTURE).unwrap());
        m.apply_cached(snap.clone());
        assert!(m.info().cached);
        assert_eq!(m.view(None).rows.len(), 6);
        m.apply_snapshot(snap.clone());
        assert!(!m.info().cached);
        // A late cache read never replaces a live snapshot.
        m.apply_cached(Arc::new(Snapshot::default()));
        assert_eq!(m.snapshot().unwrap().tasks.len(), 6);
    }

    #[test]
    fn seq_increases_and_errors_clear() {
        let mut m = FleetModel::new("home");
        m.snapshot_failed("timeout".into());
        let a = m.view(None);
        assert_eq!(a.snapshot.error.as_deref(), Some("timeout"));
        m.apply_snapshot(Arc::new(Snapshot::parse(FIXTURE).unwrap()));
        let b = m.view(None);
        assert!(b.seq > a.seq);
        assert_eq!(b.snapshot.error, None);
        assert_eq!(b.snapshot.generated_at, Some(GEN));
    }
}
