//! The wire types the containers view renders, and the pure projection that builds them from
//! engine records. Nothing here does I/O.
//!
//! What never crosses: environment variables, raw label maps, absolute host paths (mount
//! sources, compose folders, socket paths), full command lines.

use std::path::{Path, PathBuf};

use bollard::models::{
    ContainerInspectResponse, ContainerSummary, ImageSummary, Network, SystemVersion, Volume,
};
use serde::Serialize;

use super::group::{self, LABEL_CONFIG_FILES, LABEL_SERVICE, LABEL_WORKING_DIR};
use crate::status::TaskState;
use crate::timefmt::rfc3339_to_epoch;

/// Row caps, so one payload stays well under the IPC budget.
pub const MAX_CONTAINERS: usize = 300;
pub const MAX_IMAGES: usize = 300;
pub const MAX_VOLUMES: usize = 300;
pub const MAX_NETWORKS: usize = 100;
/// Longest name, image or tag text sent to the UI.
const MAX_TEXT: usize = 200;

/// Times and exit code of one container, from inspecting it alone when it is selected.
/// Nothing else from the inspect response (environment, mounts, command) is kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainerDetail {
    pub id: String,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub started_at: Option<i64>,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub finished_at: Option<i64>,
    pub exit_code: Option<i32>,
    pub restart_count: Option<u32>,
}

impl ContainerDetail {
    pub fn from_inspect(id: &str, i: &ContainerInspectResponse) -> Self {
        let s = i.state.as_ref();
        let time = |t: Option<&String>| t.and_then(|t| rfc3339_to_epoch(t)).filter(|t| *t > 0);
        Self {
            id: id.to_string(),
            started_at: time(s.and_then(|s| s.started_at.as_ref())),
            finished_at: time(s.and_then(|s| s.finished_at.as_ref())),
            exit_code: s
                .and_then(|s| s.exit_code)
                .and_then(|c| i32::try_from(c).ok()),
            restart_count: i.restart_count.and_then(|n| u32::try_from(n).ok()),
        }
    }
}

/// The engine as the status bar needs it: discovery and `/version` only, no listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EngineSummary {
    pub status: EngineStatus,
    pub context: Option<String>,
    pub engine_version: Option<String>,
    pub kiri_version: Option<String>,
}

/// A task's worktree, as the grouping needs it. Built by the caller from the fleet snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeRef {
    pub task_id: String,
    /// Project label (folder name), when known.
    pub project: Option<String>,
    pub worktree: PathBuf,
    pub state: TaskState,
}

/// Whether the engine could be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum EngineStatus {
    /// Listed successfully.
    Ready,
    /// No socket, or it did not answer.
    NotRunning,
    /// The active context is not a local Unix socket.
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
    Unknown,
}

