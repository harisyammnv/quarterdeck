//! Reports and status history end to end, against a fake home and the shared fixture snapshot.

mod common;

use std::sync::Arc;

use quarterdeck_core::reports::{self, ReportError};
use quarterdeck_core::snapshot::Snapshot;
use quarterdeck_core::{FmHome, HomeState};

fn state() -> HomeState {
    HomeState {
        snapshot: Some(Arc::new(Snapshot::parse(common::FIXTURE).unwrap())),
        ..HomeState::default()
    }
}

#[tokio::test]
async fn reads_a_report_by_task_id() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("data/app-login-fix")).unwrap();
    std::fs::write(
        dir.path().join("data/app-login-fix/report.md"),
        "# Login flake\n\nFixed.\n",
    )
    .unwrap();
    let r = reports::read(&home, &state(), "app-login-fix")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.markdown, "# Login flake\n\nFixed.\n");
    // A backlog record without a running task still has its report.
    assert!(reports::read(&home, &state(), "deps-rollup-merge")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn refuses_ids_outside_the_fleet() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    assert!(matches!(
        reports::read(&home, &state(), "nope").await,
        Err(ReportError::UnknownTask(_))
    ));
    assert!(matches!(
        reports::read(&home, &state(), "../state").await,
        Err(ReportError::BadId)
    ));
    assert!(matches!(
        reports::read(&home, &HomeState::default(), "app-login-fix").await,
        Err(ReportError::UnknownTask(_))
    ));
}

#[tokio::test]
async fn history_is_newest_first() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    std::fs::write(
        dir.path().join("state/app-login-fix.status"),
        "working [at=10]: setup done\nneeds-decision [at=20] [key=k]: A or B?\nresolved [at=30] [key=k]: A\n",
    )
    .unwrap();
    let h = reports::history(&home, &state(), "app-login-fix")
        .await
        .unwrap();
    let words: Vec<_> = h.events.iter().map(|e| e.word.as_str()).collect();
    assert_eq!(words, ["resolved", "needs-decision", "working"]);
    assert!(matches!(
        reports::history(&home, &state(), "nope").await,
        Err(ReportError::UnknownTask(_))
    ));
}

#[tokio::test]
async fn list_names_only_reports_that_exist_newest_first() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    // The fixture's scout_reports name perf-scout and auth-audit; only perf-scout has a file.
    std::fs::create_dir_all(dir.path().join("data/perf-scout")).unwrap();
    std::fs::write(
        dir.path().join("data/perf-scout/report.md"),
        "# Slow query\n",
    )
    .unwrap();
    let l = reports::list(&home, &state()).await;
    assert!(!l.truncated);
    assert_eq!(l.items.len(), 1);
    assert_eq!(l.items[0].task_id, "perf-scout");
    assert_eq!(l.items[0].kind.as_deref(), Some("scout"));
    assert!(l.items[0].modified.is_some());
    // No snapshot yet: nothing is listed.
    assert!(reports::list(&home, &HomeState::default())
        .await
        .items
        .is_empty());
}

#[tokio::test]
async fn list_leaves_out_reports_that_leave_the_data_directory() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("report.md"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("data/perf-scout")).unwrap();
    assert!(reports::list(&home, &state()).await.items.is_empty());
}

#[tokio::test]
async fn list_is_bounded() {
    let dir = common::fake_home();
    let home = FmHome::at(dir.path()).unwrap();
    let n = reports::MAX_LISTED + 5;
    let mut scouts = Vec::new();
    for i in 0..n {
        let id = format!("scout-{i:03}");
        std::fs::create_dir_all(dir.path().join("data").join(&id)).unwrap();
        std::fs::write(dir.path().join("data").join(&id).join("report.md"), "x").unwrap();
        scouts.push(serde_json::json!({ "id": id, "kind": "scout" }));
    }
    let json = serde_json::json!({ "schema": "fm-fleet-snapshot.v1", "scout_reports": scouts });
    let st = HomeState {
        snapshot: Some(Arc::new(Snapshot::parse(&json.to_string()).unwrap())),
        ..HomeState::default()
    };
    let l = reports::list(&home, &st).await;
    assert_eq!(l.items.len(), reports::MAX_LISTED);
    assert!(l.truncated);
}
