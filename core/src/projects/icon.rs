//! Project icons, from image files a project's clone already contains.
//!
//! Resolution follows the design's order, first match wins:
//! 1. an icon the repository declares for tools at a known path (`.quarterdeck/`, `.github/`);
//! 2. a team pack mapping, and 3. a forge avatar: not in Phase 1, because packs do not exist
//!    yet and the app makes no network calls (contract ruling 5);
//! 4. a favicon, app icon or logo at a fixed list of common paths;
//! 5. otherwise nothing, and the window draws a monogram.
//!
//! The roots are the project clone and worktree paths the snapshot names for the project's
//! tasks. Only the fixed candidate paths are tried, at most two directories deep, with no walk.
//! Every read goes through [`crate::safe_read`] (the file must stay under the root after
//! symlinks resolve and be a regular file), is capped at [`MAX_BYTES`], and must start with
//! the bytes of a known image type. The answer is a `data:` URL for an `<img>`: the CSP allows
//! `data:` images, and an `<img>` never runs SVG script.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use crate::engine::HomeState;
use crate::safe_read;

/// Paths tried in order, relative to the project's clone.
pub const CANDIDATES: &[&str] = &[
    // 1. Declared for tools.
    ".quarterdeck/icon.svg",
    ".quarterdeck/icon.png",
    ".github/icon.svg",
    ".github/icon.png",
    ".github/logo.svg",
    ".github/logo.png",
    // 4. App icons and favicons, vector first.
    "icon.svg",
    "logo.svg",
    "public/icon.svg",
    "public/favicon.svg",
    "public/logo.svg",
    "app/icon.svg",
    "src/app/icon.svg",
    "web/app/icon.svg",
    "web/public/favicon.svg",
    "website/public/favicon.svg",
    "static/favicon.svg",
    "assets/icon.svg",
    "assets/logo.svg",
    "assets/favicon.svg",
    "docs/logo.svg",
    "icon.png",
    "logo.png",
    "public/icon.png",
    "public/favicon.png",
    "public/logo.png",
    "public/apple-touch-icon.png",
    "public/images/logo.png",
    "app/icon.png",
    "src/app/icon.png",
    "app/icons/icon.png",
    "src-tauri/icons/icon.png",
    "ui/app/icon.png",
    "ui/public/favicon.png",
    "static/favicon.png",
    "assets/icon.png",
    "assets/logo.png",
    "assets/favicon.png",
    "docs/logo.png",
    "favicon.ico",
    "public/favicon.ico",
    "app/favicon.ico",
    "src/app/favicon.ico",
    "static/favicon.ico",
    "assets/favicon.ico",
];

/// The largest icon file read (contract section 7).
pub const MAX_BYTES: u64 = 256 * 1024;

/// How long a miss is remembered before the files are looked at again.
const MISS_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IconError {
    #[error("{0:?} is not a project name")]
    BadName(String),
}

#[derive(Clone, Debug)]
enum Entry {
    /// The file that matched, its modification time, and the encoded icon.
    Hit(PathBuf, Option<SystemTime>, String),
    Miss(Instant),
}

/// Resolves project icons and remembers them by root: a hit is reused while the matched
/// file's modification time is unchanged, a miss for a few minutes.
#[derive(Debug, Default)]
pub struct IconCache {
    entries: Mutex<HashMap<PathBuf, Entry>>,
}

impl IconCache {
    /// The icon of project `name`, from the clone of a snapshot task in that project, else one
    /// of its worktrees. `None` means "draw a monogram".
    pub async fn for_project(
        &self,
        state: &HomeState,
        name: &str,
    ) -> Result<Option<String>, IconError> {
        if !super::is_plain_component(name) {
            return Err(IconError::BadName(name.to_string()));
        }
        self.prune(state);
        for root in roots(state, name) {
            if let Some(url) = self.get(&root).await {
                return Ok(Some(url));
            }
        }
        Ok(None)
    }

