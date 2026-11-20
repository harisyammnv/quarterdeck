//! The memory browser: Firstmate's preferences, learnings, task briefs and project guides,
//! read from the home as Markdown and never written.
//!
//! The UI never names a path. [MemoryIndex::scan] lists documents under fixed, allow-listed
//! locations and gives each an opaque id; [MemoryIndex::read] maps an id back to the path the
//! scan found and reads it through a contained, bounded read that re-checks the file still
//! resolves inside the folder it was listed in. Markdown is sent as text and rendered by the
//! UI's shared, sanitizing Markdown component.

pub mod fsread;
pub mod search;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub use search::{MemoryHit, MemorySearch};

/// Documents larger than this are read up to the limit and marked truncated.
pub const MAX_DOC_BYTES: u64 = 1024 * 1024;
/// At most this many task folders are listed for briefs.
pub const MAX_BRIEFS: usize = 2000;

/// What a memory document is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum MemoryKind {
    /// `data/captain.md`: how the operator likes work delivered.
    Preferences,
    /// `data/captain-shared.md`: preferences inherited by second mates.
    SharedPreferences,
    /// `data/learnings.md`: what the fleet has learned.
    Learnings,
    /// `data/<task>/brief.md`: the instructions a task was given.
    Brief,
    /// `AGENTS.md` (or `CLAUDE.md`) at the root of a project the fleet works on.
    ProjectGuide,
}

/// One document in the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemoryItem {
    /// Opaque id for `memory_read`.
    pub id: String,
    pub kind: MemoryKind,
    pub title: String,
    /// The task id or project name the document belongs to, when it has one.
    pub owner: Option<String>,
    /// A short display path relative to its root, such as `data/learnings.md`.
    pub label: String,
    /// Modification time, Unix seconds.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub modified: Option<i64>,
}

/// Everything the memory list shows: preferences, shared preferences and learnings first
/// (when present), then project guides by name, then briefs newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemoryView {
    pub items: Vec<MemoryItem>,
}

/// A document ready to render.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemoryDoc {
    pub item: MemoryItem,
    /// The Markdown source; untrusted, rendered only through the shared Markdown component.
    pub text: String,
    /// Top-level list items, for a count such as "38 entries".
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub entries: u32,
    /// The file was longer than [MAX_DOC_BYTES] and only its start is in `text`.
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("no memory document has the id {0:?}")]
    UnknownId(String),
    #[error("{0} could not be read: {1}")]
    Unreadable(String, fsread::ReadError),
}

/// A project the fleet works on, by display name and checkout path (from the snapshot).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRoot {
    pub name: String,
    pub path: PathBuf,
}

/// A scanned list plus the private id-to-path map behind it.
#[derive(Clone, Debug, Default)]
pub struct MemoryIndex {
    view: MemoryView,
    paths: HashMap<String, Located>,
}

#[derive(Clone, Debug)]
struct Located {
    path: PathBuf,
    /// The folder the file must stay inside, after symlinks are resolved.
    root: PathBuf,
}

const CORE_DOCS: [(&str, &str, MemoryKind, &str); 3] = [
    (
        "preferences",
        "captain.md",
        MemoryKind::Preferences,
        "Preferences",
    ),
    (
        "shared-preferences",
        "captain-shared.md",
        MemoryKind::SharedPreferences,
        "Shared preferences",
    ),
    (
        "learnings",
        "learnings.md",
        MemoryKind::Learnings,
        "Learnings",
    ),
];

const GUIDE_NAMES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

