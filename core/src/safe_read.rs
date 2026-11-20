//! Bounded reads that cannot leave a root.
//!
//! Every direct file read in an area goes through here. The target is resolved against the
//! root, both are canonicalized (so symlinks are followed before the check, not after), and
//! anything that does not stay under the root, or is not a regular file, is refused. Reads
//! stop at `max_bytes`.

use std::path::{Path, PathBuf};

use tokio::io::AsyncReadExt;

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("not found")]
    NotFound,
    #[error("the path leaves the folder it must stay in")]
    Escapes,
    #[error("not a regular file")]
    NotAFile,
    #[error("could not read: {0}")]
    Io(std::io::Error),
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::Io(e)
        }
    }
}

/// Text read from a file, and whether it was cut at the size cap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capped<T> {
    pub content: T,
    pub truncated: bool,
}

/// Resolves `target` (relative to `root`, or absolute) and proves it is a regular file under
/// `root` after every symlink is resolved.
pub async fn resolve_under(root: &Path, target: &Path) -> Result<PathBuf, ReadError> {
    let root = tokio::fs::canonicalize(root).await?;
    let joined = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    let real = tokio::fs::canonicalize(&joined).await?;
    if !real.starts_with(&root) {
        return Err(ReadError::Escapes);
    }
    if !tokio::fs::metadata(&real).await?.is_file() {
        return Err(ReadError::NotAFile);
    }
    Ok(real)
}

/// Reads at most `max_bytes` of a file under `root`, and says whether it stopped early.
pub async fn read_bytes_capped(
    root: &Path,
    target: &Path,
    max_bytes: u64,
) -> Result<Capped<Vec<u8>>, ReadError> {
    let real = resolve_under(root, target).await?;
    let file = tokio::fs::File::open(&real).await?;
    let mut bytes = Vec::new();
    // One byte past the cap tells a file of exactly max_bytes from a longer one.
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .await?;
    let truncated = bytes.len() as u64 > max_bytes;
    bytes.truncate(usize::try_from(max_bytes).unwrap_or(usize::MAX));
    Ok(Capped {
        content: bytes,
        truncated,
    })
}

/// Reads at most `max_bytes` of a text file under `root`, cut back to a whole character,
/// with invalid UTF-8 replaced.
pub async fn read_text_capped(
    root: &Path,
    target: &Path,
    max_bytes: u64,
) -> Result<Capped<String>, ReadError> {
    let Capped {
        mut content,
        truncated,
    } = read_bytes_capped(root, target, max_bytes).await?;
    if truncated {
        drop_partial_char(&mut content);
    }
    Ok(Capped {
        content: String::from_utf8_lossy(&content).into_owned(),
        truncated,
    })
}

/// Drops a UTF-8 sequence cut short at the end, so a capped read never ends on half a
/// character.
fn drop_partial_char(bytes: &mut Vec<u8>) {
    let len = bytes.len();
    let mut start = len;
    // Walk back over at most three continuation bytes to the last lead byte.
    while start > 0 && len - start < 4 {
        start -= 1;
        if bytes[start] & 0b1100_0000 != 0b1000_0000 {
            break;
        }
    }
    let Some(&lead) = bytes.get(start) else {
        return;
    };
    let need = match lead {
        b if b & 0b1000_0000 == 0 => 1,
        b if b & 0b1110_0000 == 0b1100_0000 => 2,
        b if b & 0b1111_0000 == 0b1110_0000 => 3,
        b if b & 0b1111_1000 == 0b1111_0000 => 4,
        _ => return,
    };
    if start + need > len {
        bytes.truncate(start);
    }
}

/// Reads at most `max_bytes` of a text file under `root`.
pub async fn read_text_under(
    root: &Path,
    rel_or_abs: &Path,
    max_bytes: u64,
) -> Result<String, ReadError> {
    Ok(read_text_capped(root, rel_or_abs, max_bytes).await?.content)
}

/// Reads at most `max_bytes` of a file under `root`.
pub async fn read_bytes_under(
    root: &Path,
    rel_or_abs: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, ReadError> {
    Ok(read_bytes_capped(root, rel_or_abs, max_bytes)
        .await?
        .content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("root/sub")).unwrap();
        std::fs::write(d.path().join("root/sub/a.md"), "# hello\n").unwrap();
        std::fs::write(d.path().join("outside.md"), "secret").unwrap();
        d
    }

    #[tokio::test]
    async fn reads_relative_and_absolute_paths_inside() {
        let d = tree();
        let root = d.path().join("root");
        assert_eq!(
            read_text_under(&root, Path::new("sub/a.md"), 1024)
                .await
                .unwrap(),
            "# hello\n"
        );
        let abs = root.join("sub/a.md");
        assert_eq!(
            read_text_under(&root, &abs, 1024).await.unwrap(),
            "# hello\n"
        );
    }

    #[tokio::test]
    async fn refuses_escapes_by_dots_absolute_paths_and_symlinks() {
        let d = tree();
        let root = d.path().join("root");
        assert!(matches!(
            read_text_under(&root, Path::new("../outside.md"), 1024).await,
            Err(ReadError::Escapes)
        ));
        assert!(matches!(
            read_text_under(&root, &d.path().join("outside.md"), 1024).await,
            Err(ReadError::Escapes)
        ));
        std::os::unix::fs::symlink(d.path().join("outside.md"), root.join("link.md")).unwrap();
        assert!(matches!(
            read_text_under(&root, Path::new("link.md"), 1024).await,
            Err(ReadError::Escapes)
        ));
    }

    #[tokio::test]
    async fn refuses_directories_and_reports_missing() {
        let d = tree();
        let root = d.path().join("root");
        assert!(matches!(
            read_text_under(&root, Path::new("sub"), 1024).await,
            Err(ReadError::NotAFile)
        ));
        assert!(matches!(
            read_text_under(&root, Path::new("nope.md"), 1024).await,
            Err(ReadError::NotFound)
        ));
    }

    #[tokio::test]
    async fn caps_size_and_says_so() {
        let d = tree();
        let root = d.path().join("root");
        std::fs::write(root.join("big.md"), "é".repeat(100)).unwrap();
        let c = read_text_capped(&root, Path::new("big.md"), 11)
            .await
            .unwrap();
        assert!(c.truncated);
        assert_eq!(c.content, "é".repeat(5), "cut back to a whole character");
        let exact = read_bytes_capped(&root, Path::new("sub/a.md"), 8)
            .await
            .unwrap();
        assert!(!exact.truncated);
        assert_eq!(exact.content.len(), 8);
    }
}
