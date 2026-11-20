//! The skills browser: every `SKILL.md` folder in the known skill directories, read-only.
//!
//! Roots are fixed: Firstmate's own `.agents/skills`, the user-level `~/.agents/skills` and
//! `~/.claude/skills`, and `.agents/skills` and `.claude/skills` in each project the fleet
//! works on. Only `<root>/<folder>/SKILL.md` is read, so the walk is one level deep. The UI
//! gets opaque ids and short labels, never paths.

mod frontmatter;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::memory::fsread;
use crate::memory::ProjectRoot;
pub use frontmatter::FrontMatter;

/// Skill files larger than this are cut, with a note.
pub const MAX_SKILL_BYTES: u64 = 1024 * 1024;
/// Bytes read from each skill to find its front matter while listing.
const PROBE_BYTES: u64 = 32 * 1024;
/// Folders read per root, and skills listed in total.
pub const MAX_PER_ROOT: usize = 500;
pub const MAX_SKILLS: usize = 2000;
const LOCK_MAX_BYTES: u64 = 1024 * 1024;

/// Where a skill is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum SkillScope {
    /// Firstmate's own `.agents/skills`.
    Firstmate,
    /// The user's `~/.agents/skills` or `~/.claude/skills`.
    User,
    /// A project's `.agents/skills` or `.claude/skills`.
    Project,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SkillItem {
    /// Opaque id for `skill_read`.
    pub id: String,
    pub name: String,
    /// The trigger description, on one line; empty when the skill has none.
    pub description: String,
    pub scope: SkillScope,
    /// Where it came from: a source repository when one is recorded, the project for a
    /// project skill, `firstmate` for Firstmate's own.
    pub source: Option<String>,
    /// Short labels of the folders it was found in, such as `~/.agents/skills`.
    pub locations: Vec<String>,
    /// How many other installed skills have the same name.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub also_in: u32,
    /// Marked internal in its metadata (not meant to be invoked by hand).
    pub internal: bool,
    /// The front matter could not be read as YAML.
    pub malformed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SkillsView {
    /// Firstmate skills first, then user, then project; by name within each.
    pub items: Vec<SkillItem>,
    /// More skills were found than [MAX_SKILLS] or a root held more than [MAX_PER_ROOT].
    pub capped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SkillDoc {
    pub item: SkillItem,
    /// The Markdown body after the front matter; untrusted, rendered only through the shared
    /// Markdown component.
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("no skill has the id {0:?}")]
    UnknownId(String),
    #[error("the skill {0} could not be read: {1}")]
    Unreadable(String, fsread::ReadError),
}

/// Where to look.
#[derive(Clone, Debug, Default)]
pub struct SkillSources {
    /// The Firstmate home root.
    pub firstmate: Option<PathBuf>,
    /// The user's home directory.
    pub user_home: Option<PathBuf>,
    pub projects: Vec<ProjectRoot>,
}

struct Root {
    scope: SkillScope,
    /// Stable part of the id, such as `user:agents`.
    key: String,
    label: String,
    dir: PathBuf,
    project: Option<String>,
}

impl SkillSources {
    fn roots(&self) -> Vec<Root> {
        let mut roots = Vec::new();
        if let Some(home) = &self.firstmate {
            roots.push(Root {
                scope: SkillScope::Firstmate,
                key: "firstmate".into(),
                label: "Firstmate .agents/skills".into(),
                dir: home.join(".agents/skills"),
                project: None,
            });
        }
        if let Some(home) = &self.user_home {
            for (key, rel) in [("agents", ".agents/skills"), ("claude", ".claude/skills")] {
                roots.push(Root {
                    scope: SkillScope::User,
                    key: format!("user:{key}"),
                    label: format!("~/{rel}"),
                    dir: home.join(rel),
                    project: None,
                });
            }
        }
        let mut projects: Vec<&ProjectRoot> = self.projects.iter().collect();
        projects.sort_by(|a, b| a.name.cmp(&b.name));
        projects.dedup_by(|a, b| a.name == b.name);
        for p in projects {
            for (key, rel) in [("agents", ".agents/skills"), ("claude", ".claude/skills")] {
                roots.push(Root {
                    scope: SkillScope::Project,
                    key: format!("project:{}:{key}", p.name),
                    label: format!("{}/{rel}", p.name),
                    dir: p.path.join(rel),
                    project: Some(p.name.clone()),
                });
            }
        }
        roots
    }
}

/// A scanned list plus the private id-to-path map behind it.
#[derive(Clone, Debug, Default)]
pub struct SkillIndex {
    view: SkillsView,
    paths: HashMap<String, Located>,
}

#[derive(Clone, Debug)]
struct Located {
    /// The skill folder; `SKILL.md` is read inside it and must stay inside it.
    dir: PathBuf,
}

impl SkillIndex {
    /// Lists every skill under `sources`.
    pub async fn scan(sources: &SkillSources) -> Self {
        let lock = match &sources.user_home {
            Some(home) => read_lock(home).await,
            None => HashMap::new(),
        };
        let mut index = Self::default();
        let mut by_real: HashMap<PathBuf, usize> = HashMap::new();
        for root in sources.roots() {
            let dirs = fsread::child_dirs(&root.dir, MAX_PER_ROOT + 1);
            if dirs.len() > MAX_PER_ROOT {
                index.view.capped = true;
            }
            for folder in dirs.into_iter().take(MAX_PER_ROOT) {
                let dir = root.dir.join(&folder);
                let file = dir.join("SKILL.md");
                if fsread::file_meta(&file).is_none() {
                    continue;
                }
                // The same folder reached twice (a user skill linked from ~/.claude/skills to
                // ~/.agents/skills) is one skill installed in two places.
                let real = fsread::real_path(&dir).unwrap_or_else(|| dir.clone());
                if let Some(&at) = by_real.get(&real) {
                    let item = &mut index.view.items[at];
                    if !item.locations.contains(&root.label) {
                        item.locations.push(root.label.clone());
                    }
                    continue;
                }
                if index.view.items.len() >= MAX_SKILLS {
                    index.view.capped = true;
                    break;
                }
                let head = fsread::read_doc(&dir, &file, PROBE_BYTES).await.ok();
                let fm = head
                    .as_ref()
                    .map(|(text, _)| frontmatter::split(text).0)
                    .unwrap_or_default();
                let name = fm
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| folder.clone());
                let source = match root.scope {
                    SkillScope::Firstmate => Some("firstmate".to_string()),
                    SkillScope::User => lock.get(&folder).or_else(|| lock.get(&name)).cloned(),
                    SkillScope::Project => root.project.clone(),
                };
                let id = format!("{}:{folder}", root.key);
                by_real.insert(real, index.view.items.len());
                index.paths.insert(id.clone(), Located { dir });
                index.view.items.push(SkillItem {
                    id,
                    name,
                    description: one_line(fm.description.as_deref().unwrap_or("")),
                    scope: root.scope,
                    source,
                    locations: vec![root.label.clone()],
                    also_in: 0,
                    internal: fm.internal,
                    malformed: head.is_none() || fm.malformed,
                });
            }
        }
        let mut counts: HashMap<String, u32> = HashMap::new();
        for item in &index.view.items {
            *counts.entry(item.name.to_lowercase()).or_default() += 1;
        }
        for item in &mut index.view.items {
            let n = counts.get(&item.name.to_lowercase()).copied().unwrap_or(1);
            // A folder linked into a second root also counts as another place.
            let extra = u32::try_from(item.locations.len())
                .unwrap_or(1)
                .saturating_sub(1);
            item.also_in = n.saturating_sub(1) + extra;
        }
        index.view.items.sort_by(|a, b| {
            a.scope
                .rank()
                .cmp(&b.scope.rank())
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });
        index
    }

    pub fn view(&self) -> &SkillsView {
        &self.view
    }

    /// Reads skill `id` for display.
    pub async fn read(&self, id: &str) -> Result<SkillDoc, SkillError> {
        let unknown = || SkillError::UnknownId(id.to_string());
        let loc = self.paths.get(id).ok_or_else(unknown)?;
        let item = self
            .view
            .items
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or_else(unknown)?;
        let (text, truncated) =
            fsread::read_doc(&loc.dir, &loc.dir.join("SKILL.md"), MAX_SKILL_BYTES)
                .await
                .map_err(|e| SkillError::Unreadable(item.name.clone(), e))?;
        let body = frontmatter::split(&text).1.to_string();
        Ok(SkillDoc {
            item,
            text: body,
            truncated,
        })
    }
}

