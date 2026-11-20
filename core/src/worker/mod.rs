//! One worker, read up close: its terminal tail, its report and its status history.
//!
//! Every reader here takes a [`TaskId`], which only exists for a well-formed id that names a
//! task this home knows about. A raw string from the webview never reaches a path or an
//! argument vector.

pub mod history;
pub mod report;
pub mod sanitize;
pub mod tail;

use std::fmt;

use crate::home::FmHome;

/// The longest task id accepted.
const MAX_ID_LEN: usize = 128;

/// A validated task id: a known task in this home with a safe name.
///
/// The name must start with an ASCII letter or digit and use only ASCII letters, digits, `.`,
/// `_` and `-`, with no `..`. It can therefore never be a path, an option or an explicit
/// backend target. The task must have a `state/<id>.meta` record.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TaskIdError {
    #[error("not a task id: {0:?}")]
    Malformed(String),
    #[error("no task {0} in this home")]
    Unknown(String),
}

impl TaskId {
    /// Checks the shape of `raw` only. Use [`TaskId::known`] before reading anything.
    pub fn parse(raw: &str) -> Result<Self, TaskIdError> {
        let malformed = || TaskIdError::Malformed(raw.chars().take(MAX_ID_LEN).collect());
        let mut chars = raw.chars();
        let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
        if !first_ok
            || raw.len() > MAX_ID_LEN
            || raw.contains("..")
            || !raw
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(malformed());
        }
        Ok(Self(raw.to_string()))
    }

    /// Checks the shape of `raw` and that `home` has a meta record for it.
    pub fn known(home: &FmHome, raw: &str) -> Result<Self, TaskIdError> {
        let id = Self::parse(raw)?;
        let meta = home.state_dir().join(format!("{}.meta", id.0));
        match std::fs::symlink_metadata(&meta) {
            Ok(m) if m.is_file() => Ok(id),
            _ => Err(TaskIdError::Unknown(id.0)),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reads at most the last `cap` bytes of a file, starting on a character boundary.
/// Returns the text and whether anything before it was dropped.
pub(crate) fn read_tail(path: &std::path::Path, cap: u64) -> std::io::Result<(String, bool)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(cap);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::with_capacity(usize::try_from(len - start).unwrap_or(0));
    f.take(cap).read_to_end(&mut buf)?;
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if start > 0 {
        // The cut may land mid-line; drop the partial first line.
        match text.find('\n') {
            Some(nl) => {
                text.drain(..=nl);
            }
            None => text.clear(),
        }
    }
    Ok((text, start > 0))
}

#[cfg(test)]
pub(crate) mod testutil {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use crate::home::FmHome;

    pub fn script(path: &Path, body: &str) {
        std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A fake home with one known task, `app-login-fix`.
    pub fn fake_home() -> (tempfile::TempDir, FmHome) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for d in ["state", "bin", "data"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        std::fs::write(root.join("bin/fm-bearings-snapshot.sh"), "").unwrap();
        std::fs::write(root.join("bin/fm-inbox.sh"), "").unwrap();
        std::fs::write(root.join("state/app-login-fix.meta"), "kind=ship\n").unwrap();
        let home = FmHome::at(root).unwrap();
        (dir, home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_ids() {
        for ok in ["a", "app-login-fix", "qd.terminal_2", "9lives"] {
            assert!(TaskId::parse(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_paths_options_and_targets() {
        for bad in [
            "",
            "-n",
            "--all",
            "../x",
            "a/b",
            "a..b",
            ".hidden",
            "a b",
            "a:b",
            "%1",
            "a\n",
            "tmux:main:1",
            "é",
        ] {
            assert!(
                matches!(TaskId::parse(bad), Err(TaskIdError::Malformed(_))),
                "{bad:?}"
            );
        }
        assert!(TaskId::parse(&"a".repeat(MAX_ID_LEN + 1)).is_err());
    }

    #[test]
    fn known_needs_a_meta_file() {
        let (dir, home) = testutil::fake_home();
        assert!(TaskId::known(&home, "app-login-fix").is_ok());
        assert_eq!(
            TaskId::known(&home, "other"),
            Err(TaskIdError::Unknown("other".into()))
        );
        // A symlinked meta record is not trusted.
        std::os::unix::fs::symlink(
            dir.path().join("state/app-login-fix.meta"),
            dir.path().join("state/linked.meta"),
        )
        .unwrap();
        assert!(TaskId::known(&home, "linked").is_err());
    }

    #[test]
    fn read_tail_drops_the_partial_first_line() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, "first line\nsecond\nthird\n").unwrap();
        assert_eq!(
            read_tail(&p, 1000).unwrap(),
            ("first line\nsecond\nthird\n".into(), false)
        );
        assert_eq!(read_tail(&p, 10).unwrap(), ("third\n".into(), true));
    }
}
