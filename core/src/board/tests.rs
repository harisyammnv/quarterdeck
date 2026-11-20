use std::sync::Arc;

use super::*;
use crate::engine::HomeState;
use crate::snapshot::{BacklogRecord, Snapshot, Task};
use crate::status::parse_line;

const FIXTURE: &str = include_str!("../../tests/fixtures/board/fleet-snapshot.json");
// The fixture's `generated` time, 2026-11-20T10:00:00Z.
const GEN: i64 = 1_795_132_800 + 10 * 3600;

fn state_from(json: &str) -> HomeState {
    HomeState {
        snapshot: Some(Arc::new(Snapshot::parse(json).unwrap())),
        ..HomeState::default()
    }
}

fn fixture() -> HomeState {
    state_from(FIXTURE)
}

fn ids_of(cards: &[BoardCard]) -> Vec<&str> {
    cards.iter().map(|c| c.id.as_str()).collect()
}

fn decision_ids(list: &[Decision]) -> Vec<&str> {
    list.iter().map(|d| d.id.as_str()).collect()
}

fn card<'a>(cards: &'a [BoardCard], id: &str) -> &'a BoardCard {
    cards.iter().find(|c| c.id == id).unwrap()
}

#[test]
fn no_snapshot_is_not_ready() {
    let v = view(&HomeState::default());
    assert!(!v.ready);
    assert_eq!(v, BoardView::default());
}

#[test]
fn places_every_record_exactly_once() {
    let v = view(&fixture());
    assert!(v.ready);
    assert_eq!(v.generated_at, Some(GEN));
    assert_eq!(ids_of(&v.in_flight), ["login-flake", "auth-audit"]);
    assert_eq!(
        ids_of(&v.queued),
        [
            "python-bump",
            "cache-plan",
            "browser-pin",
            "clock-fix",
            "storage-move"
        ]
    );
    assert_eq!(
        decision_ids(&v.decisions.live),
        ["deps-rollup", "visual-pick"]
    );
    assert_eq!(ids_of(&v.done), ["infra-sync", "pin-update"]);
    assert_eq!(v.done_total, 2);
    assert_eq!(v.queued_total, 5);
    assert_eq!(
        v.other,
        [BoardUnplaced {
            id: "odd-row".into(),
            why: "unknown state \"parked\"".into()
        }]
    );
    // 12 backlog records, each placed exactly once.
    let placed = v.queued.len() + v.in_flight.len() + v.live_count() + v.done.len() + v.other.len();
    assert_eq!(placed, 12);
    assert!(v
        .queued
        .iter()
        .all(|c| c.column == BoardColumn::Queued && c.live.is_none()));
    assert!(v
        .in_flight
        .iter()
        .all(|c| c.column == BoardColumn::InFlight && c.live.is_some()));
    assert!(v.done.iter().all(|c| c.column == BoardColumn::Done));
}

#[test]
fn live_calls_are_captain_actionable_holds_oldest_first() {
    let v = view(&fixture());
    // deps-rollup was held at 09:38, visual-pick at 09:56.
    let d = &v.decisions.live[0];
    assert_eq!(d.raised_at, rfc3339_to_epoch("2026-11-20T09:38:00Z"));
    assert_eq!(d.question, "Merge the dependency rollup?");
    assert!(d.detail.as_deref().unwrap().contains("(2) hold until"));
    assert_eq!(d.bucket, HoldBucket::Live);
    assert_eq!(
        d.pr_url.as_deref(),
        Some("https://forge.example.invalid/acme/ledger-api/pull/1207")
    );
    // The PR is not repeated among the links, and the javascript: link is dropped.
    assert_eq!(
        d.links,
        ["https://forge.example.invalid/acme/ledger-api/actions/runs/1"]
    );
    assert!(!d.has_report);
    assert!(v.decisions.live[1].has_report);
}

