//! TEMPORARY stand-in for the foundation seam (contract section 3): `snapshot::Snapshot`,
//! `snapshot::{Task, BacklogRecord}` and `engine::HomeState`. Deleted when this branch is
//! rebased onto the foundation; the board code imports these names only through here.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;

use crate::status::StatusEvent;
use crate::timefmt::rfc3339_to_epoch;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub generated: Option<i64>,
    pub tasks: Vec<Task>,
    pub backlog: Vec<BacklogRecord>,
}

#[derive(Clone, Debug, Default)]
pub struct Task {
    pub id: String,
    pub kind: Option<String>,
    pub state: Option<String>,
    pub pr_url: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct BacklogRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub captain_actionable: bool,
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
    #[serde(default)]
    pub report_path: Option<PathBuf>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub order: Option<i64>,
    // Fields the board adds to the foundation's record, one `#[serde(default)]` line each.
    #[serde(default)]
    pub hold_set: Option<String>,
    #[serde(default)]
    pub hold_age_days: Option<f64>,
    #[serde(default)]
    pub unresolved_blocker_ids: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub merged: Option<String>,
    #[serde(default)]
    pub reported: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct HomeState {
    pub snapshot: Option<Arc<Snapshot>>,
    pub events: BTreeMap<String, StatusEvent>,
}

#[derive(Deserialize)]
struct RawSnap {
    schema: String,
    #[serde(default)]
    generated: Option<String>,
    #[serde(default)]
    backlog: Option<RawBacklog>,
    #[serde(default)]
    tasks: Vec<RawTask>,
}
#[derive(Deserialize)]
struct RawBacklog {
    #[serde(default)]
    records: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct RawTask {
    id: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    current_state: Option<RawState>,
    #[serde(default)]
    pr: Option<RawPr>,
    #[serde(default)]
    backlog: Option<RawTitle>,
}
#[derive(Deserialize)]
struct RawState {
    #[serde(default)]
    state: Option<String>,
}
#[derive(Deserialize)]
struct RawPr {
    #[serde(default)]
    url: Option<String>,
}
#[derive(Deserialize)]
struct RawTitle {
    #[serde(default)]
    title: Option<String>,
}

impl Snapshot {
    pub fn parse(json: &str) -> Result<Self, String> {
        let raw: RawSnap = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if raw.schema != "fm-fleet-snapshot.v1" {
            return Err(format!("schema {}", raw.schema));
        }
        Ok(Self {
            generated: raw.generated.as_deref().and_then(rfc3339_to_epoch),
            // Unstructured backlog lines have no id; the foundation drops them the same way.
            backlog: raw
                .backlog
                .map(|b| b.records)
                .unwrap_or_default()
                .into_iter()
                .filter(|v| v.get("structured").and_then(serde_json::Value::as_bool) == Some(true))
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            tasks: raw
                .tasks
                .into_iter()
                .map(|t| Task {
                    id: t.id,
                    kind: t.kind,
                    state: t.current_state.and_then(|s| s.state),
                    pr_url: t.pr.and_then(|p| p.url),
                    title: t.backlog.and_then(|b| b.title),
                })
                .collect(),
        })
    }
}
