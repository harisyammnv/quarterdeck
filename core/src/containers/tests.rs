//! Tests against invented, recorded-shape fixtures and a fake engine. No socket is touched.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bollard::models::{
    ContainerInspectResponse, ContainerStatsResponse, ContainerSummary, ImageSummary, Network,
    SystemVersion, Volume, VolumeListResponse,
};

use super::*;
use crate::status::TaskState;

fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/containers")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Fake {
    version: Result<SystemVersion, String>,
    containers: Result<Vec<ContainerSummary>, String>,
    images_fail: bool,
    logs: Vec<logs::LogChunk>,
    inspects: std::sync::atomic::AtomicUsize,
}

impl Fake {
    fn kiri() -> Self {
        Self {
            version: Ok(fixture("version-kiri.json")),
            containers: Ok(fixture("containers.json")),
            images_fail: false,
            logs: Vec::new(),
            inspects: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl DockerApi for Fake {
    async fn version(&self) -> Result<SystemVersion, ApiError> {
        self.version.clone().map_err(ApiError)
    }
    async fn containers(&self) -> Result<Vec<ContainerSummary>, ApiError> {
        self.containers.clone().map_err(ApiError)
    }
    async fn inspect(&self, id: &str) -> Result<ContainerInspectResponse, ApiError> {
        self.inspects
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut all: BTreeMap<String, ContainerInspectResponse> = fixture("inspect.json");
        all.remove(id)
            .ok_or_else(|| ApiError("no such container".into()))
    }
    async fn images(&self) -> Result<Vec<ImageSummary>, ApiError> {
        if self.images_fail {
            return Err(ApiError("images broke".into()));
        }
        Ok(fixture("images.json"))
    }
    async fn volumes(&self) -> Result<Vec<Volume>, ApiError> {
        Ok(fixture::<VolumeListResponse>("volumes.json")
            .volumes
            .unwrap_or_default())
    }
    async fn networks(&self) -> Result<Vec<Network>, ApiError> {
        Ok(fixture("networks.json"))
    }
    async fn stats(&self, _id: &str) -> Result<ContainerStatsResponse, ApiError> {
        Ok(fixture("stats.json"))
    }
    async fn logs(&self, _id: &str, _tail: usize) -> Result<Vec<logs::LogChunk>, ApiError> {
        Ok(self.logs.clone())
    }
}

fn wt(task: &str, project: &str, path: &str, state: TaskState) -> WorktreeRef {
    WorktreeRef {
        task_id: task.into(),
        project: Some(project.into()),
        worktree: PathBuf::from(path),
        state,
    }
}

fn sample_worktrees() -> Vec<WorktreeRef> {
    vec![
        wt(
            "app-login",
            "example-app",
            "/p/example-app/fm-login",
            TaskState::Working,
        ),
        wt(
            "route-perf",
            "route-planner",
            "/p/route-planner/fm-route-perf",
            TaskState::Working,
        ),
        wt(
            "dash-profile",
            "portal-web",
            "/p/portal-web/fm-dash-profile",
            TaskState::Blocked,
        ),
        wt(
            "auth-audit",
            "portal-web",
            "/p/portal-web/fm-auth-audit",
            TaskState::Paused,
        ),
        // A duplicate worktree is shown once.
        wt(
            "app-login-dup",
            "example-app",
            "/p/example-app/fm-login",
            TaskState::Working,
        ),
    ]
}

const API: &str = "a1a1a1a1a1a1";

#[tokio::test]
async fn groups_by_compose_folder_and_bind_mount() {
    let svc = Containers::default();
    let v = svc
        .list_with(&Fake::kiri(), "kiri", &sample_worktrees())
        .await;
    assert_eq!(v.status, EngineStatus::Ready);
    assert_eq!(v.context.as_deref(), Some("kiri"));
    assert_eq!(v.engine_version.as_deref(), Some("27.5.1"));
    assert_eq!(v.kiri_version.as_deref(), Some("0.2.1"));
    let ids: Vec<_> = v.groups.iter().map(|g| g.task_id.as_str()).collect();
    assert_eq!(
        ids,
        ["app-login", "route-perf", "dash-profile", "auth-audit"]
    );

    let login = &v.groups[0];
    assert_eq!(login.worktree, "fm-login");
    let names: Vec<_> = login
        .containers
        .iter()
        .map(|c| c.service.as_deref().unwrap())
        .collect();
    assert_eq!(names, ["api", "db"]);
    let api = &login.containers[0];
    assert_eq!(api.id, API);
    assert_eq!(api.name, "login-api-1");
    assert_eq!(api.state, ContainerState::Running);
    assert_eq!(api.compose.as_deref(), Some("fm-login/compose.yaml"));
    assert_eq!(api.exit_code, None, "a running container has no exit code");
    assert_eq!(
        api.ports,
        [PortMap {
            host: Some(8080),
            container: 8080,
            proto: "tcp".into()
        }],
        "IPv4 and IPv6 bindings of one port are one entry"
    );

    // Bind mount under the worktree; a sibling folder with a longer name does not match.
    assert_eq!(v.groups[1].containers.len(), 1);
    assert_eq!(v.groups[1].containers[0].name, "route-web");
    let dash = &v.groups[2].containers[0];
    assert_eq!(dash.state, ContainerState::Exited);
    assert_eq!(dash.exit_code, Some(1));
    assert_eq!(dash.ports[0].host, None);
    assert!(v.groups[3].containers.is_empty());

    let other: Vec<_> = v.other.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(other, ["scratch-db", "route-cache"], "running first");

    assert_eq!(v.counts.containers, 6);
    assert_eq!(v.counts.running, 4);
    assert_eq!(v.counts.images, 2);
    assert_eq!(v.images[0].tag, "postgres:16");
    assert_eq!(v.images[0].more_tags, 1);
    assert_eq!(v.images[1].tag, "<none>");
    assert_eq!(v.images[1].containers, None);
    assert_eq!(v.volumes[0].name, "login_pg");
    assert_eq!(v.networks[1].name, "login_default");
    assert!(v.networks[1].internal);
    assert!(!v.truncated);
}

#[tokio::test]
async fn nothing_sensitive_crosses_the_wire() {
    let v = Containers::default()
        .list_with(&Fake::kiri(), "kiri", &sample_worktrees())
        .await;
    let json = serde_json::to_string(&v).unwrap();
    for needle in [
        "SECRET_TOKEN",
        "do-not-send",
        "/p/",
        "/var/lib",
        "com.docker.compose",
        "working_dir",
    ] {
        assert!(!json.contains(needle), "view leaks {needle}");
    }
}

#[tokio::test]
async fn plain_docker_is_not_kiri_and_side_lists_may_fail() {
    let fake = Fake {
        version: Ok(fixture("version-docker.json")),
        images_fail: true,
        ..Fake::kiri()
    };
    let v = Containers::default().list_with(&fake, "desktop", &[]).await;
    assert_eq!(v.status, EngineStatus::Ready);
    assert_eq!(v.kiri_version, None);
    assert!(v.images.is_empty());
    assert!(v.groups.is_empty());
    assert_eq!(v.other.len(), 6);
}

#[tokio::test]
async fn unreachable_engine_is_a_state_not_an_error() {
    let fake = Fake {
        version: Err("connection refused".into()),
        ..Fake::kiri()
    };
    let v = Containers::default()
        .list_with(&fake, "kiri", &sample_worktrees())
        .await;
    assert_eq!(v.status, EngineStatus::NotRunning);
    assert!(v.detail.unwrap().contains("connection refused"));
    assert!(v.groups.is_empty());

    let fake = Fake {
        containers: Err("boom".into()),
        ..Fake::kiri()
    };
    let v = Containers::default().list_with(&fake, "kiri", &[]).await;
    assert_eq!(v.status, EngineStatus::NotRunning);
}

#[tokio::test]
async fn missing_socket_and_tcp_contexts_are_never_dialled() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = Containers::new(DiscoveryEnv {
        docker_host: Some(format!("unix://{}/absent.sock", tmp.path().display())),
        ..DiscoveryEnv::default()
    });
    let v = svc.list(&[]).await;
    assert_eq!(v.status, EngineStatus::NotRunning);
    assert_eq!(v.context.as_deref(), Some("DOCKER_HOST"));
    assert!(!serde_json::to_string(&v).unwrap().contains("absent.sock"));

    let svc = Containers::new(DiscoveryEnv {
        docker_host: Some("tcp://engine.example.invalid:2375".into()),
        ..DiscoveryEnv::default()
    });
    let v = svc.list(&[]).await;
    assert_eq!(v.status, EngineStatus::Unsupported);
    assert!(v.detail.unwrap().contains("tcp"));
}

#[tokio::test]
async fn logs_and_stats_only_reach_listed_containers() {
    let svc = Containers::new(DiscoveryEnv {
        docker_host: Some("tcp://engine.example.invalid:2375".into()),
        ..DiscoveryEnv::default()
    });
    assert!(matches!(
        svc.logs("../../etc").await,
        Err(ContainersError::BadId)
    ));
    assert!(matches!(
        svc.stats("A1A1A1A1A1A1").await,
        Err(ContainersError::BadId)
    ));
    assert!(matches!(
        svc.stats(API).await,
        Err(ContainersError::UnknownId(_))
    ));
    svc.list_with(&Fake::kiri(), "kiri", &[]).await;
    assert_eq!(svc.resolve(API).unwrap(), "a1".repeat(32));
    // Listed, but the configured endpoint is not dialled.
    assert!(matches!(
        svc.stats(API).await,
        Err(ContainersError::Unavailable(_))
    ));
}

#[tokio::test]
async fn stats_match_docker_stats_arithmetic() {
    let svc = Containers::default();
    let s = svc.stats_with(&Fake::kiri(), API, "x").await.unwrap();
    assert_eq!(s.cpu_percent, Some(10.0));
    assert_eq!(s.memory_bytes, Some(222_300_000));
    assert_eq!(s.memory_limit, Some(8_000_000_000));
}

#[test]
fn stats_without_a_previous_sample_have_no_cpu() {
    let mut raw: ContainerStatsResponse = fixture("stats.json");
    raw.precpu_stats = None;
    raw.memory_stats = None;
    let s = stats::sample(API, &raw);
    assert_eq!(s.cpu_percent, None);
    assert_eq!(s.memory_bytes, None);
}

#[tokio::test]
async fn logs_are_bounded_and_clean() {
    let fake = Fake {
        logs: vec![logs::LogChunk {
            stderr: true,
            bytes: b"2026-11-20T10:00:00Z \x1b[33mWARN\x1b[0m slow query\n".to_vec(),
        }],
        ..Fake::kiri()
    };
    let l = Containers::default()
        .logs_with(&fake, API, "x")
        .await
        .unwrap();
    assert_eq!(l.id, API);
    assert_eq!(l.lines.len(), 1);
    assert_eq!(l.lines[0].text, "WARN slow query");
    assert!(l.lines[0].stderr);
    assert!(!l.truncated);
}

#[test]
fn short_ids_are_strict() {
    assert!(valid_short_id("0123456789ab"));
    assert!(!valid_short_id("0123456789a"));
    assert!(!valid_short_id("0123456789abc"));
    assert!(!valid_short_id("0123456789aB"));
    assert!(!valid_short_id("../../../etc"));
}

#[test]
fn listing_is_capped() {
    let one: Vec<ContainerSummary> = fixture("containers.json");
    let many: Vec<ContainerSummary> =
        std::iter::repeat_n(one[0].clone(), view::MAX_CONTAINERS + 5).collect();
    let listing = view::Listing {
        version: fixture("version-kiri.json"),
        containers: many,
        ..view::Listing::default()
    };
    let v = view::build("kiri", &listing, &[]);
    assert!(v.truncated);
    assert_eq!(v.other.len(), view::MAX_CONTAINERS);
    assert_eq!(
        v.counts.containers,
        u32::try_from(view::MAX_CONTAINERS + 5).unwrap()
    );
}

#[test]
fn projection_of_300_containers_and_200_worktrees_is_fast() {
    let one: Vec<ContainerSummary> = fixture("containers.json");
    let many: Vec<ContainerSummary> = one
        .iter()
        .cycle()
        .take(view::MAX_CONTAINERS)
        .cloned()
        .collect();
    let wts: Vec<WorktreeRef> = (0..200)
        .map(|i| {
            wt(
                &format!("t{i}"),
                "example-app",
                &format!("/p/example-app/fm-{i}"),
                TaskState::Working,
            )
        })
        .chain(sample_worktrees())
        .collect();
    let listing = view::Listing {
        version: fixture("version-kiri.json"),
        containers: many,
        ..view::Listing::default()
    };
    let start = std::time::Instant::now();
    let v = view::build("kiri", &listing, &wts);
    assert!(start.elapsed() < std::time::Duration::from_millis(250));
    assert!(serde_json::to_vec(&v).unwrap().len() < 256 * 1024);
}

fn home_state(events: &[(&str, &str)]) -> crate::engine::HomeState {
    let json = include_str!("../../tests/fixtures/fleet-snapshot.json");
    let snapshot = crate::snapshot::Snapshot::parse(json).unwrap();
    crate::engine::HomeState {
        seq: 1,
        snapshot: Some(std::sync::Arc::new(snapshot)),
        events: events
            .iter()
            .map(|(id, line)| ((*id).to_string(), crate::status::parse_line(line).unwrap()))
            .collect(),
        info: crate::fleet::SnapshotInfo::default(),
    }
}

#[test]
fn worktrees_come_from_the_snapshot_with_the_fleet_state_rule() {
    let state = home_state(&[]);
    let wts = worktrees(&state);
    let snapshot = state.snapshot.as_ref().unwrap();
    let with_worktree = snapshot
        .tasks
        .iter()
        .filter(|t| t.worktree.is_some())
        .count();
    assert!(with_worktree > 0, "the fixture has task worktrees");
    assert_eq!(wts.len(), with_worktree);
    let first = snapshot
        .tasks
        .iter()
        .find(|t| t.worktree.is_some())
        .unwrap();
    assert_eq!(wts[0].task_id, first.id);
    assert_eq!(wts[0].project, first.project_name());

    // A status event newer than the snapshot wins; an older one does not.
    let generated = snapshot.generated.unwrap();
    let id = first.id.clone();
    let newer = format!("blocked [at={}]: x", generated + 5);
    let older = format!("blocked [at={}]: x", generated - 5);
    let s = home_state(&[(id.as_str(), newer.as_str())]);
    assert_eq!(worktrees(&s)[0].state, TaskState::Blocked);
    let s = home_state(&[(id.as_str(), older.as_str())]);
    let snap_state = TaskState::from_word(first.state.as_deref().unwrap_or("unknown"));
    if snap_state != TaskState::Unknown {
        assert_eq!(worktrees(&s)[0].state, snap_state);
    }
}

#[test]
fn no_snapshot_means_no_worktrees() {
    let state = crate::engine::HomeState {
        seq: 0,
        snapshot: None,
        events: std::collections::BTreeMap::new(),
        info: crate::fleet::SnapshotInfo::default(),
    };
    assert!(worktrees(&state).is_empty());
}

#[test]
fn engine_errors_do_not_name_the_socket_path() {
    let socket = std::path::Path::new("/p/home/.kiri/run/docker.sock");
    assert_eq!(
        redact(
            "error trying to connect: /p/home/.kiri/run/docker.sock: refused",
            socket
        ),
        "error trying to connect: the socket: refused"
    );
}

#[tokio::test]
async fn detail_inspects_one_container_and_keeps_only_times() {
    let svc = Containers::default();
    let fake = Fake::kiri();
    let full = "e5".repeat(32);
    let d = svc.detail_with(&fake, "e5e5e5e5e5e5", &full).await.unwrap();
    assert_eq!(fake.inspects.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(d.exit_code, Some(1));
    assert!(d.started_at.is_some() && d.finished_at.is_some());
    let json = serde_json::to_string(&d).unwrap();
    assert!(!json.contains("SECRET_TOKEN") && !json.contains("do-not-send"));
    let running = svc.detail_with(&fake, API, &"a1".repeat(32)).await.unwrap();
    assert_eq!(running.started_at, Some(1_795_168_800));
    assert_eq!(
        running.finished_at, None,
        "the zero time is not a finish time"
    );
}

#[tokio::test]
async fn summary_reads_version_only() {
    let svc = Containers::default();
    let fake = Fake::kiri();
    let s = svc.summary_with(&fake, "kiri").await;
    assert_eq!(s.status, EngineStatus::Ready);
    assert_eq!(s.kiri_version.as_deref(), Some("0.2.1"));
    assert_eq!(fake.inspects.load(std::sync::atomic::Ordering::SeqCst), 0);
    let down = Fake {
        version: Err("refused".into()),
        ..Fake::kiri()
    };
    assert_eq!(
        svc.summary_with(&down, "kiri").await.status,
        EngineStatus::NotRunning
    );
    let tcp = Containers::new(DiscoveryEnv {
        docker_host: Some("tcp://engine.example.invalid:2375".into()),
        ..DiscoveryEnv::default()
    });
    assert_eq!(tcp.summary().await.status, EngineStatus::Unsupported);
}
