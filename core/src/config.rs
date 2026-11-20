//! The app's own configuration: one small JSON file in the app's data directory.
//!
//! It holds only choices the app makes for itself, today the Firstmate home picked in the
//! window. An app opened from Finder has no `QD_FM_HOME` or `FM_HOME`, so this file is what
//! lets it find the home again on the next launch. Nothing here is ever written into the
//! Firstmate home.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::home::{FmHome, HomeError, HOME_ENV_VARS};

/// File name inside the app's configuration directory.
pub const CONFIG_FILE: &str = "config.json";

/// Longest configuration file the app will read, in bytes.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// The Firstmate home chosen in the window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read the app configuration: {0}")]
    Read(std::io::Error),
    #[error("the app configuration is not valid: {0}")]
    Parse(serde_json::Error),
    #[error("the app configuration is larger than {MAX_CONFIG_BYTES} bytes")]
    TooLarge,
    #[error("could not save the app configuration: {0}")]
    Write(std::io::Error),
}

impl AppConfig {
    /// Reads the configuration in `dir`. A missing file is the default configuration.
    pub fn load(dir: &Path) -> Result<Self, ConfigError> {
        use std::io::Read as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        // Checked on the open handle (no symlink followed, no blocking on a FIFO), then read
        // no further than the cap.
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(dir.join(CONFIG_FILE))
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(ConfigError::Read(e)),
        };
        let meta = file.metadata().map_err(ConfigError::Read)?;
        if !meta.is_file() {
            return Err(ConfigError::Read(std::io::Error::other(
                "not a regular file",
            )));
        }
        if meta.len() > MAX_CONFIG_BYTES {
            return Err(ConfigError::TooLarge);
        }
        let mut text = String::new();
        file.take(MAX_CONFIG_BYTES)
            .read_to_string(&mut text)
            .map_err(ConfigError::Read)?;
        serde_json::from_str(&text).map_err(ConfigError::Parse)
    }

    /// Writes the configuration to `dir`, creating it when needed. The file is replaced
    /// atomically, so a crash mid-write never leaves a half-written configuration.
    pub fn save(&self, dir: &Path) -> Result<(), ConfigError> {
        std::fs::create_dir_all(dir).map_err(ConfigError::Write)?;
        let json = serde_json::to_string_pretty(self).map_err(|e| ConfigError::Write(e.into()))?;
        static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = dir.join(format!(".{CONFIG_FILE}.{}.{n}.tmp", std::process::id()));
        match std::fs::remove_file(&tmp) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(ConfigError::Write(e))
            }
            _ => {}
        }
        {
            use std::io::Write as _;
            use std::os::unix::fs::OpenOptionsExt as _;
            // Created exclusively, so a planted link at the temporary name is never followed.
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(ConfigError::Write)?;
            f.write_all(format!("{json}\n").as_bytes())
                .map_err(ConfigError::Write)?;
        }
        std::fs::rename(&tmp, dir.join(CONFIG_FILE)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ConfigError::Write(e)
        })
    }
}

/// Where the active home came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum HomeSource {
    /// `QD_FM_HOME` in the environment.
    QdFmHome,
    /// `FM_HOME` in the environment.
    FmHome,
    /// The home chosen in the window, saved in the app configuration.
    Chosen,
}

impl HomeSource {
    fn from_env_var(name: &str) -> Self {
        if name == "QD_FM_HOME" {
            Self::QdFmHome
        } else {
            Self::FmHome
        }
    }
}

/// Finds the home: an environment variable wins, so a developer can point a run elsewhere
/// without touching the saved choice; otherwise the home chosen in the window.
///
/// A variable that is set but names an invalid home is an error, not a silent fallback,
/// because the person who set it expects that home.
pub fn resolve_home(
    lookup: impl Fn(&str) -> Option<std::ffi::OsString>,
    config: &AppConfig,
) -> Result<(FmHome, HomeSource), HomeError> {
    for var in HOME_ENV_VARS {
        if let Some(root) = lookup(var).filter(|v| !v.is_empty()) {
            return FmHome::at(PathBuf::from(root)).map(|h| (h, HomeSource::from_env_var(var)));
        }
    }
    match &config.home {
        Some(root) => FmHome::at(root.clone()).map(|h| (h, HomeSource::Chosen)),
        None => Err(HomeError::NotConfigured),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn fake_home() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for d in ["state", "bin"] {
            std::fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        for f in ["bin/fm-fleet-snapshot.sh", "bin/fm-inbox.sh"] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        dir
    }

    #[test]
    fn missing_file_is_default_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_dir = dir.path().join("nested/app");
        assert_eq!(AppConfig::load(&cfg_dir).unwrap(), AppConfig::default());
        let cfg = AppConfig {
            home: Some(PathBuf::from("/x/home")),
        };
        cfg.save(&cfg_dir).unwrap();
        assert_eq!(AppConfig::load(&cfg_dir).unwrap(), cfg);
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(&cfg_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![OsString::from(CONFIG_FILE)]);
    }

    #[test]
    fn unknown_keys_are_ignored_and_bad_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(CONFIG_FILE), r#"{"home":"/h","later":1}"#).unwrap();
        assert_eq!(
            AppConfig::load(dir.path()).unwrap().home,
            Some(PathBuf::from("/h"))
        );
        std::fs::write(dir.path().join(CONFIG_FILE), "{").unwrap();
        assert!(matches!(
            AppConfig::load(dir.path()),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn oversized_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let big = " ".repeat(usize::try_from(MAX_CONFIG_BYTES).unwrap() + 1);
        std::fs::write(dir.path().join(CONFIG_FILE), big).unwrap();
        assert!(matches!(
            AppConfig::load(dir.path()),
            Err(ConfigError::TooLarge)
        ));
    }

    #[test]
    fn environment_wins_over_the_chosen_home() {
        let env_home = fake_home();
        let chosen = fake_home();
        let cfg = AppConfig {
            home: Some(chosen.path().to_path_buf()),
        };
        let p = env_home.path().as_os_str().to_owned();
        let (home, source) = resolve_home(|k| (k == "FM_HOME").then(|| p.clone()), &cfg).unwrap();
        assert_eq!((home.root(), source), (env_home.path(), HomeSource::FmHome));
        let (home, source) = resolve_home(|_| None, &cfg).unwrap();
        assert_eq!((home.root(), source), (chosen.path(), HomeSource::Chosen));
    }

    #[test]
    fn invalid_env_home_is_an_error_not_a_fallback() {
        let chosen = fake_home();
        let cfg = AppConfig {
            home: Some(chosen.path().to_path_buf()),
        };
        let err = resolve_home(
            |k| (k == "QD_FM_HOME").then(|| "/nonexistent/qd".into()),
            &cfg,
        )
        .unwrap_err();
        assert!(matches!(err, HomeError::Invalid(..)));
    }

    #[test]
    fn nothing_configured() {
        assert_eq!(
            resolve_home(|_| None, &AppConfig::default()).unwrap_err(),
            HomeError::NotConfigured
        );
    }
}
