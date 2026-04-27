use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::config::Config;
use crate::discover::{DirectoryDepth, discover_markdown_files_for_targets};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scope {
    Skills,
    Plans,
    ActivePlans,
    CompletedPlans,
}

pub(crate) fn discover_scope_files(config: &Config, scope: Scope) -> Result<Vec<PathBuf>> {
    if matches!(scope, Scope::ActivePlans | Scope::CompletedPlans) {
        ensure_directory_if_exists("plans_dir", &config.plans_root())?;
    }

    let root = scope_root(config, scope);
    if !root.exists() {
        return Ok(Vec::new());
    }
    ensure_directory_if_exists(scope_field_name(scope), &root)?;

    let mut files =
        discover_markdown_files_for_targets(config, &[root], DirectoryDepth::Recursive)?;
    if scope == Scope::Skills {
        files.retain(|path| path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md"));
    }
    Ok(files)
}

fn scope_root(config: &Config, scope: Scope) -> PathBuf {
    match scope {
        Scope::Skills => config.skills_root(),
        Scope::Plans => config.plans_root(),
        Scope::ActivePlans => config.plans_root().join("active"),
        Scope::CompletedPlans => config.plans_root().join("completed"),
    }
}

fn scope_field_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Skills => "skills_dir",
        Scope::Plans | Scope::ActivePlans | Scope::CompletedPlans => "plans_dir",
    }
}

fn ensure_directory_if_exists(field: &str, path: &Path) -> Result<()> {
    if path.exists() && !path.is_dir() {
        bail!(
            "configured {} path {} is not a directory",
            field,
            path.display()
        );
    }
    Ok(())
}
