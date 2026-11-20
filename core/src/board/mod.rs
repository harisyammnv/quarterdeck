//! The board and the decisions strip, projected from the fleet snapshot.
//!
//! Source: the foundation's typed read of `bin/fm-fleet-snapshot.sh --json`. Its backlog
//! records hold every queued, in-flight and done task, and every captain hold carries a
//! `hold_bucket` that Firstmate decides from structured fields only. This module sorts on
//! `state`, `hold_bucket` and `captain_actionable` and nothing else: hold reasons are passed
//! through as text to show, never read to classify.
//!
//! The projection is total: every record lands in exactly one place, and a state or bucket
//! this version does not know goes to [`BoardView::other`] instead of being dropped.
//! In-flight cards take their live state from status events with the same rule as the fleet
//! list (`fleet.rs`): an event wins when it is newer than the snapshot or the snapshot says
//! `unknown`, and the card then says it came from the latest event.
//!
//! [`view`] is pure: no I/O and no clock. Only this compact view crosses to the UI.

use serde::Serialize;

use crate::engine::HomeState;
use crate::fleet::StateSource;
use crate::snapshot::{BacklogRecord, Snapshot, Task};
use crate::status::TaskState;
use crate::timefmt::rfc3339_to_epoch;

mod links;

pub use links::{safe_report_path, safe_web_url};

/// Longest prose passed to the UI, in characters. Hold reasons state a decision's options,
/// so the cap is generous; it only stops a runaway record.
const MAX_TEXT: usize = 4000;
/// Longest title passed to the UI, in characters.
const MAX_TITLE: usize = 300;
/// Most links or blocker ids kept per card.
const MAX_LINKS: usize = 8;
/// Most done cards sent; the total is sent alongside.
pub const MAX_DONE: usize = 40;

/// Which column a card sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum BoardColumn {
    Queued,
    InFlight,
    Done,
}

/// Firstmate's classification of a captain hold (`hold_bucket`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum HoldBucket {
    /// Waiting on you now.
    Live,
    /// Deferred until a date.
    Dated,
    /// Waiting on other work.
    Blocked,
    /// Undated and old enough that Firstmate set it aside.
    Aged,
}

impl HoldBucket {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "live" => Some(Self::Live),
            "dated" => Some(Self::Dated),
            "blocked" => Some(Self::Blocked),
            "aged" => Some(Self::Aged),
            _ => None,
        }
    }
}

/// The live state of an in-flight card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BoardLiveState {
    pub state: TaskState,
    /// The state word as written, so unknown words are still shown honestly.
    pub word: String,
    pub source: StateSource,
    /// The latest status event's text, when there is one.
    pub latest: Option<String>,
    /// The latest status event's time, Unix seconds.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub latest_at: Option<i64>,
}

/// One card on the board. Live captain holds are not cards: they are [`Decisions::live`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BoardCard {
    pub id: String,
    pub title: String,
    pub column: BoardColumn,
    pub project: Option<String>,
    pub kind: Option<String>,
    /// When the card entered its current place, Unix seconds: the hold time for a held card,
    /// the completion date for done work, the filed date otherwise.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub since: Option<i64>,
    /// Set for a deferred captain hold (dated, blocked or aged).
    pub hold: Option<HoldBucket>,
    /// The deferral date as written (`YYYY-MM-DD`), for dated holds.
    pub until: Option<String>,
    /// Unfinished work this card waits on.
    pub blocked_by: Vec<String>,
    /// In-flight cards only.
    pub live: Option<BoardLiveState>,
    pub pr_url: Option<String>,
    /// Firstmate recorded a report for this task; the reports view opens it by id.
    pub has_report: bool,
}

/// One open captain decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Decision {
    /// The task id. A reply note carries it.
    pub id: String,
    /// The question: the held task's title.
    pub question: String,
    /// The hold reason as Firstmate recorded it, which states the options. Plain text.
    pub detail: Option<String>,
    pub project: Option<String>,
    pub kind: Option<String>,
    pub bucket: HoldBucket,
    pub until: Option<String>,
    /// The hold's age in whole days, when Firstmate could compute it.
    pub age_days: Option<u32>,
    pub blocked_by: Vec<String>,
    /// When the hold was set, Unix seconds, when known.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub raised_at: Option<i64>,
    pub pr_url: Option<String>,
    pub has_report: bool,
    /// Other https links recorded on the task.
    pub links: Vec<String>,
}

