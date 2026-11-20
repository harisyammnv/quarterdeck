//! Which worktree a container belongs to.
//!
//! A container belongs to a worktree when its compose working-directory label, or the source
//! of one of its bind mounts, sits inside that worktree's path (design 5.8). Paths are compared
//! by component, so `/p/app` does not claim `/p/app-2`, and the deepest worktree wins when
//! worktrees nest.

use std::path::{Path, PathBuf};

/// Docker Compose labels read for grouping and display.
pub const LABEL_WORKING_DIR: &str = "com.docker.compose.project.working_dir";
pub const LABEL_CONFIG_FILES: &str = "com.docker.compose.project.config_files";
pub const LABEL_SERVICE: &str = "com.docker.compose.service";

/// Index of the worktree in `worktrees` that holds any of `paths`, deepest first.
pub fn owner<'a>(
    worktrees: impl IntoIterator<Item = &'a Path>,
    paths: &[PathBuf],
) -> Option<usize> {
    worktrees
        .into_iter()
        .enumerate()
        .filter(|(_, wt)| wt.is_absolute() && wt.components().count() > 1)
        .filter(|(_, wt)| paths.iter().any(|p| p.starts_with(wt)))
        .max_by_key(|(_, wt)| wt.components().count())
        .map(|(i, _)| i)
}

/// The compose file as a short label: relative to the worktree, prefixed with its folder name,
/// or the bare file name outside any worktree. Never an absolute path.
pub fn compose_label(config_files: &str, worktree: Option<&Path>) -> Option<String> {
    let first = config_files
        .split(',')
        .map(str::trim)
        .find(|s| !s.is_empty())?;
    let file = Path::new(first);
    if let Some(wt) = worktree {
        if let Ok(rel) = file.strip_prefix(wt) {
            let folder = wt.file_name()?.to_string_lossy();
            return Some(format!("{folder}/{}", rel.to_string_lossy()));
        }
    }
    file.file_name().map(|n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn matches_by_component_and_prefers_deepest() {
        let wts = [p("/p/app"), p("/p/app-2"), p("/p/app/nested")];
        let refs = || wts.iter().map(PathBuf::as_path);
        assert_eq!(owner(refs(), &[p("/p/app-2/src")]), Some(1));
        assert_eq!(owner(refs(), &[p("/p/app/src")]), Some(0));
        assert_eq!(owner(refs(), &[p("/p/app/nested/db")]), Some(2));
        assert_eq!(owner(refs(), &[p("/p/application")]), None);
        assert_eq!(owner(refs(), &[]), None);
    }

    #[test]
    fn ignores_root_and_relative_worktrees() {
        let wts = [p("/"), p("relative")];
        assert_eq!(owner(wts.iter().map(PathBuf::as_path), &[p("/p/x")]), None);
    }

    #[test]
    fn compose_label_is_never_absolute() {
        let wt = p("/p/example-app/fm-login");
        assert_eq!(
            compose_label("/p/example-app/fm-login/compose.yaml", Some(&wt)).as_deref(),
            Some("fm-login/compose.yaml")
        );
        assert_eq!(
            compose_label("/p/elsewhere/docker-compose.yml,/p/x.yml", Some(&wt)).as_deref(),
            Some("docker-compose.yml")
        );
        assert_eq!(compose_label("", None), None);
    }
}