    /// The icon under `root`, cached.
    pub async fn get(&self, root: &Path) -> Option<String> {
        let cached = self.lock().get(root).cloned();
        match cached {
            Some(Entry::Hit(path, mtime, url)) if modified(&path).await == mtime => {
                return Some(url)
            }
            Some(Entry::Miss(at)) if at.elapsed() < MISS_TTL => return None,
            _ => {}
        }
        let found = resolve_with_path(root).await;
        let entry = match &found {
            Some((path, url)) => Entry::Hit(path.clone(), modified(path).await, url.clone()),
            None => Entry::Miss(Instant::now()),
        };
        self.lock().insert(root.to_path_buf(), entry);
        found.map(|(_, url)| url)
    }

    /// Forgets roots no task in the snapshot names any more, so worktrees that come and go
    /// over a long session do not grow the cache.
    fn prune(&self, state: &HomeState) {
        let Some(snap) = state.snapshot.as_deref() else {
            return;
        };
        let live: std::collections::HashSet<&Path> = snap
            .tasks
            .iter()
            .flat_map(|t| [t.project.as_deref(), t.worktree.as_deref()])
            .flatten()
            .collect();
        self.lock().retain(|root, _| live.contains(root.as_path()));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The clone paths, then the worktree paths, of the snapshot's tasks in project `name`.
fn roots(state: &HomeState, name: &str) -> Vec<PathBuf> {
    let Some(snap) = state.snapshot.as_deref() else {
        return Vec::new();
    };
    let tasks: Vec<_> = snap
        .tasks
        .iter()
        .filter(|t| t.project_name().as_deref() == Some(name))
        .collect();
    let mut out: Vec<PathBuf> = Vec::new();
    for p in tasks
        .iter()
        .filter_map(|t| t.project.clone())
        .chain(tasks.iter().filter_map(|t| t.worktree.clone()))
    {
        if p.is_absolute() && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

async fn modified(path: &Path) -> Option<SystemTime> {
    tokio::fs::metadata(path)
        .await
        .and_then(|m| m.modified())
        .ok()
}

/// The first usable candidate under `root`, as a `data:` URL.
pub async fn resolve(root: &Path) -> Option<String> {
    resolve_with_path(root).await.map(|(_, url)| url)
}

async fn resolve_with_path(root: &Path) -> Option<(PathBuf, String)> {
    for rel in CANDIDATES {
        if let Some(hit) = load(root, Path::new(rel)).await {
            return Some(hit);
        }
    }
    None
}

async fn load(root: &Path, rel: &Path) -> Option<(PathBuf, String)> {
    let real = safe_read::resolve_under(root, rel).await.ok()?;
    let read = safe_read::read_bytes_capped(root, &real, MAX_BYTES)
        .await
        .ok()?;
    if read.truncated || read.content.is_empty() {
        return None;
    }
    let mime = sniff(&read.content)?;
    Some((
        real,
        format!("data:{mime};base64,{}", base64(&read.content)),
    ))
}

/// The image type, from the file's first bytes. Anything else is refused.
fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if b.starts_with(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.len() > 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else if is_svg(b) {
        Some("image/svg+xml")
    } else {
        None
    }
}

/// Text that opens an `<svg>` element (after an optional XML prolog, comments or BOM) and
/// carries no script. An `<img>` would not run script anyway; refusing it keeps the rule simple.
fn is_svg(b: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(b) else {
        return false;
    };
    let head = text.trim_start_matches('\u{feff}').trim_start();
    let opens = head.starts_with("<svg") || head.starts_with("<?xml") || head.starts_with("<!--");
    let lower = text.to_ascii_lowercase();
    opens
        && lower.contains("<svg")
        && !lower.contains("<script")
        && !lower.contains("<foreignobject")
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ABC[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::snapshot::{Snapshot, Task};

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#;
    const ICO: &[u8] = &[0, 0, 1, 0, 1, 0];

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[test]
    fn base64_matches_known_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn candidates_stay_shallow_and_relative() {
        for c in CANDIDATES {
            assert!(!c.starts_with('/') && !c.contains(".."), "{c}");
            assert!(
                c.matches('/').count() <= 2,
                "{c} is deeper than two directories"
            );
        }
    }

    #[tokio::test]
    async fn declared_icon_beats_a_favicon() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "public/favicon.ico", ICO);
        put(dir.path(), ".github/icon.png", PNG);
        let url = resolve(dir.path()).await.unwrap();
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
    }

    #[tokio::test]
    async fn vector_favicon_is_found_and_typed() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "public/favicon.svg", SVG.as_bytes());
        put(dir.path(), "public/favicon.ico", ICO);
        assert!(resolve(dir.path())
            .await
            .unwrap()
            .starts_with("data:image/svg+xml;base64,"));
    }

    #[tokio::test]
    async fn refuses_wrong_bytes_script_and_oversize() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "icon.png", b"not an image at all");
        put(
            dir.path(),
            "icon.svg",
            br#"<svg><script>alert(1)</script></svg>"#,
        );
        put(
            dir.path(),
            "logo.png",
            &[PNG, &vec![0; MAX_BYTES as usize]].concat(),
        );
        assert_eq!(resolve(dir.path()).await, None);
        put(dir.path(), "favicon.ico", ICO);
        assert!(resolve(dir.path())
            .await
            .unwrap()
            .starts_with("data:image/x-icon;"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_a_symlink_out_of_the_clone() {
        let outside = tempfile::tempdir().unwrap();
        put(outside.path(), "secret.png", PNG);
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.png"),
            dir.path().join("icon.png"),
        )
        .unwrap();
        assert_eq!(resolve(dir.path()).await, None);
    }

    #[tokio::test]
    async fn cache_reuses_hits_until_the_file_changes_and_remembers_misses() {
        let dir = tempfile::tempdir().unwrap();
        let cache = IconCache::default();
        assert_eq!(cache.get(dir.path()).await, None);
        put(dir.path(), "icon.png", PNG);
        // A miss is remembered for a while.
        assert_eq!(cache.get(dir.path()).await, None);
        let cache = IconCache::default();
        assert!(cache
            .get(dir.path())
            .await
            .unwrap()
            .starts_with("data:image/png;"));
        // A rewrite with a later modification time is noticed.
        put(dir.path(), "icon.png", ICO);
        let f = std::fs::File::options()
            .write(true)
            .open(dir.path().join("icon.png"))
            .unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(5))
            .unwrap();
        assert!(cache
            .get(dir.path())
            .await
            .unwrap()
            .starts_with("data:image/x-icon;"));
    }

    #[tokio::test]
    async fn project_lookup_checks_names_and_uses_snapshot_roots() {
        let clone = tempfile::tempdir().unwrap();
        let wt = tempfile::tempdir().unwrap();
        put(wt.path(), "public/favicon.svg", SVG.as_bytes());
        let t = Task {
            id: "t1".into(),
            project: Some(clone.path().to_path_buf()),
            worktree: Some(wt.path().to_path_buf()),
            ..Task::default()
        };
        let name = t.project_name().unwrap();
        let state = HomeState {
            snapshot: Some(Arc::new(Snapshot {
                tasks: vec![t],
                ..Snapshot::default()
            })),
            ..HomeState::default()
        };
        let cache = IconCache::default();
        // The clone has no icon, so the worktree's is used.
        let url = cache.for_project(&state, &name).await.unwrap().unwrap();
        assert!(url.starts_with("data:image/svg+xml;"));
        assert_eq!(cache.for_project(&state, "other").await, Ok(None));
        assert_eq!(
            cache.for_project(&state, "..").await,
            Err(IconError::BadName("..".into()))
        );
        assert_eq!(
            cache.for_project(&HomeState::default(), "x").await,
            Ok(None)
        );
    }
}