impl ContainerState {
    fn from_word(word: &str) -> Self {
        match word.to_ascii_lowercase().as_str() {
            "created" => Self::Created,
            "running" => Self::Running,
            "paused" => Self::Paused,
            "restarting" => Self::Restarting,
            "removing" => Self::Removing,
            "exited" => Self::Exited,
            "dead" => Self::Dead,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PortMap {
    /// The published host port, when published.
    pub host: Option<u16>,
    pub container: u16,
    pub proto: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainerRow {
    /// Short id (12 hex characters); the handle for logs and stats.
    pub id: String,
    /// Container name without the leading slash.
    pub name: String,
    /// The compose service, when the container belongs to a compose project.
    pub service: Option<String>,
    pub image: String,
    pub state: ContainerState,
    /// The engine's status text, such as "Up 5 minutes" or "Exited (1) 2 hours ago".
    pub status: String,
    pub exit_code: Option<i32>,
    pub ports: Vec<PortMap>,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub created_at: Option<i64>,
    /// The compose file, relative to its worktree folder.
    pub compose: Option<String>,
}

/// The containers that belong to one task's worktree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainerGroup {
    pub task_id: String,
    pub project: Option<String>,
    /// The worktree folder name.
    pub worktree: String,
    pub state: TaskState,
    pub containers: Vec<ContainerRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImageRow {
    pub id: String,
    /// First `repo:tag`, or `<none>`.
    pub tag: String,
    /// How many more tags point at the image.
    pub more_tags: u32,
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub size: u64,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub created_at: Option<i64>,
    /// Containers using the image, when the engine reports it.
    pub containers: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct VolumeRow {
    pub name: String,
    pub driver: String,
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub created_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NetworkRow {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
}

/// Totals before any cap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Counts {
    pub containers: u32,
    pub running: u32,
    pub images: u32,
    pub volumes: u32,
    pub networks: u32,
}

/// Everything the containers view renders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainersView {
    pub status: EngineStatus,
    /// The Docker context name.
    pub context: Option<String>,
    /// Why the engine is not ready, in words for the UI.
    pub detail: Option<String>,
    pub engine_version: Option<String>,
    /// Kiri's version, when the engine is Kiri.
    pub kiri_version: Option<String>,
    /// One group per task worktree, in fleet order, including worktrees with no containers.
    pub groups: Vec<ContainerGroup>,
    /// Containers that sit in no task worktree.
    pub other: Vec<ContainerRow>,
    pub images: Vec<ImageRow>,
    pub volumes: Vec<VolumeRow>,
    pub networks: Vec<NetworkRow>,
    pub counts: Counts,
    /// Some list was cut at its cap.
    pub truncated: bool,
}

impl ContainersView {
    /// A view for an engine that could not be listed.
    pub fn unavailable(status: EngineStatus, context: Option<String>, detail: String) -> Self {
        Self {
            status,
            context,
            detail: Some(detail),
            engine_version: None,
            kiri_version: None,
            groups: Vec::new(),
            other: Vec::new(),
            images: Vec::new(),
            volumes: Vec::new(),
            networks: Vec::new(),
            counts: Counts::default(),
            truncated: false,
        }
    }
}

/// What the engine returned for one listing.
#[derive(Clone, Debug, Default)]
pub struct Listing {
    pub version: SystemVersion,
    pub containers: Vec<ContainerSummary>,
    pub images: Vec<ImageSummary>,
    pub volumes: Vec<Volume>,
    pub networks: Vec<Network>,
}

/// The first 12 characters of an id, without any `sha256:` prefix.
pub fn short_id(id: &str) -> String {
    let id = id.strip_prefix("sha256:").unwrap_or(id);
    id.chars().take(12).collect()
}

fn clip(s: &str) -> String {
    s.chars().take(MAX_TEXT).collect()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Kiri's version from the standard `/version` fields.
pub fn kiri_version(v: &SystemVersion) -> Option<String> {
    let component = v
        .components
        .iter()
        .flatten()
        .find(|c| c.name.eq_ignore_ascii_case("kiri"))
        .map(|c| clip(&c.version));
    let platform = v
        .platform
        .as_ref()
        .is_some_and(|p| p.name.eq_ignore_ascii_case("kiri"));
    component.or_else(|| platform.then(|| "unknown".to_string()))
}

/// The exit code in a status such as "Exited (137) 2 minutes ago", when there is one.
fn exit_code(status: &str) -> Option<i32> {
    let rest = status.trim().strip_prefix("Exited (")?;
    rest.split_once(')')?.0.trim().parse().ok()
}

fn row(c: &ContainerSummary, wt: Option<&Path>) -> ContainerRow {
    let labels = c.labels.as_ref();
    let label = |k: &str| labels.and_then(|l| l.get(k)).filter(|v| !v.is_empty());
    let name = c.names.iter().flatten().next().map_or_else(
        || short_id(c.id.as_deref().unwrap_or("")),
        |n| clip(n.trim_start_matches('/')),
    );
    let state = c.state.as_ref().map_or(ContainerState::Unknown, |s| {
        ContainerState::from_word(s.as_ref())
    });
    let mut ports: Vec<PortMap> = c
        .ports
        .iter()
        .flatten()
        .map(|p| PortMap {
            host: p.public_port,
            container: p.private_port,
            proto: p
                .typ
                .as_ref()
                .map_or_else(|| "tcp".into(), |t| clip(t.as_ref())),
        })
        .collect();
    ports.sort_by_key(|p| (p.host.is_none(), p.host, p.container));
    ports.dedup();
    ports.truncate(16);
    let status = c.status.as_deref().unwrap_or("");
    ContainerRow {
        id: short_id(c.id.as_deref().unwrap_or("")),
        name,
        service: label(LABEL_SERVICE).map(|s| clip(s)),
        image: clip(c.image.as_deref().unwrap_or("")),
        state,
        status: clip(status),
        exit_code: exit_code(status)
            .filter(|_| matches!(state, ContainerState::Exited | ContainerState::Dead)),
        ports,
        created_at: c.created,
        compose: label(LABEL_CONFIG_FILES).and_then(|f| group::compose_label(f, wt)),
    }
}

/// Paths that tie a container to a folder: the compose working directory and bind sources.
fn anchor_paths(c: &ContainerSummary) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = c
        .labels
        .as_ref()
        .and_then(|l| l.get(LABEL_WORKING_DIR))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for m in c.mounts.iter().flatten() {
        let bind = m.typ.as_ref().is_some_and(|t| t == "bind");
        if let (true, Some(src)) = (bind, m.source.as_deref()) {
            out.push(PathBuf::from(src));
        }
    }
    out
}

fn sort_rows(rows: &mut [ContainerRow]) {
    rows.sort_by(|a, b| {
        let live = |r: &ContainerRow| r.state != ContainerState::Running;
        (live(a), a.service.as_ref().unwrap_or(&a.name))
            .cmp(&(live(b), b.service.as_ref().unwrap_or(&b.name)))
    });
}

/// Builds the view from one listing. Pure.
pub fn build(context: &str, listing: &Listing, worktrees: &[WorktreeRef]) -> ContainersView {
    let mut seen = std::collections::BTreeSet::new();
    let worktrees: Vec<&WorktreeRef> = worktrees
        .iter()
        .filter(|w| seen.insert(w.worktree.clone()))
        .collect();
    let mut groups: Vec<ContainerGroup> = worktrees
        .iter()
        .map(|w| ContainerGroup {
            task_id: clip(&w.task_id),
            project: w.project.as_deref().map(clip),
            worktree: w
                .worktree
                .file_name()
                .map_or_else(|| clip(&w.task_id), |n| clip(&n.to_string_lossy())),
            state: w.state,
            containers: Vec::new(),
        })
        .collect();
    let mut other = Vec::new();
    let mut truncated = listing.containers.len() > MAX_CONTAINERS;
    for c in listing.containers.iter().take(MAX_CONTAINERS) {
        let idx = group::owner(
            worktrees.iter().map(|w| w.worktree.as_path()),
            &anchor_paths(c),
        );
        let wt = idx.map(|i| worktrees[i].worktree.as_path());
        let r = row(c, wt);
        match idx {
            Some(i) => groups[i].containers.push(r),
            None => other.push(r),
        }
    }
    for g in &mut groups {
        sort_rows(&mut g.containers);
    }
    sort_rows(&mut other);

    let images = listing
        .images
        .iter()
        .take(MAX_IMAGES)
        .map(|i| {
            let tags: Vec<&String> = i
                .repo_tags
                .iter()
                .filter(|t| t.as_str() != "<none>:<none>")
                .collect();
            ImageRow {
                id: short_id(&i.id),
                tag: tags
                    .first()
                    .map_or_else(|| "<none>".to_string(), |t| clip(t)),
                more_tags: count(tags.len().saturating_sub(1)),
                size: u64::try_from(i.size).unwrap_or(0),
                created_at: Some(i.created).filter(|t| *t > 0),
                containers: u32::try_from(i.containers).ok(),
            }
        })
        .collect();
    let volumes = listing
        .volumes
        .iter()
        .take(MAX_VOLUMES)
        .map(|v| VolumeRow {
            name: clip(&v.name),
            driver: clip(&v.driver),
            created_at: v.created_at.as_deref().and_then(rfc3339_to_epoch),
        })
        .collect();
    let networks = listing
        .networks
        .iter()
        .take(MAX_NETWORKS)
        .map(|n| NetworkRow {
            id: short_id(n.id.as_deref().unwrap_or("")),
            name: clip(n.name.as_deref().unwrap_or("")),
            driver: clip(n.driver.as_deref().unwrap_or("")),
            scope: clip(n.scope.as_deref().unwrap_or("")),
            internal: n.internal.unwrap_or(false),
        })
        .collect();
    truncated |= listing.images.len() > MAX_IMAGES
        || listing.volumes.len() > MAX_VOLUMES
        || listing.networks.len() > MAX_NETWORKS;

    let running = listing
        .containers
        .iter()
        .filter(|c| c.state.as_ref().is_some_and(|s| s.to_string() == "running"))
        .count();
    ContainersView {
        status: EngineStatus::Ready,
        context: Some(clip(context)),
        detail: None,
        engine_version: listing.version.version.as_deref().map(clip),
        kiri_version: kiri_version(&listing.version),
        groups,
        other,
        images,
        volumes,
        networks,
        counts: Counts {
            containers: count(listing.containers.len()),
            running: count(running),
            images: count(listing.images.len()),
            volumes: count(listing.volumes.len()),
            networks: count(listing.networks.len()),
        },
        truncated,
    }
}
