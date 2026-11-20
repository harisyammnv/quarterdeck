//! Contained, bounded reads.
//!
//! The signatures match `core/src/safe_read.rs` from the integration contract, which the
//! foundation provides; until it lands this module stands in for it, and the integration
//! commit replaces every call with `crate::safe_read`.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use tokio::io::AsyncReadExt;

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("it resolves outside the folder it was listed in")]
    Escapes,
    #[error("it is not a regular file")]
    NotFile,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Reads at most `max_bytes` of `rel_or_abs` (relative to `root` when relative), which must
/// resolve inside `root` after symlinks. Invalid UTF-8 is replaced.
pub async fn read_text_under(
    root: &Path,
    rel_or_abs: &Path,
    max_bytes: u64,
) -> Result<String, ReadError> {
    let bytes = read_bytes_under(root, rel_or_abs, max_bytes).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub async fn read_bytes_under(
    root: &Path,
    rel_or_abs: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, ReadError> {
    let root = tokio::fs::canonicalize(root).await?;
    let real = tokio::fs::canonicalize(root.join(rel_or_abs)).await?;
    if !real.starts_with(&root) {
        return Err(ReadError::Escapes);
    }
    let file = tokio::fs::File::open(&real).await?;
    if !file.metadata().await?.is_file() {
        return Err(ReadError::NotFile);
    }
    let mut buf = Vec::new();
    file.take(max_bytes).read_to_end(&mut buf).await?;
    Ok(buf)
}

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

/// Reads a document up to `limit` bytes, reporting whether it was longer. A character the
/// limit cut in half is dropped rather than shown as a replacement mark.
pub async fn read_doc(root: &Path, path: &Path, limit: u64) -> Result<(String, bool), ReadError> {
    let mut bytes = read_bytes_under(root, path, limit.saturating_add(1)).await?;
    let truncated = u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit;
    if truncated {
        bytes.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        if let Err(e) = std::str::from_utf8(&bytes) {
            if e.error_len().is_none() {
                bytes.truncate(e.valid_up_to());
            }
        }
    }
    Ok((String::from_utf8_lossy(&bytes).into_owned(), truncated))
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
        std::fs::write(&p, b"ok \xff bad").unwrap();
        assert_eq!(
            read_doc(dir.path(), &p, 100).await.unwrap().0,
            "ok \u{fffd} bad"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinks_out_of_the_root_are_refused() {
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.md");
        std::fs::write(&secret, "secret").unwrap();
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("notes.md");
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        assert!(matches!(
            read_doc(root.path(), &link, 100).await,
            Err(ReadError::Escapes)
        ));
        assert!(matches!(
            read_doc(root.path(), Path::new("../x"), 100).await,
            Err(ReadError::Io(_) | ReadError::Escapes)
        ));
        let real = root.path().join("real.md");
        std::fs::write(&real, "inside").unwrap();
        let inner = root.path().join("inner.md");
        std::os::unix::fs::symlink(&real, &inner).unwrap();
        assert_eq!(
            read_doc(root.path(), &inner, 100).await.unwrap().0,
            "inside"
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
    }
}
