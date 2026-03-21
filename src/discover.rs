use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use crate::config::Config;
use crate::diagnostics::PatternMatcher;

pub fn discover_markdown_files_for_targets(
    config: &Config,
    targets: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();

    for target in targets {
        let metadata = target
            .metadata()
            .with_context(|| format!("failed to read {}", target.display()))?;
        if metadata.is_dir() {
            for path in discover_markdown_files_under(config, target)? {
                files.insert(path);
            }
        } else {
            files.insert(target.clone());
        }
    }

    Ok(files.into_iter().collect())
}

fn discover_markdown_files_under(config: &Config, root: &Path) -> Result<Vec<PathBuf>> {
    let include = PatternMatcher::new(&config.include)?;
    let exclude = PatternMatcher::new(&config.exclude)?;
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
        let relative = relative_path(&config.repository_root, &path)?;
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

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is not under {}", path.display(), root.display()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}
