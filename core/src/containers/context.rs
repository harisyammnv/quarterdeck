//! Finding the Docker engine's socket from the active Docker context.
//!
//! The order matches the Docker CLI: `DOCKER_HOST`, then `DOCKER_CONTEXT`, then
//! `currentContext` in `config.json`, then the `default` context. A named context is looked up
//! by scanning `contexts/meta/*/meta.json` for its `Name`, so the hashing scheme of the
//! directory names does not matter. Only `unix://` endpoints are used; anything else is
//! reported as unsupported and never dialled.
//!
//! `config.json` also holds registry credentials, so only `currentContext` is deserialized and
//! the file content is never logged or returned.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The socket of the `default` context: the Docker CLI's own default for that context, used
/// only when no `DOCKER_HOST` and no named context are configured.
pub const DEFAULT_SOCKET: &str = "/var/run/docker.sock";

/// Cap on `config.json` and each `meta.json`.
const MAX_CONFIG_BYTES: u64 = 256 * 1024;
/// Cap on the number of context directories scanned.
const MAX_CONTEXTS: usize = 256;

/// Where the engine is, as far as configuration says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    /// A local Unix socket.
    Unix { context: String, socket: PathBuf },
    /// A host with another scheme (`tcp`, `ssh`, `npipe`): not dialled.
    Unsupported { context: String, scheme: String },
    /// The named context has no metadata.
    UnknownContext { context: String },
}

impl Endpoint {
    pub fn context(&self) -> &str {
        match self {
            Self::Unix { context, .. }
            | Self::Unsupported { context, .. }
            | Self::UnknownContext { context } => context,
        }
    }
}

/// The inputs discovery reads from the process environment.
#[derive(Clone, Debug, Default)]
pub struct DiscoveryEnv {
    pub docker_host: Option<String>,
    pub docker_context: Option<String>,
    /// `DOCKER_CONFIG`, or `~/.docker`.
    pub config_dir: Option<PathBuf>,
}

impl DiscoveryEnv {
    pub fn from_process() -> Self {
        let nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let config_dir = nonempty("DOCKER_CONFIG")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".docker")));
        Self {
            docker_host: nonempty("DOCKER_HOST"),
            docker_context: nonempty("DOCKER_CONTEXT"),
            config_dir,
        }
    }
}

/// Resolves the active endpoint. Blocking: reads a few small files.
pub fn discover(env: &DiscoveryEnv) -> Endpoint {
    if let Some(host) = env.docker_host.as_deref() {
        return from_host("DOCKER_HOST", host);
    }
    let dir = env.config_dir.as_deref();
    let context = env
        .docker_context
        .clone()
        .or_else(|| dir.and_then(current_context))
        .unwrap_or_else(|| "default".to_string());
    if context == "default" {
        return Endpoint::Unix {
            context,
            socket: PathBuf::from(DEFAULT_SOCKET),
        };
    }
    match dir.and_then(|d| context_host(d, &context)) {
        Some(host) => from_host(&context, &host),
        None => Endpoint::UnknownContext { context },
    }
}

fn from_host(context: &str, host: &str) -> Endpoint {
    let host = host.trim();
    if let Some(path) = host.strip_prefix("unix://") {
        if !path.is_empty() {
            return Endpoint::Unix {
                context: context.to_string(),
                socket: PathBuf::from(path),
            };
        }
    }
    let scheme = host
        .split_once("://")
        .map_or("unknown", |(s, _)| s)
        .chars()
        .take(16)
        .collect();
    Endpoint::Unsupported {
        context: context.to_string(),
        scheme,
    }
}

fn read_capped(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let mut buf = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_CONFIG_BYTES)
        .read_to_string(&mut buf)
        .ok()?;
    Some(buf)
}

fn current_context(dir: &Path) -> Option<String> {
    #[derive(Deserialize)]
    struct Config {
        #[serde(rename = "currentContext", default)]
        current_context: Option<String>,
    }
    let text = read_capped(&dir.join("config.json"))?;
    let cfg: Config = serde_json::from_str(&text).ok()?;
    cfg.current_context.filter(|c| !c.trim().is_empty())
}

