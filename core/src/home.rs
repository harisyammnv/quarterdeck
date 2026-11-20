//! Locating and validating the Firstmate home.

use std::path::{Path, PathBuf};

/// Environment variables consulted, in order. `QD_FM_HOME` lets the app point somewhere
/// other than the shell's `FM_HOME` without changing it.
pub const HOME_ENV_VARS: [&str; 2] = ["QD_FM_HOME", "FM_HOME"];

/// A validated Firstmate home directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FmHome {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HomeError {
    #[error("no Firstmate home configured: set QD_FM_HOME or FM_HOME to the home directory")]
    NotConfigured,
    #[error("{0} is not a Firstmate home: {1} is missing")]
    Invalid(PathBuf, &'static str),
}

impl FmHome {
    /// Required entries, relative to the home root.
    const REQUIRED: [&'static str; 3] = ["state", "bin/fm-bearings-snapshot.sh", "bin/fm-inbox.sh"];

    /// Validates `root` as a Firstmate home.
    pub fn at(root: impl Into<PathBuf>) -> Result<Self, HomeError> {
        let root = root.into();
        for rel in Self::REQUIRED {
            if !root.join(rel).exists() {
                return Err(HomeError::Invalid(root, rel));
            }
        }
        Ok(Self { root })
    }

    /// Finds the home from the environment.
    pub fn from_env() -> Result<Self, HomeError> {
        Self::from_lookup(|k| std::env::var_os(k))
    }

    /// Finds the home using `lookup` for environment variables (testable).
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self, HomeError> {
        let root = HOME_ENV_VARS
            .iter()
            .find_map(|k| lookup(k).filter(|v| !v.is_empty()))
            .ok_or(HomeError::NotConfigured)?;
        Self::at(PathBuf::from(root))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    pub fn backlog_file(&self) -> PathBuf {
        self.root.join("data").join("backlog.md")
    }

    pub fn script(&self, name: &str) -> PathBuf {
        self.root.join("bin").join(name)
    }

    /// A short label for the UI: the last two path components, never the full user path.
    pub fn label(&self) -> String {
        let parts: Vec<_> = self
            .root
            .components()
            .rev()
            .take(2)
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        parts.into_iter().rev().collect::<Vec<_>>().join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_home() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("state")).unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::write(dir.path().join("bin/fm-bearings-snapshot.sh"), "").unwrap();
        std::fs::write(dir.path().join("bin/fm-inbox.sh"), "").unwrap();
        dir
    }

    #[test]
    fn validates_required_entries() {
        let dir = fake_home();
        assert!(FmHome::at(dir.path()).is_ok());
        std::fs::remove_file(dir.path().join("bin/fm-inbox.sh")).unwrap();
        assert_eq!(
            FmHome::at(dir.path()),
            Err(HomeError::Invalid(
                dir.path().to_path_buf(),
                "bin/fm-inbox.sh"
            ))
        );
    }

    #[test]
    fn env_order_and_empty_values() {
        let dir = fake_home();
        let p = dir.path().as_os_str().to_owned();
        let home = FmHome::from_lookup(|k| (k == "FM_HOME").then(|| p.clone())).unwrap();
        assert_eq!(home.root(), dir.path());
        let none = FmHome::from_lookup(|k| (k == "QD_FM_HOME").then(Default::default));
        assert_eq!(none, Err(HomeError::NotConfigured));
    }

    #[test]
    fn label_is_short() {
        let home = FmHome {
            root: PathBuf::from("/x/y/orca/firstmate"),
        };
        assert_eq!(home.label(), "orca/firstmate");
    }
}
