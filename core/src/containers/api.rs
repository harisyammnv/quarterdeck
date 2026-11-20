//! The engine calls the view needs, behind a trait so tests run without an engine.
//!
//! Only read calls exist here: version, list, inspect, stats, logs. Nothing starts, stops,
//! removes or execs.

use std::future::Future;
use std::path::Path;
use std::time::Duration;

use bollard::models::{
    ContainerInspectResponse, ContainerStatsResponse, ContainerSummary, ImageSummary, Network,
    SystemVersion, Volume,
};
use bollard::query_parameters::{
    InspectContainerOptions, ListContainersOptionsBuilder, ListImagesOptionsBuilder,
    ListNetworksOptions, ListVolumesOptions, LogsOptionsBuilder, StatsOptionsBuilder,
};
use bollard::Docker;
use futures_util::StreamExt;

use super::logs::{LogChunk, MAX_RAW_BYTES};

/// An engine call failed; the text is the engine's or the connector's.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ApiError(pub String);

impl From<bollard::errors::Error> for ApiError {
    fn from(e: bollard::errors::Error) -> Self {
        Self(e.to_string())
    }
}

/// The read-only engine surface.
pub trait DockerApi: Send + Sync {
    fn version(&self) -> impl Future<Output = Result<SystemVersion, ApiError>> + Send;
    fn containers(&self) -> impl Future<Output = Result<Vec<ContainerSummary>, ApiError>> + Send;
    fn inspect(
        &self,
        id: &str,
    ) -> impl Future<Output = Result<ContainerInspectResponse, ApiError>> + Send;
    fn images(&self) -> impl Future<Output = Result<Vec<ImageSummary>, ApiError>> + Send;
    fn volumes(&self) -> impl Future<Output = Result<Vec<Volume>, ApiError>> + Send;
    fn networks(&self) -> impl Future<Output = Result<Vec<Network>, ApiError>> + Send;
    fn stats(
        &self,
        id: &str,
    ) -> impl Future<Output = Result<ContainerStatsResponse, ApiError>> + Send;
    /// The last `tail` lines, timestamped, stopping after [`MAX_RAW_BYTES`].
    fn logs(
        &self,
        id: &str,
        tail: usize,
    ) -> impl Future<Output = Result<Vec<LogChunk>, ApiError>> + Send;
}

/// Seconds the connector waits for any one call.
const CALL_TIMEOUT_SECS: u64 = 8;

/// The real engine over a Unix socket.
pub struct Engine {
    docker: Docker,
    socket: std::path::PathBuf,
}

impl Engine {
    /// Prepares a client for `socket`. Connects lazily, on the first call.
    pub fn unix(socket: &Path) -> Result<Self, ApiError> {
        let path = socket
            .to_str()
            .ok_or_else(|| ApiError("the socket path is not valid UTF-8".into()))?;
        let docker =
            Docker::connect_with_unix(path, CALL_TIMEOUT_SECS, bollard::API_DEFAULT_VERSION)?;
        Ok(Self {
            docker,
            socket: socket.to_path_buf(),
        })
    }
}

impl Engine {
    /// The socket this client talks to.
    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

async fn timed<T>(
    f: impl Future<Output = Result<T, bollard::errors::Error>>,
) -> Result<T, ApiError> {
    match tokio::time::timeout(Duration::from_secs(CALL_TIMEOUT_SECS), f).await {
        Ok(r) => r.map_err(ApiError::from),
        Err(_) => Err(ApiError(format!("no answer within {CALL_TIMEOUT_SECS} s"))),
    }
}

impl DockerApi for Engine {
    async fn version(&self) -> Result<SystemVersion, ApiError> {
        timed(self.docker.version()).await
    }

    async fn containers(&self) -> Result<Vec<ContainerSummary>, ApiError> {
        let opts = ListContainersOptionsBuilder::default().all(true).build();
        timed(self.docker.list_containers(Some(opts))).await
    }

    async fn inspect(&self, id: &str) -> Result<ContainerInspectResponse, ApiError> {
        timed(
            self.docker
                .inspect_container(id, None::<InspectContainerOptions>),
        )
        .await
    }

    async fn images(&self) -> Result<Vec<ImageSummary>, ApiError> {
        let opts = ListImagesOptionsBuilder::default().build();
        timed(self.docker.list_images(Some(opts))).await
    }

    async fn volumes(&self) -> Result<Vec<Volume>, ApiError> {
        let r = timed(self.docker.list_volumes(None::<ListVolumesOptions>)).await?;
        Ok(r.volumes.unwrap_or_default())
    }

    async fn networks(&self) -> Result<Vec<Network>, ApiError> {
        timed(self.docker.list_networks(None::<ListNetworksOptions>)).await
    }

    async fn stats(&self, id: &str) -> Result<ContainerStatsResponse, ApiError> {
        // One sample with the previous reading filled in, so CPU can be computed.
        let opts = StatsOptionsBuilder::default()
            .stream(false)
            .one_shot(false)
            .build();
        let mut stream = self.docker.stats(id, Some(opts));
        match tokio::time::timeout(Duration::from_secs(CALL_TIMEOUT_SECS), stream.next()).await {
            Ok(Some(r)) => r.map_err(ApiError::from),
            Ok(None) => Err(ApiError("the engine sent no stats".into())),
            Err(_) => Err(ApiError(format!("no answer within {CALL_TIMEOUT_SECS} s"))),
        }
    }

    async fn logs(&self, id: &str, tail: usize) -> Result<Vec<LogChunk>, ApiError> {
        use bollard::container::LogOutput;
        let opts = LogsOptionsBuilder::default()
            .stdout(true)
            .stderr(true)
            .timestamps(true)
            .follow(false)
            .tail(&tail.to_string())
            .build();
        let mut stream = self.docker.logs(id, Some(opts));
        let mut chunks = Vec::new();
        let mut raw = 0usize;
        let read = async {
            while let Some(item) = stream.next().await {
                let (stderr, bytes) = match item? {
                    LogOutput::StdErr { message } => (true, message),
                    LogOutput::StdOut { message } | LogOutput::Console { message } => {
                        (false, message)
                    }
                    LogOutput::StdIn { .. } => continue,
                };
                raw += bytes.len();
                chunks.push(LogChunk {
                    stderr,
                    bytes: bytes.to_vec(),
                });
                if raw >= MAX_RAW_BYTES {
                    break;
                }
            }
            Ok::<_, bollard::errors::Error>(())
        };
        match tokio::time::timeout(Duration::from_secs(CALL_TIMEOUT_SECS), read).await {
            Ok(r) => r.map_err(ApiError::from)?,
            Err(_) => return Err(ApiError(format!("no answer within {CALL_TIMEOUT_SECS} s"))),
        }
        Ok(chunks)
    }
}