#[test]
fn deferred_holds_keep_their_reason_and_sit_in_queued() {
    let v = view(&fixture());
    // Oldest first: cache-plan was filed 2026-10-30 (no hold stamp), python-bump held
    // 2026-11-10, browser-pin 2026-11-19.
    assert_eq!(
        decision_ids(&v.decisions.deferred),
        ["cache-plan", "python-bump", "browser-pin"]
    );
    assert_eq!(v.decisions.deferred_total, 3);
    let by = |id: &str| v.decisions.deferred.iter().find(|d| d.id == id).unwrap();
    assert_eq!(by("python-bump").bucket, HoldBucket::Dated);
    assert_eq!(by("python-bump").until.as_deref(), Some("2026-12-02"));
    assert_eq!(by("cache-plan").bucket, HoldBucket::Aged);
    assert_eq!(by("cache-plan").age_days, Some(21));
    // An escaping report path does not count as a report.
    assert!(!by("cache-plan").has_report);
    assert_eq!(by("browser-pin").bucket, HoldBucket::Blocked);
    assert_eq!(by("browser-pin").blocked_by, ["clock-fix"]);
    let c = card(&v.queued, "python-bump");
    assert_eq!(c.hold, Some(HoldBucket::Dated));
    assert_eq!(c.until.as_deref(), Some("2026-12-02"));
    assert_eq!(c.since, rfc3339_to_epoch("2026-11-10T08:00:00Z"));
    let plain = card(&v.queued, "storage-move");
    assert_eq!(plain.blocked_by, ["api-cutover"]);
    assert_eq!(plain.hold, None);
}

#[test]
fn in_flight_state_follows_the_fleet_merge_rule() {
    let mut s = fixture();
    let v = view(&s);
    let lf = card(&v.in_flight, "login-flake").live.clone().unwrap();
    assert_eq!(
        (lf.state, lf.source),
        (TaskState::Working, StateSource::Snapshot)
    );
    let aa = card(&v.in_flight, "auth-audit").live.clone().unwrap();
    assert_eq!(
        (aa.state, aa.source),
        (TaskState::Unknown, StateSource::Snapshot)
    );

    // An older event does not override a known snapshot state, but is still shown as latest.
    let older = parse_line(&format!("blocked [at={}]: old", GEN - 60)).unwrap();
    s.events.insert("login-flake".into(), older);
    // An older event does fill in where the snapshot could not tell.
    let fill = parse_line(&format!("working [at={}]: reading", GEN - 60)).unwrap();
    s.events.insert("auth-audit".into(), fill);
    let v = view(&s);
    let lf = card(&v.in_flight, "login-flake").live.clone().unwrap();
    assert_eq!(
        (lf.state, lf.source),
        (TaskState::Working, StateSource::Snapshot)
    );
    assert_eq!(lf.latest.as_deref(), Some("old"));
    let aa = card(&v.in_flight, "auth-audit").live.clone().unwrap();
    assert_eq!(
        (aa.state, aa.source),
        (TaskState::Working, StateSource::Event)
    );

    // A newer event wins.
    let newer = parse_line(&format!("needs-decision [at={}]: A or B?", GEN + 5)).unwrap();
    s.events.insert("login-flake".into(), newer);
    let lf = card(&view(&s).in_flight, "login-flake")
        .live
        .clone()
        .unwrap();
    assert_eq!(
        (lf.state, lf.source),
        (TaskState::NeedsDecision, StateSource::Event)
    );
    assert_eq!(lf.latest_at, Some(GEN + 5));
}

#[test]
fn ages_and_links_on_cards() {
    let v = view(&fixture());
    let day = date_to_epoch;
    let inf = card(&v.in_flight, "login-flake");
    assert_eq!(inf.since, day("2026-11-19"));
    // The task's recorded PR fills in when the backlog has none.
    assert_eq!(
        inf.pr_url.as_deref(),
        Some("https://forge.example.invalid/acme/ledger-api/pull/1215")
    );
    assert_eq!(card(&v.done, "pin-update").since, day("2026-11-18"));
    // An http:// PR link is not passed on.
    assert_eq!(card(&v.done, "infra-sync").pr_url, None);
    assert!(card(&v.in_flight, "auth-audit").has_report);
}

