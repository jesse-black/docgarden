use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::config::Config;
use crate::discover::discover_markdown_files_for_targets;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scope {
    Skills,
    Plans,
    ActivePlans,
    CompletedPlans,
}

pub(crate) fn discover_scope_files(config: &Config, scope: Scope) -> Result<Vec<PathBuf>> {
    let root = scope_root(config, scope);
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        bail!(
            "configured {} path {} is not a directory",
            scope_field_name(scope),
            root.display()
        );
    }

    let mut files = discover_markdown_files_for_targets(config, &[root])?;
    if scope == Scope::Skills {
        files.retain(|path| path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md"));
    }
    Ok(files)
}

pub(crate) fn scope_from_switches(skills: bool, plans: bool) -> Option<Scope> {
    if skills {
        Some(Scope::Skills)
    } else if plans {
        Some(Scope::Plans)
    } else {
        None
    }
}

pub(crate) fn list_scope_from_switches(
    skills: bool,
    plans: bool,
    active_plans: bool,
    completed_plans: bool,
) -> Option<Scope> {
    if skills {
        Some(Scope::Skills)
    } else if plans {
        Some(Scope::Plans)
    } else if active_plans {
        Some(Scope::ActivePlans)
    } else if completed_plans {
        Some(Scope::CompletedPlans)
    } else {
        None
    }
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
