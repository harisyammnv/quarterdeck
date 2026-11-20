//! Directory listing and file metadata for the memory and skills scans. Reading file
//! contents always goes through [`crate::safe_read`].

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::safe_read::{self, ReadError};
/// Size and modification time of a regular file (after symlinks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meta {
    pub bytes: u64,
    pub modified: Option<i64>,
}

pub fn file_meta(path: &Path) -> Option<Meta> {
    let m = std::fs::metadata(path).ok()?;
    if !m.is_file() {
        return None;
    }
    let modified = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_secs()).ok());
    Some(Meta {
        bytes: m.len(),
        modified,
    })
}

/// Names of the visible subdirectories of `dir`, sorted, at most `cap`. Hidden entries and
/// names that are not valid UTF-8 are skipped. A missing or unreadable directory has none.
pub fn child_dirs(dir: &Path, cap: usize) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if name.starts_with('.') || name.chars().any(char::is_control) {
                return None;
            }
            // Follows symlinks: a linked skill folder counts.
            std::fs::metadata(e.path()).ok()?.is_dir().then_some(name)
        })
        .collect();
    names.sort();
    names.truncate(cap);
    names
}

/// The resolved path of `path`, when it exists.
pub fn real_path(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Reads a document under `root`, up to `limit` bytes; true when it was longer.
pub async fn read_doc(root: &Path, path: &Path, limit: u64) -> Result<(String, bool), ReadError> {
    let capped = safe_read::read_text_capped(root, path, limit).await?;
    Ok((capped.content, capped.truncated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn limit_and_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.md");
        std::fs::write(&p, "héllo").unwrap();
        // "h" plus the first byte of "é" fits in 2 bytes; the cut character is dropped.
        assert_eq!(
            read_doc(dir.path(), &p, 2).await.unwrap(),
            ("h".into(), true)
        );
        assert_eq!(
            read_doc(dir.path(), &p, 100).await.unwrap(),
            ("héllo".into(), false)
        );
    }

    #[test]
    fn directories_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        for d in ["b", "a", ".hidden"] {
            std::fs::create_dir(dir.path().join(d)).unwrap();
        }
        std::fs::write(dir.path().join("file"), "").unwrap();
        assert_eq!(child_dirs(dir.path(), 10), ["a", "b"]);
        assert_eq!(child_dirs(dir.path(), 1), ["a"]);
        assert!(file_meta(&dir.path().join("a")).is_none());
        assert!(file_meta(&dir.path().join("nope")).is_none());
        assert!(child_dirs(&dir.path().join("nope"), 10).is_empty());
        assert!(real_path(&dir.path().join("nope")).is_none());
    }
}