/// The open decisions, split the way the strip and the decisions view show them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Decisions {
    /// Waiting on you now (`captain_actionable`), oldest first. Never capped.
    pub live: Vec<Decision>,
    /// Set aside: dated, blocked or aged holds, each with the reason Firstmate gives.
    pub deferred: Vec<Decision>,
}

/// A record the projection could not place, shown so nothing disappears silently.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BoardUnplaced {
    pub id: String,
    pub why: String,
}

/// Everything the board view and the decisions strip render.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BoardView {
    /// False until the first snapshot lands; the UI then shows a loading state.
    pub ready: bool,
    /// The snapshot's `generated` time, Unix seconds.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub generated_at: Option<i64>,
    /// Queued work in backlog order, deferred holds included.
    pub queued: Vec<BoardCard>,
    /// In-flight work in backlog order.
    pub in_flight: Vec<BoardCard>,
    /// Finished work, newest first, at most [`MAX_DONE`].
    pub done: Vec<BoardCard>,
    /// How many done records there are in all.
    pub done_total: u32,
    pub decisions: Decisions,
    /// Records with a state or bucket this version does not know.
    pub other: Vec<BoardUnplaced>,
}

impl BoardView {
    /// How many decisions wait on you now. Shown uncapped.
    pub fn live_count(&self) -> usize {
        self.decisions.live.len()
    }
}

/// Parses a calendar date such as `2026-11-20` (or a full RFC 3339 time) into Unix seconds
/// at midnight UTC.
pub(crate) fn date_to_epoch(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(t) = rfc3339_to_epoch(s) {
        return Some(t);
    }
    time::Date::parse(s, time::macros::format_description!("[year]-[month]-[day]"))
        .ok()
        .map(|d| d.midnight().assume_utc().unix_timestamp())
}

/// Cuts `s` to at most `max` characters, on a character boundary, marking the cut.
fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    match s.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &s[..at]),
        None => s.to_string(),
    }
}

fn text(s: Option<&String>, max: usize) -> Option<String> {
    s.map(|v| clip(v, max)).filter(|v| !v.is_empty())
}

fn days(d: Option<&serde_json::Number>) -> Option<u32> {
    let d = d?;
    match d.as_u64() {
        Some(n) => Some(u32::try_from(n).unwrap_or(u32::MAX)),
        None => d
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0)
            // Bounded and non-negative, so the cast cannot wrap.
            .map(|v| v.min(f64::from(u32::MAX)).floor() as u32),
    }
}

fn ids(list: &[String]) -> Vec<String> {
    list.iter()
        .map(|s| clip(s, 120))
        .filter(|s| !s.is_empty())
        .take(MAX_LINKS)
        .collect()
}

fn other_links(rec: &BacklogRecord, pr: Option<&String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for link in rec.links.iter().filter_map(|l| safe_web_url(l)) {
        if Some(&link) != pr && !out.contains(&link) {
            out.push(link);
        }
        if out.len() == MAX_LINKS {
            break;
        }
    }
    out
}

fn has_report(rec: &BacklogRecord) -> bool {
    rec.report_path
        .as_deref()
        .and_then(|p| p.to_str())
        .and_then(safe_report_path)
        .is_some()
}

