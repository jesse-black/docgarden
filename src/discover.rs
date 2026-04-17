use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::WalkBuilder;

use crate::config::Config;
use crate::diagnostics::PatternMatcher;
use crate::paths::repository_relative_path;

pub fn discover_markdown_files_for_targets(
    config: &Config,
    targets: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let include = PatternMatcher::new(&config.include)?;
    let exclude = PatternMatcher::new(&config.exclude)?;
    let mut files = BTreeSet::new();

    for target in targets {
        let metadata = target
            .metadata()
            .with_context(|| format!("failed to read {}", target.display()))?;
        if metadata.is_dir() {
            for path in discover_markdown_files_under(config, target, &include, &exclude)? {
                files.insert(path);
            }
        } else {
            if !is_markdown_path(target) {
                bail!(
                    "{} is not a Markdown file; only .md files are supported as explicit targets",
                    target.display()
                );
            }
            files.insert(target.clone());
        }
    }

    Ok(files.into_iter().collect())
}

fn discover_markdown_files_under(
    config: &Config,
    root: &Path,
    include: &PatternMatcher,
    exclude: &PatternMatcher,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(config.respect_gitignore)
        .git_exclude(config.respect_gitignore)
        .git_global(config.respect_gitignore)
        .ignore(config.respect_gitignore)
        .require_git(false);
    walker.follow_links(false);

    for entry in walker.build() {
        let entry = entry?;
        if entry
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let path = entry.into_path();
        if !is_markdown_path(&path) {
            continue;
        }
        let relative = repository_relative_path(&config.repository_root, &path)?;
        if !include.is_match(&relative, false) {
            continue;
        }
        if exclude.is_match(&relative, false) {
            continue;
        }
        files.push(path);
    }

    files.sort();
    Ok(files)
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("md")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::config::Config;

    #[test]
    fn discovered_set_includes_md_and_excludes_non_md() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docgarden.toml"), "").unwrap();
        fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
        fs::write(root.join("docs/notes.txt"), "Plain text.\n").unwrap();

        let config = Config::load(root, None).unwrap();
        let targets = vec![root.join("docs").canonicalize().unwrap()];
        let discovered = discover_markdown_files_for_targets(&config, &targets).unwrap();

        let names: Vec<_> = discovered
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(names.contains(&"guide.md"), "expected guide.md in results");
        assert!(!names.contains(&"notes.txt"), "expected notes.txt excluded");
    }
}