impl MemoryIndex {
    /// Lists the memory documents under the home's `data` directory and the given projects.
    /// Only file metadata is read; this is quick, but it touches the disk, so call it off the
    /// UI path.
    pub fn scan(data: &Path, projects: &[ProjectRoot]) -> Self {
        let mut index = Self::default();
        for (id, name, kind, title) in CORE_DOCS {
            let path = data.join(name);
            if let Some(meta) = fsread::file_meta(&path) {
                index.add(
                    data,
                    path,
                    MemoryItem {
                        id: id.to_string(),
                        kind,
                        title: title.to_string(),
                        owner: None,
                        label: format!("data/{name}"),
                        modified: meta.modified,
                    },
                );
            }
        }

        let mut projects: Vec<&ProjectRoot> = projects.iter().collect();
        projects.sort_by(|a, b| a.name.cmp(&b.name));
        projects.dedup_by(|a, b| a.name == b.name);
        for p in projects {
            for guide in GUIDE_NAMES {
                let path = p.path.join(guide);
                if let Some(meta) = fsread::file_meta(&path) {
                    index.add(
                        &p.path,
                        path,
                        MemoryItem {
                            id: format!("project:{}", p.name),
                            kind: MemoryKind::ProjectGuide,
                            title: p.name.clone(),
                            owner: Some(p.name.clone()),
                            label: guide.to_string(),
                            modified: meta.modified,
                        },
                    );
                    break;
                }
            }
        }

        let mut briefs = Vec::new();
        for task in fsread::child_dirs(data, MAX_BRIEFS) {
            let dir = data.join(&task);
            let path = dir.join("brief.md");
            let Some(meta) = fsread::file_meta(&path) else {
                continue;
            };
            briefs.push((
                dir,
                path,
                MemoryItem {
                    id: format!("brief:{task}"),
                    kind: MemoryKind::Brief,
                    title: task.clone(),
                    label: format!("data/{task}/brief.md"),
                    owner: Some(task),
                    modified: meta.modified,
                },
            ));
        }
        briefs.sort_by(|a, b| {
            b.2.modified
                .cmp(&a.2.modified)
                .then_with(|| a.2.id.cmp(&b.2.id))
        });
        for (dir, path, item) in briefs {
            index.add(&dir, path, item);
        }
        index
    }

    fn add(&mut self, root: &Path, path: PathBuf, item: MemoryItem) {
        self.paths.insert(
            item.id.clone(),
            Located {
                path,
                root: root.to_path_buf(),
            },
        );
        self.view.items.push(item);
    }

    pub fn view(&self) -> &MemoryView {
        &self.view
    }

    fn item(&self, id: &str) -> Option<&MemoryItem> {
        self.view.items.iter().find(|e| e.id == id)
    }

    /// Reads the text of document `id`, up to `limit` bytes; true when it was longer.
    pub async fn read_text(&self, id: &str, limit: u64) -> Result<(String, bool), MemoryError> {
        let loc = self
            .paths
            .get(id)
            .ok_or_else(|| MemoryError::UnknownId(id.to_string()))?;
        fsread::read_doc(&loc.root, &loc.path, limit)
            .await
            .map_err(|e| {
                let label = self
                    .item(id)
                    .map_or_else(|| id.to_string(), |i| i.label.clone());
                MemoryError::Unreadable(label, e)
            })
    }

    /// Reads document `id` for display.
    pub async fn read(&self, id: &str) -> Result<MemoryDoc, MemoryError> {
        let item = self
            .item(id)
            .cloned()
            .ok_or_else(|| MemoryError::UnknownId(id.to_string()))?;
        let (text, truncated) = self.read_text(id, MAX_DOC_BYTES).await?;
        Ok(MemoryDoc {
            entries: count_entries(&text),
            text,
            truncated,
            item,
        })
    }
}