/// Oldest first; unknown times last, keeping input order.
fn oldest_first(a: Option<i64>, b: Option<i64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// The live state of an in-flight task, by main's merge rule.
fn live_state(state: &HomeState, snap: &Snapshot, task: Option<&Task>, id: &str) -> BoardLiveState {
    let event = state.events.get(id);
    let word = task
        .and_then(|t| t.state.clone())
        .filter(|w| !w.trim().is_empty())
        .unwrap_or_else(|| "unknown".into());
    let snapshot_knows = TaskState::from_word(&word) != TaskState::Unknown;
    let newer = |at: Option<i64>| match (snap.generated, at) {
        (Some(generated), Some(at)) => at > generated,
        (None, _) => true,
        (Some(_), None) => false,
    };
    let (word, source) = match event {
        Some(e) if newer(e.at) || !snapshot_knows => (e.word.clone(), StateSource::Event),
        _ => (word, StateSource::Snapshot),
    };
    BoardLiveState {
        state: TaskState::from_word(&word),
        word: clip(&word, 40),
        source,
        latest: event
            .map(|e| clip(&e.text, MAX_TITLE))
            .filter(|t| !t.is_empty()),
        latest_at: event.and_then(|e| e.at),
    }
}

/// Projects the board from the latest home state.
pub fn view(state: &HomeState) -> BoardView {
    let Some(snap) = state.snapshot.as_deref() else {
        return BoardView::default();
    };
    let mut v = BoardView {
        ready: true,
        generated_at: snap.generated,
        ..BoardView::default()
    };
    let mut done = Vec::new();
    for rec in &snap.backlog {
        let id = rec.id.trim();
        if id.is_empty() {
            continue;
        }
        let unplaced = |why: String| BoardUnplaced {
            id: clip(id, 120),
            why,
        };
        let bucket_word = rec.hold_bucket.as_deref();
        let bucket = bucket_word.and_then(HoldBucket::from_word);
        if let (Some(word), None) = (bucket_word, bucket) {
            v.other.push(unplaced(format!(
                "unknown hold bucket {:?}",
                clip(word, 40)
            )));
            continue;
        }
        // `captain_actionable` is defined as `hold_bucket == "live"`; if they ever disagree
        // the record is shown as unplaced rather than guessed.
        if rec.captain_actionable != (bucket == Some(HoldBucket::Live)) {
            v.other.push(unplaced(
                "captain_actionable and hold_bucket disagree".into(),
            ));
            continue;
        }
        let column = match rec.state.as_deref() {
            Some("queued") => BoardColumn::Queued,
            Some("in_flight") => BoardColumn::InFlight,
            Some("done") => BoardColumn::Done,
            other => {
                v.other.push(unplaced(format!(
                    "unknown state {:?}",
                    clip(other.unwrap_or("none"), 40)
                )));
                continue;
            }
        };
        let task = snap.tasks.iter().find(|t| t.id == id);
        // Same precedence as the shell's `task_pr_open`: the task row, else the record.
        let pr_url = task
            .and_then(|t| t.pr_url.as_deref())
            .or(rec.pr_url.as_deref())
            .and_then(safe_web_url);
        let hold_set = rec.hold_set.as_deref().and_then(rfc3339_to_epoch);
        let filed = rec.since.as_deref().and_then(date_to_epoch);
        let title = text(rec.title.as_ref(), MAX_TITLE)
            .or_else(|| task.and_then(|t| text(t.title.as_ref(), MAX_TITLE)))
            .unwrap_or_else(|| clip(id, 120));
        let project = text(rec.repo.as_ref(), 120);
        let kind =
            text(rec.kind.as_ref(), 40).or_else(|| task.and_then(|t| text(t.kind.as_ref(), 40)));
        let blocked_by = ids(&rec.unresolved_blocker_ids);
        let until = text(rec.hold_until.as_ref(), 40);
        let report = has_report(rec);

        if let Some(bucket) = bucket {
            let decision = Decision {
                id: clip(id, 120),
                question: title.clone(),
                detail: text(rec.hold_reason.as_ref(), MAX_TEXT)
                    .or_else(|| text(rec.blocked_reason.as_ref(), MAX_TEXT)),
                project: project.clone(),
                kind: kind.clone(),
                bucket,
                until: until.clone(),
                age_days: days(rec.hold_age_days.as_ref()),
                blocked_by: blocked_by.clone(),
                raised_at: hold_set.or(filed),
                links: other_links(rec, pr_url.as_ref()),
                pr_url: pr_url.clone(),
                has_report: report,
            };
            if bucket == HoldBucket::Live {
                // A live hold is a call, not a card: the strip and the Waiting column show it.
                v.decisions.live.push(decision);
                continue;
            }
            v.decisions.deferred.push(decision);
        }

        let since = match column {
            BoardColumn::Done => rec
                .merged
                .as_deref()
                .or(rec.reported.as_deref())
                .and_then(date_to_epoch)
                .or(filed),
            _ if bucket.is_some() => hold_set.or(filed),
            _ => filed,
        };
        let card = BoardCard {
            id: clip(id, 120),
            title,
            column,
            project,
            kind,
            since,
            hold: bucket,
            until,
            blocked_by,
            live: (column == BoardColumn::InFlight).then(|| live_state(state, snap, task, id)),
            pr_url,
            has_report: report,
        };
        match column {
            BoardColumn::Queued => v.queued.push(card),
            BoardColumn::InFlight => v.in_flight.push(card),
            BoardColumn::Done => done.push(card),
        }
    }
    v.decisions
        .live
        .sort_by(|a, b| oldest_first(a.raised_at, b.raised_at));
    // Newest finished first; stable, so equal dates keep backlog order.
    done.sort_by(|a, b| oldest_first(b.since, a.since));
    v.done_total = u32::try_from(done.len()).unwrap_or(u32::MAX);
    done.truncate(MAX_DONE);
    v.done = done;
    v
}

#[cfg(test)]
mod tests;
