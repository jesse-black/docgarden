use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootMarker<'a> {
    File(&'a str),
    Directory(&'a str),
}

pub fn infer_repository_root(
    targets: &[PathBuf],
    explicit_config: Option<&Path>,
    markers: &[RootMarker<'_>],
) -> Result<PathBuf> {
    if let Some(config_path) = explicit_config {
        let config_path = config_path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", config_path.display()))?;
        return Ok(config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/")));
    }

    let target_dirs: Vec<_> = targets
        .iter()
        .map(|target| {
            if target.is_dir() {
                target.clone()
            } else {
                target.parent().unwrap_or(target.as_path()).to_path_buf()
            }
        })
        .collect();
    let start = match common_ancestor(&target_dirs) {
        Some(path) => path,
        None => std::env::current_dir().context("failed to read current working directory")?,
    };

    if let Some(root) = find_root_by_markers(&start, markers) {
        return Ok(root);
    }

    std::env::current_dir().context("failed to read current working directory")
}

pub fn find_root_by_markers(start: &Path, markers: &[RootMarker<'_>]) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|ancestor| matches_markers(ancestor, markers))
        .map(Path::to_path_buf)
}

fn matches_markers(path: &Path, markers: &[RootMarker<'_>]) -> bool {
    markers.iter().any(|marker| match marker {
        RootMarker::File(name) => path.join(name).is_file(),
        RootMarker::Directory(name) => path.join(name).is_dir(),
    })
}

fn common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut components: Vec<OsString> = paths
        .first()?
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();

    for path in &paths[1..] {
        let other: Vec<OsString> = path
            .components()
            .map(|component| component.as_os_str().to_os_string())
            .collect();
        let shared = components
            .iter()
            .zip(other.iter())
            .take_while(|(left, right)| left == right)
            .count();
        components.truncate(shared);
    }

    if components.is_empty() {
        return None;
    }

    let mut ancestor = PathBuf::new();
    for component in components {
        ancestor.push(component);
    }
    Some(ancestor)
}

#[cfg(test)]
mod tests {
    use super::{RootMarker, common_ancestor, find_root_by_markers, infer_repository_root};
    use std::fs;
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use tempfile::TempDir;

    fn markers<'a>() -> &'a [RootMarker<'a>] {
        &[
            RootMarker::File("docgarden.toml"),
            RootMarker::File("pyproject.toml"),
            RootMarker::Directory(".git"),
        ]
    }

    fn touch(path: &Path) -> Result<()> {
        fs::write(path, "")?;
        Ok(())
    }

    #[test]
    fn finds_file_marker_from_nested_target() -> Result<()> {
        let temp = TempDir::new()?;
        let repo = temp.path().join("repo");
        let nested = repo.join("docs/guides");
        fs::create_dir_all(&nested)?;
        touch(&repo.join("pyproject.toml"))?;

        let root = infer_repository_root(&[nested], None, markers())?;

        assert_eq!(root, repo);
        Ok(())
    }

    #[test]
    fn falls_back_to_directory_marker_when_file_marker_is_missing() -> Result<()> {
        let temp = TempDir::new()?;
        let repo = temp.path().join("repo");
        let nested = repo.join("docs/guides");
        fs::create_dir_all(&nested)?;
        fs::create_dir(repo.join(".git"))?;

        let root = infer_repository_root(&[nested], None, markers())?;

        assert_eq!(root, repo);
        Ok(())
    }

    #[test]
    fn explicit_config_parent_wins() -> Result<()> {
        let temp = TempDir::new()?;
        let repo = temp.path().join("repo");
        let config_dir = repo.join("config");
        let nested = repo.join("docs/guides");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&nested)?;
        let config_path = config_dir.join("custom.toml");
        touch(&config_path)?;
        touch(&repo.join("pyproject.toml"))?;

        let root = infer_repository_root(&[nested], Some(&config_path), markers())?;

        assert_eq!(root, config_dir);
        Ok(())
    }

    #[test]
    fn empty_targets_fall_back_to_current_directory() -> Result<()> {
        let current_dir = std::env::current_dir()?;

        let root = infer_repository_root(&[], None, &[])?;

        assert_eq!(root, current_dir);
        Ok(())
    }

    #[test]
    fn finds_root_from_starting_path_markers() -> Result<()> {
        let temp = TempDir::new()?;
        let repo = temp.path().join("repo");
        let nested = repo.join("src/bin");
        fs::create_dir_all(&nested)?;
        touch(&repo.join("docgarden.toml"))?;

        let root = find_root_by_markers(&nested, markers());

        assert_eq!(root, Some(repo));
        Ok(())
    }

    #[test]
    fn common_ancestor_returns_shared_prefix() {
        let paths = vec![
            PathBuf::from("/tmp/repo/docs/a.md"),
            PathBuf::from("/tmp/repo/src/lib.rs"),
        ];

        assert_eq!(common_ancestor(&paths), Some(PathBuf::from("/tmp/repo")));
    }
}