fn record(id: &str, extra: &str) -> String {
    format!(r#"{{"id":"{id}","structured":true,"title":"t {id}","state":"queued"{extra}}}"#)
}

fn snapshot(records: &[String]) -> String {
    format!(
        r#"{{"schema":"fm-fleet-snapshot.v1","generated":"2026-11-20T10:00:00Z","backlog":{{"records":[{}]}},"tasks":[]}}"#,
        records.join(",")
    )
}

#[test]
fn live_count_is_never_capped() {
    let records: Vec<_> = (0..137)
        .map(|i| {
            record(
                &format!("hold-{i:03}"),
                r#","hold_bucket":"live","captain_actionable":true"#,
            )
        })
        .collect();
    let v = view(&state_from(&snapshot(&records)));
    assert_eq!(v.live_count(), 137);
}

#[test]
fn done_is_capped_with_a_total() {
    let records: Vec<_> = (0..(MAX_DONE + 25))
        .map(|i| {
            record(&format!("d-{i:03}"), r#","state":"done""#).replace(r#""state":"queued","#, "")
        })
        .collect();
    let v = view(&state_from(&snapshot(&records)));
    assert_eq!(v.done.len(), MAX_DONE);
    assert_eq!(v.done_total as usize, MAX_DONE + 25);
}

#[test]
fn disagreeing_or_unknown_buckets_are_unplaced_not_guessed() {
    let records = [
        record("a", r#","hold_bucket":"live","captain_actionable":false"#),
        record("b", r#","hold_bucket":null,"captain_actionable":true"#),
        record(
            "c",
            r#","hold_bucket":"snoozed","captain_actionable":false"#,
        ),
    ];
    let v = view(&state_from(&snapshot(&records)));
    assert_eq!(v.live_count(), 0);
    assert!(v.queued.is_empty());
    let why: Vec<_> = v
        .other
        .iter()
        .map(|u| (u.id.as_str(), u.why.as_str()))
        .collect();
    assert_eq!(
        why,
        [
            ("a", "captain_actionable and hold_bucket disagree"),
            ("b", "captain_actionable and hold_bucket disagree"),
            ("c", "unknown hold bucket \"snoozed\""),
        ]
    );
}

#[test]
fn tolerates_sparse_records() {
    let v = view(&state_from(&snapshot(&[
        r#"{"id":"x","structured":true,"state":"in_flight","new_field":[1]}"#.into(),
    ])));
    let c = &v.in_flight[0];
    assert_eq!(c.title, "x");
    assert_eq!(c.since, None);
    assert_eq!(c.live.as_ref().unwrap().state, TaskState::Unknown);
}

#[test]
fn long_text_is_clipped_on_a_char_boundary() {
    let long = "é".repeat(MAX_TEXT + 50);
    let records = [record(
        "big",
        &format!(r#","hold_bucket":"live","captain_actionable":true,"hold_reason":"{long}""#),
    )];
    let v = view(&state_from(&snapshot(&records)));
    let detail = v.decisions.live[0].detail.as_deref().unwrap();
    assert_eq!(detail.chars().count(), MAX_TEXT + 1);
    assert!(detail.ends_with('…'));
}

#[test]
fn view_is_fast_on_a_large_home() {
    // Contract budget: 500 tasks and 2000 backlog records in under 5 ms (release build).
    let mut snap = Snapshot {
        generated: Some(GEN),
        ..Snapshot::default()
    };
    let mut events = std::collections::BTreeMap::new();
    for i in 0..2000 {
        let id = format!("task-{i:04}");
        let (st, bucket) = match i % 5 {
            0 => ("in_flight", None),
            1 => ("queued", Some("live")),
            2 => ("queued", Some("dated")),
            3 => ("done", None),
            _ => ("queued", None),
        };
        if i < 500 {
            snap.tasks.push(Task {
                id: id.clone(),
                state: Some("working".into()),
                ..Task::default()
            });
            events.insert(
                id.clone(),
                parse_line(&format!("working [at={}]: x", GEN + 1)).unwrap(),
            );
        }
        snap.backlog.push(BacklogRecord {
            id,
            title: Some("A task title of ordinary length".into()),
            state: Some(st.into()),
            hold_bucket: bucket.map(Into::into),
            captain_actionable: bucket == Some("live"),
            hold_reason: bucket.map(|_| "Options: (1) yes; (2) no.".into()),
            since: Some("2026-11-01".into()),
            ..BacklogRecord::default()
        });
    }
    let state = HomeState {
        snapshot: Some(Arc::new(snap)),
        events,
        ..HomeState::default()
    };
    let runs = 5;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        std::hint::black_box(view(&state));
    }
    let each = start.elapsed() / runs;
    let budget = if cfg!(debug_assertions) { 50 } else { 5 };
    assert!(
        each.as_millis() < budget,
        "view took {each:?} per run, budget {budget} ms"
    );
    let v = view(&state);
    assert_eq!(v.live_count(), 400);
    assert_eq!(v.queued.len(), MAX_QUEUED);
    assert_eq!(v.queued_total, 800);
    assert_eq!(v.decisions.deferred.len(), MAX_DEFERRED);
    assert_eq!(v.decisions.deferred_total, 400);
}

#[test]
fn payload_stays_inside_the_ipc_budget() {
    // Contract budget: one IPC payload under 256 KiB. Queued, deferred and done are capped;
    // in-flight work and live calls never are, so this home is large but plausible:
    // 60 tasks in flight, 80 live calls, 400 deferred holds, 1500 queued, 300 done.
    let mut snap = Snapshot {
        generated: Some(GEN),
        ..Snapshot::default()
    };
    let reason = "Rollup is green, 13 of 13 checks. Options: (1) merge now; (2) hold until the actions update is fixed; (3) decide later.";
    let mut push = |n: usize, state: &str, bucket: Option<&str>| {
        for i in 0..n {
            snap.backlog.push(BacklogRecord {
                id: format!("{state}-{}-{i:04}", bucket.unwrap_or("none")),
                title: Some("A task title of an ordinary length, about this long".into()),
                repo: Some("route-planner".into()),
                kind: Some("ship".into()),
                state: Some(state.into()),
                hold_bucket: bucket.map(Into::into),
                captain_actionable: bucket == Some("live"),
                hold_reason: bucket.map(|_| reason.into()),
                pr_url: Some("https://forge.example.invalid/acme/route-planner/pull/1234".into()),
                since: Some("2026-11-01".into()),
                ..BacklogRecord::default()
            });
        }
    };
    push(60, "in_flight", None);
    push(80, "queued", Some("live"));
    push(400, "queued", Some("dated"));
    push(1500, "queued", None);
    push(300, "done", None);
    let v = view(&HomeState {
        snapshot: Some(Arc::new(snap)),
        ..HomeState::default()
    });
    assert_eq!((v.live_count(), v.in_flight.len()), (80, 60));
    let bytes = serde_json::to_vec(&v).unwrap().len();
    assert!(bytes < 256 * 1024, "board payload is {bytes} bytes");
}

#[test]
fn parses_dates() {
    assert_eq!(date_to_epoch("1970-01-02"), Some(86_400));
    assert_eq!(date_to_epoch(" 2026-11-20 "), Some(1_795_132_800));
    assert_eq!(date_to_epoch("1970-01-01T00:00:10Z"), Some(10));
    assert_eq!(date_to_epoch("2026-13-01"), None);
    assert_eq!(date_to_epoch("yesterday"), None);
}
