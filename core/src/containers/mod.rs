//! Docker and Kiri containers (design 5.8, Phase 1 read-only).
//!
//! Finds the engine from the active Docker context ([`context`]), lists containers, images,
//! volumes and networks over its Unix socket ([`api`]), groups containers under the task
//! worktree that holds their compose folder or a bind-mounted path ([`group`]), and reads a
//! bounded log tail ([`logs`]) and one stats sample ([`stats`]) for a container the last
//! listing returned. Only read calls are made; nothing is started, stopped, removed or exec'd.

pub mod api;
pub mod context;
pub mod group;
pub mod logs;
pub mod stats;
pub mod view;

use std::collections::BTreeMap;
use std::sync::Mutex;

use futures_util::stream::{self, StreamExt};

use crate::engine::HomeState;
use crate::status::TaskState;

pub use api::{ApiError, DockerApi};
pub use context::{DiscoveryEnv, Endpoint};
pub use logs::{ContainerLogs, LogLine};
pub use stats::ContainerStats;
pub use view::{
    ContainerGroup, ContainerRow, ContainerState, ContainersView, Counts, EngineStatus, ImageRow,
    NetworkRow, PortMap, VolumeRow, WorktreeRef,
};

/// The task worktrees to group containers under, in snapshot order, with each task's state
/// merged as the fleet list merges it: a status event wins when it is newer than the
/// snapshot or the snapshot could not tell. Pure; tasks without a worktree are skipped.
pub fn worktrees(state: &HomeState) -> Vec<WorktreeRef> {
    let Some(snapshot) = state.snapshot.as_ref() else {
        return Vec::new();
    };
    snapshot
        .tasks
        .iter()
        .filter_map(|t| {
            let worktree = t.worktree.clone()?;
            let word = t.state.as_deref().unwrap_or("unknown");
            let known = TaskState::from_word(word) != TaskState::Unknown;
            let newer = |at: Option<i64>| match (snapshot.generated, at) {
                (Some(generated), Some(at)) => at > generated,
                (None, _) => true,
                (Some(_), None) => false,
            };
            let state = match state.events.get(&t.id) {
                Some(e) if newer(e.at) || !known => e.state(),
                _ => TaskState::from_word(word),
            };
            Some(WorktreeRef {
                task_id: t.id.clone(),
                project: t.project_name(),
                worktree,
                state,
            })
        })
        .collect()
}