fn context_host(dir: &Path, name: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Meta {
        #[serde(rename = "Name", default)]
        name: String,
        #[serde(rename = "Endpoints", default)]
        endpoints: Endpoints,
    }
    #[derive(Deserialize, Default)]
    struct Endpoints {
        #[serde(default)]
        docker: Option<DockerEndpoint>,
    }
    #[derive(Deserialize)]
    struct DockerEndpoint {
        #[serde(rename = "Host", default)]
        host: Option<String>,
    }
    let entries = std::fs::read_dir(dir.join("contexts").join("meta")).ok()?;
    entries
        .flatten()
        .take(MAX_CONTEXTS)
        .filter_map(|e| read_capped(&e.path().join("meta.json")))
        .filter_map(|t| serde_json::from_str::<Meta>(&t).ok())
        .find(|m| m.name == name)
        .and_then(|m| m.endpoints.docker.and_then(|d| d.host))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/containers/docker-config")
    }

    fn env(dir: Option<PathBuf>) -> DiscoveryEnv {
        DiscoveryEnv {
            docker_host: None,
            docker_context: None,
            config_dir: dir,
        }
    }

    #[test]
    fn current_context_resolves_through_meta_name() {
        let e = discover(&env(Some(fixture_dir())));
        assert_eq!(
            e,
            Endpoint::Unix {
                context: "kiri".into(),
                socket: PathBuf::from("/p/home/.kiri/run/docker.sock"),
            }
        );
    }

    #[test]
    fn docker_context_env_wins_over_config() {
        let mut e = env(Some(fixture_dir()));
        e.docker_context = Some("desktop".into());
        assert_eq!(
            discover(&e),
            Endpoint::Unix {
                context: "desktop".into(),
                socket: PathBuf::from("/p/home/.docker/run/docker.sock"),
            }
        );
    }

    #[test]
    fn docker_host_wins_and_tcp_is_unsupported() {
        let mut e = env(Some(fixture_dir()));
        e.docker_host = Some("tcp://engine.example.invalid:2376".into());
        assert_eq!(
            discover(&e),
            Endpoint::Unsupported {
                context: "DOCKER_HOST".into(),
                scheme: "tcp".into(),
            }
        );
        e.docker_host = Some("unix:///p/run/docker.sock".into());
        assert_eq!(
            discover(&e),
            Endpoint::Unix {
                context: "DOCKER_HOST".into(),
                socket: PathBuf::from("/p/run/docker.sock"),
            }
        );
    }

    #[test]
    fn ssh_context_is_unsupported() {
        let mut e = env(Some(fixture_dir()));
        e.docker_context = Some("remote".into());
        assert!(matches!(discover(&e), Endpoint::Unsupported { scheme, .. } if scheme == "ssh"));
    }

    #[test]
    fn missing_config_means_default_socket() {
        let e = discover(&env(Some(PathBuf::from("/p/does-not-exist"))));
        assert_eq!(
            e,
            Endpoint::Unix {
                context: "default".into(),
                socket: PathBuf::from(DEFAULT_SOCKET),
            }
        );
        assert_eq!(discover(&env(None)).context(), "default");
    }

    #[test]
    fn unknown_context_is_reported() {
        let mut e = env(Some(fixture_dir()));
        e.docker_context = Some("nope".into());
        assert_eq!(
            discover(&e),
            Endpoint::UnknownContext {
                context: "nope".into()
            }
        );
    }

    #[test]
    fn malformed_and_oversized_config_fall_back_to_default() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("config.json"), "{not json").unwrap();
        assert_eq!(discover(&env(Some(tmp.path().into()))).context(), "default");
        let big = format!(
            "{{\"pad\":\"{}\",\"currentContext\":\"kiri\"}}",
            "x".repeat(usize::try_from(MAX_CONFIG_BYTES).unwrap())
        );
        std::fs::write(tmp.path().join("config.json"), big).unwrap();
        assert_eq!(discover(&env(Some(tmp.path().into()))).context(), "default");
    }
}