impl SkillScope {
    fn rank(self) -> u8 {
        match self {
            Self::Firstmate => 0,
            Self::User => 1,
            Self::Project => 2,
        }
    }
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Source repositories recorded by the skills CLI in `~/.agents/.skill-lock.json`, by skill
/// folder name. The file belongs to another tool, so anything unexpected yields nothing.
async fn read_lock(user_home: &Path) -> HashMap<String, String> {
    let root = user_home.join(".agents");
    let Ok(text) =
        fsread::read_text_under(&root, Path::new(".skill-lock.json"), LOCK_MAX_BYTES).await
    else {
        return HashMap::new();
    };
    parse_lock(&text)
}

fn parse_lock(text: &str) -> HashMap<String, String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return HashMap::new();
    };
    let Some(skills) = v.get("skills").and_then(|s| s.as_object()) else {
        return HashMap::new();
    };
    skills
        .iter()
        .filter_map(|(name, rec)| {
            let src = rec.get("source")?.as_str()?;
            let ok = !src.is_empty()
                && src.len() <= 200
                && src
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '@'));
            ok.then(|| (name.clone(), src.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn sources(&self) -> SkillSources {
            let r = self.dir.path();
            SkillSources {
                firstmate: Some(r.join("fm")),
                user_home: Some(r.join("user")),
                projects: vec![ProjectRoot {
                    name: "ledger-api".into(),
                    path: r.join("p/ledger-api"),
                }],
            }
        }
    }

    fn skill(name: &str, desc: &str) -> String {
        format!("---\nname: {name}\ndescription: {desc}\n---\n\n# {name}\n\nBody of {name}.\n")
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        let w = |p: &str, s: &str| {
            let p = r.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, s).unwrap();
        };
        w(
            "fm/.agents/skills/ship-landing/SKILL.md",
            "---\nname: ship-landing\ndescription: >-\n  When a ship reports a pull request\n  or a ready branch.\nmetadata:\n  internal: true\n---\nLand it.\n",
        );
        w(
            "user/.agents/skills/tdd/SKILL.md",
            &skill("tdd", "Build test-first."),
        );
        w(
            "user/.agents/skills/code-review/SKILL.md",
            &skill("code-review", "Review a change."),
        );
        w(
            "user/.claude/skills/code-review/SKILL.md",
            &skill("code-review", "Another copy."),
        );
        w("user/.claude/skills/no-front/SKILL.md", "# Just a body\n");
        w(
            "user/.claude/skills/broken/SKILL.md",
            "---\nname: [unclosed\n---\nbody\n",
        );
        w("user/.claude/skills/empty-folder/README.md", "not a skill");
        w(
            "user/.claude/skills/.trash/SKILL.md",
            &skill("trash", "hidden"),
        );
        w(
            "p/ledger-api/.claude/skills/tdd/SKILL.md",
            &skill("tdd", "Project flavour."),
        );
        w(
            "user/.agents/.skill-lock.json",
            r#"{"version":3,"skills":{"tdd":{"source":"example-org/skills"},"code-review":{"source":"bad source; rm -rf"}}}"#,
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            r.join("user/.agents/skills/tdd"),
            r.join("user/.claude/skills/tdd"),
        )
        .unwrap();
        Fixture { dir }
    }

    #[tokio::test]
    async fn lists_every_root_with_scope_and_source() {
        let f = fixture();
        let idx = SkillIndex::scan(&f.sources()).await;
        let rows: Vec<_> = idx
            .view()
            .items
            .iter()
            .map(|i| (i.id.as_str(), i.name.as_str(), i.scope, i.source.as_deref()))
            .collect();
        assert_eq!(
            rows,
            [
                (
                    "firstmate:ship-landing",
                    "ship-landing",
                    SkillScope::Firstmate,
                    Some("firstmate")
                ),
                ("user:claude:broken", "broken", SkillScope::User, None),
                (
                    "user:agents:code-review",
                    "code-review",
                    SkillScope::User,
                    None
                ),
                (
                    "user:claude:code-review",
                    "code-review",
                    SkillScope::User,
                    None
                ),
                ("user:claude:no-front", "no-front", SkillScope::User, None),
                (
                    "user:agents:tdd",
                    "tdd",
                    SkillScope::User,
                    Some("example-org/skills")
                ),
                (
                    "project:ledger-api:claude:tdd",
                    "tdd",
                    SkillScope::Project,
                    Some("ledger-api")
                ),
            ]
        );
        assert!(!idx.view().capped);
    }

    #[tokio::test]
    async fn folded_descriptions_and_metadata() {
        let f = fixture();
        let idx = SkillIndex::scan(&f.sources()).await;
        let ship = &idx.view().items[0];
        assert_eq!(
            ship.description,
            "When a ship reports a pull request or a ready branch."
        );
        assert!(ship.internal);
        assert!(!ship.malformed);
        let broken = idx
            .view()
            .items
            .iter()
            .find(|i| i.name == "broken")
            .unwrap();
        assert!(broken.malformed);
        let bare = idx
            .view()
            .items
            .iter()
            .find(|i| i.name == "no-front")
            .unwrap();
        assert_eq!(bare.description, "");
        assert!(!bare.malformed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn linked_folders_are_one_skill_in_two_places() {
        let f = fixture();
        let idx = SkillIndex::scan(&f.sources()).await;
        let tdd = idx
            .view()
            .items
            .iter()
            .find(|i| i.id == "user:agents:tdd")
            .unwrap();
        assert_eq!(tdd.locations, ["~/.agents/skills", "~/.claude/skills"]);
        // Linked from a second root, and the project has its own copy.
        assert_eq!(tdd.also_in, 2);
        let review = idx
            .view()
            .items
            .iter()
            .find(|i| i.id == "user:agents:code-review")
            .unwrap();
        assert_eq!(review.also_in, 1);
        assert!(!idx.view().items.iter().any(|i| i.id == "user:claude:tdd"));
    }

    #[tokio::test]
    async fn reads_the_body_without_front_matter() {
        let f = fixture();
        let idx = SkillIndex::scan(&f.sources()).await;
        let doc = idx.read("user:agents:tdd").await.unwrap();
        assert_eq!(doc.text.trim_start(), "# tdd\n\nBody of tdd.\n");
        assert!(!doc.truncated);
        for id in ["../etc", "user:agents:../../x", "tdd", ""] {
            assert!(
                matches!(idx.read(id).await, Err(SkillError::UnknownId(_))),
                "{id}"
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_skill_file_linking_out_of_its_folder_is_refused() {
        let f = fixture();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("x.md"), "---\nname: x\n---\nsecret").unwrap();
        let file = f.dir.path().join("user/.claude/skills/no-front/SKILL.md");
        std::fs::remove_file(&file).unwrap();
        std::os::unix::fs::symlink(outside.path().join("x.md"), &file).unwrap();
        let idx = SkillIndex::scan(&f.sources()).await;
        let item = idx
            .view()
            .items
            .iter()
            .find(|i| i.id == "user:claude:no-front")
            .unwrap();
        // Listed by folder name with nothing read from the escaping file.
        assert_eq!(item.name, "no-front");
        assert!(item.malformed);
        assert!(matches!(
            idx.read("user:claude:no-front").await,
            Err(SkillError::Unreadable(_, fsread::ReadError::Escapes))
        ));
    }

    #[tokio::test]
    async fn missing_roots_and_caps() {
        let dir = tempfile::tempdir().unwrap();
        let empty = SkillIndex::scan(&SkillSources {
            firstmate: Some(dir.path().join("nope")),
            user_home: None,
            projects: vec![],
        })
        .await;
        assert_eq!(empty.view(), &SkillsView::default());

        let root = dir.path().join("fm/.agents/skills");
        for i in 0..=MAX_PER_ROOT {
            let d = root.join(format!("s{i:04}"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("SKILL.md"), "body").unwrap();
        }
        let start = std::time::Instant::now();
        let many = SkillIndex::scan(&SkillSources {
            firstmate: Some(dir.path().join("fm")),
            ..SkillSources::default()
        })
        .await;
        assert_eq!(many.view().items.len(), MAX_PER_ROOT);
        assert!(many.view().capped);
        // A loose bound that catches quadratic work without flaking in debug builds.
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn lock_file_is_tolerated() {
        assert!(parse_lock("not json").is_empty());
        assert!(parse_lock(r#"{"skills":[1,2]}"#).is_empty());
        let m = parse_lock(r#"{"skills":{"a":{"source":"org/repo"},"b":{"source":42},"c":{}}}"#);
        assert_eq!(m.len(), 1);
        assert_eq!(m["a"], "org/repo");
    }
}
