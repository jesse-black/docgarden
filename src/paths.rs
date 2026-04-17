use std::path::Path;

use anyhow::{Context, Result};

pub fn repository_relative_path(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is not under {}", path.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::repository_relative_path;

    #[test]
    fn returns_relative_path_when_under_root() {
        let root = Path::new("/repo");
        let path = Path::new("/repo/docs/guide.md");
        assert_eq!(
            repository_relative_path(root, path).unwrap(),
            "docs/guide.md"
        );
    }

    #[test]
    fn errors_when_path_is_not_under_root() {
        let root = Path::new("/repo");
        let path = Path::new("/other/file.md");
        let err = repository_relative_path(root, path).unwrap_err();
        assert!(err.to_string().contains("is not under"));
    }
}
