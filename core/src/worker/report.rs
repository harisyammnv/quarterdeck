//! A worker's report: `data/<task>/report.md`.
//!
//! The file is read as untrusted text, bounded in size, and only when it is a regular file
//! that really lives under the home's `data` directory: a symlink anywhere on the way out is
//! refused rather than followed.

use std::path::PathBuf;

use serde::Serialize;

use super::TaskId;
use crate::home::FmHome;

/// The most bytes of a report shown; a longer one is cut at the end and marked.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// A report as read from disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TaskReport {
    pub task_id: String,
    /// The Markdown source, untrusted. Render it only through a sanitizer.
    pub markdown: String,
    /// The file's size in bytes.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub bytes: u64,
    /// True when the file was longer than [`MAX_BYTES`] and only its start is here.
    pub truncated: bool,
    /// Last modified, in milliseconds since the epoch, when the file system says.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub modified_ms: Option<i64>,
}

/// The outcome of reading a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReportRead {
    Report(TaskReport),
    /// The task has no report yet.
    Missing,
    /// The report exists but cannot be shown: not a regular file, outside the home, or
    /// unreadable.
    Refused {
        message: String,
    },
}

fn report_path(home: &FmHome, id: &TaskId) -> PathBuf {
    home.root().join("data").join(id.as_str()).join("report.md")
}

/// Reads the report of `id`.
pub fn read(home: &FmHome, id: &TaskId) -> ReportRead {
    use std::io::Read;
    let path = report_path(home, id);
    let refused = |message: &str| ReportRead::Refused {
        message: message.to_string(),
    };
    // Neither the task directory nor the report may be a symlink.
    for p in [path.parent().unwrap_or(&path), path.as_path()] {
        match std::fs::symlink_metadata(p) {
            Ok(m) if m.file_type().is_symlink() => {
                return refused("the report is a link, which is not followed")
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ReportRead::Missing,
            Err(_) => return refused("the report could not be read"),
        }
    }
    let data_dir = home.root().join("data");
    let inside = match (path.canonicalize(), data_dir.canonicalize()) {
        (Ok(p), Ok(d)) => p.starts_with(d),
        _ => false,
    };
    if !inside {
        return refused("the report is outside the home's data directory");
    }
    let Ok(mut f) = std::fs::File::open(&path) else {
        return refused("the report could not be read");
    };
    let Ok(meta) = f.metadata() else {
        return refused("the report could not be read");
    };
    if !meta.is_file() {
        return refused("the report is not a regular file");
    }
    let mut buf = Vec::new();
    if (&mut f).take(MAX_BYTES).read_to_end(&mut buf).is_err() {
        return refused("the report could not be read");
    }
    let truncated = meta.len() > MAX_BYTES;
    if truncated {
        // Cut on a line so a half-read multi-byte character or table row is not shown.
        if let Some(nl) = buf.iter().rposition(|b| *b == b'\n') {
            buf.truncate(nl + 1);
        }
    }
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_millis()).ok());
    ReportRead::Report(TaskReport {
        task_id: id.to_string(),
        markdown: String::from_utf8_lossy(&buf).into_owned(),
        bytes: meta.len(),
        truncated,
        modified_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::super::testutil::fake_home;
    use super::*;

    fn id() -> TaskId {
        TaskId::parse("app-login-fix").unwrap()
    }

    #[test]
    fn missing_when_absent() {
        let (_dir, home) = fake_home();
        assert_eq!(read(&home, &id()), ReportRead::Missing);
    }

    #[test]
    fn reads_a_report() {
        let (dir, home) = fake_home();
        let d = dir.path().join("data/app-login-fix");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("report.md"), "# Done\n\nAll green.\n").unwrap();
        let ReportRead::Report(r) = read(&home, &id()) else {
            panic!("expected a report")
        };
        assert_eq!(r.markdown, "# Done\n\nAll green.\n");
        assert_eq!(r.bytes, 19);
        assert!(!r.truncated);
        assert!(r.modified_ms.is_some());
    }

    #[test]
    fn cuts_a_long_report_on_a_line() {
        let (dir, home) = fake_home();
        let d = dir.path().join("data/app-login-fix");
        std::fs::create_dir_all(&d).unwrap();
        let line = format!("{}\n", "y".repeat(99));
        std::fs::write(d.join("report.md"), line.repeat(20_000)).unwrap();
        let ReportRead::Report(r) = read(&home, &id()) else {
            panic!("expected a report")
        };
        assert!(r.truncated);
        assert!(r.markdown.len() as u64 <= MAX_BYTES);
        assert!(r.markdown.ends_with('\n'));
        assert_eq!(r.bytes, 2_000_000);
    }

    #[test]
    fn refuses_links_out() {
        let (dir, home) = fake_home();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("report.md"), "secret").unwrap();
        // A linked task directory.
        std::os::unix::fs::symlink(outside.path(), dir.path().join("data/app-login-fix")).unwrap();
        assert!(matches!(read(&home, &id()), ReportRead::Refused { .. }));
        // A linked report file.
        std::fs::remove_file(dir.path().join("data/app-login-fix")).unwrap();
        std::fs::create_dir_all(dir.path().join("data/app-login-fix")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("report.md"),
            dir.path().join("data/app-login-fix/report.md"),
        )
        .unwrap();
        assert!(matches!(read(&home, &id()), ReportRead::Refused { .. }));
    }

    #[test]
    fn refuses_a_directory() {
        let (dir, home) = fake_home();
        std::fs::create_dir_all(dir.path().join("data/app-login-fix/report.md")).unwrap();
        assert!(matches!(read(&home, &id()), ReportRead::Refused { .. }));
    }
}