/// Containers inspected at once while listing.
const INSPECT_CONCURRENCY: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum ContainersError {
    #[error("not a container id")]
    BadId,
    #[error("no container {0} in the last listing")]
    UnknownId(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("the engine refused: {0}")]
    Api(#[from] ApiError),
}

/// Engine error text can name the socket path, which holds the user's home folder; the
/// window gets "the socket" instead.
fn redact(text: &str, socket: &std::path::Path) -> String {
    match socket.to_str().filter(|s| !s.is_empty()) {
        Some(s) => text.replace(s, "the socket"),
        None => text.to_string(),
    }
}

/// A short id as the view sends it: exactly 12 lowercase hex characters.
pub fn valid_short_id(id: &str) -> bool {
    id.len() == 12
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The containers reader. Holds the ids of the last listing so logs and stats only ever reach
/// a container the UI was shown.
#[derive(Debug, Default)]
pub struct Containers {
    env: DiscoveryEnv,
    known: Mutex<BTreeMap<String, String>>,
}

impl Containers {
    pub fn new(env: DiscoveryEnv) -> Self {
        Self {
            env,
            known: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn from_env() -> Self {
        Self::new(DiscoveryEnv::from_process())
    }

    async fn endpoint(&self) -> Endpoint {
        let env = self.env.clone();
        tokio::task::spawn_blocking(move || context::discover(&env))
            .await
            .unwrap_or_else(|_| Endpoint::UnknownContext {
                context: "default".into(),
            })
    }

    /// Connects to the active context's engine, or says why it cannot.
    async fn engine(&self) -> Result<(String, api::Engine), Box<ContainersView>> {
        let fail = |status, context: String, detail: String| {
            Box::new(ContainersView::unavailable(status, Some(context), detail))
        };
        match self.endpoint().await {
            Endpoint::Unsupported { context, scheme } => Err(fail(
                EngineStatus::Unsupported,
                context,
                format!(
                    "This Docker context uses a {scheme} endpoint. Quarterdeck reads local Unix sockets only."
                ),
            )),
            Endpoint::UnknownContext { context } => Err(fail(
                EngineStatus::NotRunning,
                context,
                "The active Docker context has no endpoint on this machine.".into(),
            )),
            Endpoint::Unix { context, socket } => {
                if !tokio::fs::try_exists(&socket).await.unwrap_or(false) {
                    return Err(fail(
                        EngineStatus::NotRunning,
                        context,
                        "Nothing listens on this context's socket.".into(),
                    ));
                }
                match api::Engine::unix(&socket) {
                    Ok(e) => Ok((context, e)),
                    Err(e) => Err(fail(EngineStatus::NotRunning, context, e.0)),
                }
            }
        }
    }

    /// Lists the active engine. Never fails: an unreachable engine is a view state.
    pub async fn list(&self, worktrees: &[WorktreeRef]) -> ContainersView {
        match self.engine().await {
            Ok((context, engine)) => {
                let mut view = self.list_with(&engine, &context, worktrees).await;
                view.detail = view.detail.map(|d| redact(&d, engine.socket()));
                view
            }
            Err(view) => {
                self.known
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clear();
                *view
            }
        }
    }

    /// Lists through `api`. Only the version and container calls must succeed; an image,
    /// volume or network failure leaves that list empty.
    pub async fn list_with(
        &self,
        api: &impl DockerApi,
        context: &str,
        worktrees: &[WorktreeRef],
    ) -> ContainersView {
        let version = match api.version().await {
            Ok(v) => v,
            Err(e) => {
                return ContainersView::unavailable(
                    EngineStatus::NotRunning,
                    Some(context.to_string()),
                    format!("Docker did not answer: {e}"),
                )
            }
        };
        let (containers, images, volumes, networks) = tokio::join!(
            api.containers(),
            api.images(),
            api.volumes(),
            api.networks()
        );
        let containers = match containers {
            Ok(c) => c,
            Err(e) => {
                return ContainersView::unavailable(
                    EngineStatus::NotRunning,
                    Some(context.to_string()),
                    format!("Docker did not list containers: {e}"),
                )
            }
        };
        let ids: Vec<String> = containers
            .iter()
            .take(view::MAX_CONTAINERS)
            .filter_map(|c| c.id.clone())
            .collect();
        let inspected: BTreeMap<_, _> = stream::iter(ids.iter().cloned())
            .map(|id| async move { api.inspect(&id).await.ok().map(|r| (id, r)) })
            .buffer_unordered(INSPECT_CONCURRENCY)
            .filter_map(|r| async move { r })
            .collect()
            .await;
        {
            let mut known = self
                .known
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            known.clear();
            for id in &ids {
                known.insert(view::short_id(id), id.clone());
            }
        }
        let listing = view::Listing {
            version,
            containers,
            inspected,
            images: images.unwrap_or_default(),
            volumes: volumes.unwrap_or_default(),
            networks: networks.unwrap_or_default(),
        };
        view::build(context, &listing, worktrees)
    }

    /// The full id behind a short id from the last listing.
    fn resolve(&self, id: &str) -> Result<String, ContainersError> {
        if !valid_short_id(id) {
            return Err(ContainersError::BadId);
        }
        self.known
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(id)
            .cloned()
            .ok_or_else(|| ContainersError::UnknownId(id.to_string()))
    }

    async fn connected(&self) -> Result<api::Engine, ContainersError> {
        self.engine()
            .await
            .map(|(_, e)| e)
            .map_err(|v| ContainersError::Unavailable(v.detail.unwrap_or_default()))
    }

    fn redact_err(engine: &api::Engine, e: ContainersError) -> ContainersError {
        match e {
            ContainersError::Api(ApiError(m)) => {
                ContainersError::Api(ApiError(redact(&m, engine.socket())))
            }
            other => other,
        }
    }

    /// One stats sample for a listed container.
    pub async fn stats(&self, id: &str) -> Result<ContainerStats, ContainersError> {
        let full = self.resolve(id)?;
        let engine = self.connected().await?;
        self.stats_with(&engine, id, &full)
            .await
            .map_err(|e| Self::redact_err(&engine, e))
    }

    pub async fn stats_with(
        &self,
        api: &impl DockerApi,
        id: &str,
        full: &str,
    ) -> Result<ContainerStats, ContainersError> {
        Ok(stats::sample(id, &api.stats(full).await?))
    }

    /// The last lines of a listed container's output.
    pub async fn logs(&self, id: &str) -> Result<ContainerLogs, ContainersError> {
        let full = self.resolve(id)?;
        let engine = self.connected().await?;
        self.logs_with(&engine, id, &full)
            .await
            .map_err(|e| Self::redact_err(&engine, e))
    }

    pub async fn logs_with(
        &self,
        api: &impl DockerApi,
        id: &str,
        full: &str,
    ) -> Result<ContainerLogs, ContainersError> {
        let chunks = api.logs(full, logs::MAX_LINES).await?;
        let (lines, truncated) = logs::to_lines(&chunks);
        Ok(ContainerLogs {
            id: id.to_string(),
            lines,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests;
