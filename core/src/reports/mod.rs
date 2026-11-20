//! Worker reports: `data/<task>/report.md`, read on demand.
//!
//! A report is untrusted text. It is read only when it is a regular file that really lives
//! under the home's `data` directory, bounded in size, and rendered by the UI through the
//! shared sanitizing Markdown component.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::home::FmHome;
use crate::terminal::TaskId;

/// The most bytes of a report shown; a longer one is cut at a line and marked.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// A report as read from disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReportDoc {
    pub task_id: String,
    /// The Markdown source, untrusted. Render it only through the shared Markdown component.
    pub markdown: String,
    /// The file's size in bytes.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub bytes: u64,
    /// True when the file was longer than [`MAX_BYTES`] and only its start is here.
    pub truncated: bool,
    /// Last modified, in Unix seconds, when the file system says.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub modified: Option<i64>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReportError {
    #[error("the report is a link, which is not followed")]
    Link,
    #[error("the report is outside the home's data directory")]
    Outside,
    #[error("the report is not a regular file")]
    NotFile,
    #[error("the report could not be read: {0}")]
    Io(String),
}

/// Where the report of `id` lives.
pub fn report_path(home: &FmHome, id: &TaskId) -> PathBuf {
    home.root().join("data").join(id.as_str()).join("report.md")
}

/// Reads the report of `id`. `Ok(None)` means the task has no report yet.
pub async fn read(home: &FmHome, id: &TaskId) -> Result<Option<ReportDoc>, ReportError> {
    let data = home.root().join("data");
    let path = report_path(home, id);
    let id = id.to_string();
    tokio::task::spawn_blocking(move || read_blocking(&data, &path, id))
        .await
        .map_err(|e| ReportError::Io(e.to_string()))?
}

fn read_blocking(
    data: &Path,
    path: &Path,
    task_id: String,
) -> Result<Option<ReportDoc>, ReportError> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

    let io = |e: std::io::Error| ReportError::Io(e.kind().to_string());
    // Neither the task directory nor the report may be a link.
    for p in [path.parent().unwrap_or(path), path] {
        match std::fs::symlink_metadata(p) {
            Ok(m) if m.file_type().is_symlink() => return Err(ReportError::Link),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(e)),
        }
    }
    let canonical = path.canonicalize().map_err(io)?;
    let data = data.canonicalize().map_err(io)?;
    if !canonical.starts_with(&data) {
        return Err(ReportError::Outside);
    }
    // Open without following a final link and without blocking on a FIFO, then check the
    // handle itself, so a file swapped in after the checks above is caught.
    // Values from <fcntl.h>; std exposes custom_flags but not the constants.
    #[cfg(target_os = "macos")]
    const O_NOFOLLOW: i32 = 0x0100;
    #[cfg(target_os = "macos")]
    const O_NONBLOCK: i32 = 0x0004;
    #[cfg(not(target_os = "macos"))]
    const O_NOFOLLOW: i32 = 0o400_000;
    #[cfg(not(target_os = "macos"))]
    const O_NONBLOCK: i32 = 0o4000;
    // ELOOP: the final component was a link.
    #[cfg(target_os = "macos")]
    const ELOOP: i32 = 62;
    #[cfg(not(target_os = "macos"))]
    const ELOOP: i32 = 40;
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(&canonical)
        .map_err(|e| {
            if e.raw_os_error() == Some(ELOOP) {
                ReportError::Link
            } else {
                io(e)
            }
        })?;
    let meta = f.metadata().map_err(io)?;
    if !meta.is_file() {
        return Err(ReportError::NotFile);
    }
    // The opened file must be the one that was checked to be inside the data directory.
    let checked = std::fs::symlink_metadata(&canonical).map_err(io)?;
    if (checked.dev(), checked.ino()) != (meta.dev(), meta.ino()) {
        return Err(ReportError::Outside);
    }
    let mut buf = Vec::new();
    (&mut f).take(MAX_BYTES).read_to_end(&mut buf).map_err(io)?;
    let truncated = meta.len() > MAX_BYTES;
    if truncated {
        // Cut on a line so a half-read character or table row is not shown.
        if let Some(nl) = buf.iter().rposition(|b| *b == b'\n') {
            buf.truncate(nl + 1);
        }
    }
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok());
    Ok(Some(ReportDoc {
        task_id,
        markdown: String::from_utf8_lossy(&buf).into_owned(),
        bytes: meta.len(),
        truncated,
        modified,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_home() -> (tempfile::TempDir, FmHome) {
        let dir = tempfile::tempdir().unwrap();
        for d in ["state", "bin", "data"] {
            std::fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        std::fs::write(dir.path().join("bin/fm-bearings-snapshot.sh"), "").unwrap();
        std::fs::write(dir.path().join("bin/fm-inbox.sh"), "").unwrap();
        let home = FmHome::at(dir.path()).unwrap();
        (dir, home)
    }

    fn id() -> TaskId {
        TaskId::parse("app-login-fix").unwrap()
    }

    fn task_dir(dir: &tempfile::TempDir) -> PathBuf {
        let d = dir.path().join("data/app-login-fix");
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn none_when_absent() {
        let (_dir, home) = fake_home();
        assert_eq!(read(&home, &id()).await, Ok(None));
    }

    #[tokio::test]
    async fn reads_a_report() {
        let (dir, home) = fake_home();
        std::fs::write(task_dir(&dir).join("report.md"), "# Done\n\nAll green.\n").unwrap();
        let r = read(&home, &id()).await.unwrap().unwrap();
        assert_eq!(r.markdown, "# Done\n\nAll green.\n");
        assert_eq!(r.bytes, 19);
        assert!(!r.truncated);
        assert!(r.modified.is_some());
    }

    #[tokio::test]
    async fn cuts_a_long_report_on_a_line() {
        let (dir, home) = fake_home();
        let line = format!("{}\n", "y".repeat(99));
        std::fs::write(task_dir(&dir).join("report.md"), line.repeat(20_000)).unwrap();
        let r = read(&home, &id()).await.unwrap().unwrap();
        assert!(r.truncated);
        assert!(r.markdown.len() as u64 <= MAX_BYTES);
        assert!(r.markdown.ends_with('\n'));
        assert_eq!(r.bytes, 2_000_000);
    }

    #[tokio::test]
    async fn refuses_links_out() {
        let (dir, home) = fake_home();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("report.md"), "secret").unwrap();
        let task = dir.path().join("data/app-login-fix");
        std::os::unix::fs::symlink(outside.path(), &task).unwrap();
        assert_eq!(read(&home, &id()).await, Err(ReportError::Link));
        std::fs::remove_file(&task).unwrap();
        std::fs::create_dir_all(&task).unwrap();
        std::os::unix::fs::symlink(outside.path().join("report.md"), task.join("report.md"))
            .unwrap();
        assert_eq!(read(&home, &id()).await, Err(ReportError::Link));
    }

    #[tokio::test]
    async fn refuses_a_directory_and_a_fifo() {
        let (dir, home) = fake_home();
        std::fs::create_dir_all(task_dir(&dir).join("report.md")).unwrap();
        assert_eq!(read(&home, &id()).await, Err(ReportError::NotFile));
        std::fs::remove_dir(task_dir(&dir).join("report.md")).unwrap();
        let fifo = task_dir(&dir).join("report.md");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(made.success());
        assert_eq!(read(&home, &id()).await, Err(ReportError::NotFile));
    }
}