/// Counts top-level list items, outside code fences.
pub fn count_entries(text: &str) -> u32 {
    let mut in_fence = false;
    let mut n = 0u32;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && (line.starts_with("- ") || line.starts_with("* ")) {
            n = n.saturating_add(1);
        }
    }
    n
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A home with invented memory: preferences, learnings, briefs and two project checkouts.
    pub(crate) struct Fixture {
        pub dir: tempfile::TempDir,
        pub projects: Vec<ProjectRoot>,
    }

    impl Fixture {
        pub fn data(&self) -> PathBuf {
            self.dir.path().join("data")
        }

        pub fn index(&self) -> MemoryIndex {
            MemoryIndex::scan(&self.data(), &self.projects)
        }
    }

    pub(crate) fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        let w = |p: &str, s: &str| {
            let p = r.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, s).unwrap();
        };
        w(
            "data/captain.md",
            "# Preferences\n\n- Keep pull requests small.\n- Ask before any production change.\n",
        );
        w("data/learnings.md", "# Learnings\n\n## Builds\n\n- The cache needs a warm-up run.\n  Continued line.\n- Tests run in parallel.\n\n```\n- not an entry\n```\n");
        w("data/backlog.md", "not memory");
        w(
            "data/task-a/brief.md",
            "# Task\n\nWarm the cache before the suite.\n",
        );
        w("data/task-a/report.md", "Reports belong to another area.\n");
        w("data/task-b/notes.md", "No brief here.\n");
        w(
            "p/ledger-api/AGENTS.md",
            "# Ledger API\n\nRun the tests first.\n",
        );
        w("p/portal-web/CLAUDE.md", "# Portal\n");
        w("p/empty-repo/README.md", "# Nothing\n");
        let projects = ["ledger-api", "portal-web", "empty-repo"]
            .iter()
            .map(|n| ProjectRoot {
                name: (*n).to_string(),
                path: r.join("p").join(n),
            })
            .collect();
        Fixture { dir, projects }
    }

    fn ids(idx: &MemoryIndex) -> Vec<String> {
        idx.view().items.iter().map(|e| e.id.clone()).collect()
    }

    #[test]
    fn scans_the_allowed_locations() {
        let f = fixture();
        let idx = f.index();
        assert_eq!(
            ids(&idx),
            [
                "preferences",
                "learnings",
                "project:ledger-api",
                "project:portal-web",
                "brief:task-a"
            ]
        );
        let items = &idx.view().items;
        assert_eq!(items[3].label, "CLAUDE.md");
        assert_eq!(items[4].owner.as_deref(), Some("task-a"));
        assert_eq!(items[4].label, "data/task-a/brief.md");
    }

    #[test]
    fn shared_preferences_listed_when_present() {
        let f = fixture();
        std::fs::write(f.data().join("captain-shared.md"), "- shared\n").unwrap();
        assert_eq!(
            ids(&f.index())[..3],
            ["preferences", "shared-preferences", "learnings"]
        );
    }

    #[test]
    fn duplicate_project_names_are_listed_once() {
        let f = fixture();
        let mut projects = f.projects.clone();
        projects.push(projects[0].clone());
        let idx = MemoryIndex::scan(&f.data(), &projects);
        assert_eq!(
            ids(&idx)
                .iter()
                .filter(|i| *i == "project:ledger-api")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn reads_and_counts_entries() {
        let f = fixture();
        let idx = f.index();
        let doc = idx.read("learnings").await.unwrap();
        assert_eq!(doc.entries, 2);
        assert!(!doc.truncated);
        assert!(doc.text.starts_with("# Learnings"));
        assert_eq!(doc.item.label, "data/learnings.md");
        let guide = idx.read("project:ledger-api").await.unwrap();
        assert!(guide.text.contains("Run the tests first."));
    }

    #[tokio::test]
    async fn unknown_and_path_like_ids_are_refused() {
        let f = fixture();
        let idx = f.index();
        for id in [
            "../../etc/passwd",
            "/etc/passwd",
            "brief:../task-a",
            "backlog",
            "data/captain.md",
            "report:task-a",
        ] {
            assert!(
                matches!(idx.read(id).await, Err(MemoryError::UnknownId(_))),
                "{id}"
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_guide_linking_out_of_its_project_is_refused() {
        let f = fixture();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.md"), "secret").unwrap();
        let guide = f.projects[0].path.join("AGENTS.md");
        std::fs::remove_file(&guide).unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret.md"), &guide).unwrap();
        let idx = f.index();
        let err = idx.read("project:ledger-api").await.unwrap_err();
        assert!(
            matches!(err, MemoryError::Unreadable(_, fsread::ReadError::Escapes)),
            "{err}"
        );
    }

    #[tokio::test]
    async fn large_documents_are_truncated() {
        let f = fixture();
        let big = "- line\n".repeat(200_000);
        std::fs::write(f.data().join("learnings.md"), &big).unwrap();
        let doc = f.index().read("learnings").await.unwrap();
        assert!(doc.truncated);
        assert_eq!(doc.text.len() as u64, MAX_DOC_BYTES);
    }

    #[test]
    fn missing_home_parts_are_empty() {
        let dir = tempfile::tempdir().unwrap();
        let idx = MemoryIndex::scan(&dir.path().join("data"), &[]);
        assert_eq!(idx.view(), &MemoryView::default());
    }

    #[test]
    fn entries_count_top_level_items_only() {
        assert_eq!(count_entries("- a\n  - nested\n* b\n-c\n"), 2);
        assert_eq!(count_entries("```\n- x\n```\n- y\n"), 1);
    }
}
