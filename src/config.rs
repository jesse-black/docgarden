use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Deserialize;

use crate::defaults::{DEFAULT_SCAN_PATTERNS, default_extensions, default_special_filenames};
use crate::diagnostics::Severity;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontmatterFieldConfig {
    pub max_chars: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontmatterRuleConfig {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, FrontmatterFieldConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    #[serde(default = "default_skills_dir")]
    pub skills_dir: String,
    #[serde(default = "default_plans_dir")]
    pub plans_dir: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub extend_extensions: Vec<String>,
    #[serde(default)]
    pub remove_extensions: Vec<String>,
    #[serde(default)]
    pub extend_special_filenames: Vec<String>,
    #[serde(default)]
    pub remove_special_filenames: Vec<String>,
    #[serde(default = "default_respect_gitignore")]
    pub respect_gitignore: bool,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    pub path: String,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub disable: Option<Vec<String>>,
    #[serde(default)]
    pub enable: Option<Vec<String>>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub max_lines: Option<usize>,
    #[serde(default)]
    pub severity: Option<RuleSeverity>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub frontmatter: Option<FrontmatterRuleConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleSeverity {
    Error,
    Warn,
}

impl From<RuleSeverity> for Severity {
    fn from(value: RuleSeverity) -> Self {
        match value {
            RuleSeverity::Error => Severity::Error,
            RuleSeverity::Warn => Severity::Warning,
        }
    }
}

#[derive(Clone)]
struct CompiledRuleTarget {
    path_matcher: Gitignore,
    exclude_matcher: Gitignore,
}

impl CompiledRuleTarget {
    fn new(root: &Path, pattern: &str, exclude: &[String]) -> Result<Self> {
        Ok(Self {
            path_matcher: compile_rule_matcher(root, &[pattern.to_string()])?,
            exclude_matcher: compile_rule_matcher(root, exclude)?,
        })
    }

    fn matches(&self, relative_path: &Path) -> bool {
        matcher_matches(&self.path_matcher, relative_path)
            && !matcher_matches(&self.exclude_matcher, relative_path)
    }
}

#[derive(Clone)]
pub struct FrontmatterRule {
    target: CompiledRuleTarget,
    pub required: Vec<String>,
    pub field_max_chars: BTreeMap<String, usize>,
}

impl FrontmatterRule {
    fn new(
        root: &Path,
        pattern: String,
        exclude: Vec<String>,
        required: Vec<String>,
        field_max_chars: BTreeMap<String, usize>,
    ) -> Result<Self> {
        Ok(Self {
            target: CompiledRuleTarget::new(root, &pattern, &exclude)?,
            required,
            field_max_chars,
        })
    }

    fn matches(&self, relative_path: &Path) -> bool {
        self.target.matches(relative_path)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EffectiveFrontmatterPolicy {
    pub required: Vec<String>,
    pub field_max_chars: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetLimit {
    pub limit: usize,
    pub severity: RuleSeverity,
}

/// A lowered `[[rules]]` entry carrying all non-frontmatter rule behavior.
///
/// Stored in source order in `Config.rule_applications` so that
/// `effective_rule_policy_for_path` can apply last-writer-wins semantics.
#[derive(Clone)]
pub struct RuleApplication {
    target: CompiledRuleTarget,
    pub disable: Vec<String>,
    pub enable: Vec<String>,
    /// Severity used when `enable` contains `"unresolved-backtick-path"`.
    pub severity: RuleSeverity,
    pub max_tokens: Option<BudgetLimit>,
    pub max_lines: Option<BudgetLimit>,
}

impl RuleApplication {
    fn matches(&self, relative_path: &Path) -> bool {
        self.target.matches(relative_path)
    }
}

/// The effective per-file rule state derived by the ordered reducer.
///
/// All non-frontmatter rule behavior flows through this struct.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EffectiveRulePolicy {
    pub ignored_rules: BTreeSet<String>,
    pub backtick_path_severity: Option<Severity>,
    pub prefer_links_for_local_paths: bool,
    pub max_tokens: Option<BudgetLimit>,
    pub max_lines: Option<BudgetLimit>,
}

#[derive(Clone)]
pub struct Config {
    pub repository_root: PathBuf,
    pub skills_dir: PathBuf,
    pub plans_dir: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub rule_applications: Vec<RuleApplication>,
    pub known_extensions: BTreeSet<String>,
    pub special_filenames: BTreeSet<String>,
    pub config_path: Option<PathBuf>,
    pub config_was_explicit: bool,
    pub frontmatter_rules: Vec<FrontmatterRule>,
    pub respect_gitignore: bool,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("repository_root", &self.repository_root)
            .field("skills_dir", &self.skills_dir)
            .field("plans_dir", &self.plans_dir)
            .field("include", &self.include)
            .field("exclude", &self.exclude)
            .field("rule_application_count", &self.rule_applications.len())
            .field("known_extensions", &self.known_extensions)
            .field("special_filenames", &self.special_filenames)
            .field("config_path", &self.config_path)
            .field("config_was_explicit", &self.config_was_explicit)
            .field("frontmatter_rule_count", &self.frontmatter_rules.len())
            .field("respect_gitignore", &self.respect_gitignore)
            .finish()
    }
}

impl Config {
    pub fn load(repository_root: &Path, explicit_config: Option<&Path>) -> Result<Self> {
        let repository_root = repository_root
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", repository_root.display()))?;
        let config_was_explicit = explicit_config.is_some();
        let config_path = if let Some(path) = explicit_config {
            Some(path.to_path_buf())
        } else {
            let candidate = repository_root.join("docgarden.toml");
            if candidate.exists() {
                Some(candidate)
            } else {
                None
            }
        };

        let parsed = if let Some(path) = &config_path {
            let content = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            toml::from_str::<FileConfig>(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?
        } else {
            toml::from_str::<FileConfig>(include_str!("data/default-config.toml"))
                .context("failed to parse embedded default config")?
        };

        let skills_dir = validate_repository_relative_dir("skills_dir", &parsed.skills_dir)?;
        let plans_dir = validate_repository_relative_dir("plans_dir", &parsed.plans_dir)?;

        let mut known_extensions = default_extensions();
        for extension in parsed.extend_extensions {
            known_extensions.insert(normalize_extension(&extension));
        }
        for extension in parsed.remove_extensions {
            known_extensions.remove(&normalize_extension(&extension));
        }

        let mut special_filenames = default_special_filenames();
        for filename in parsed.extend_special_filenames {
            special_filenames.insert(filename);
        }
        for filename in parsed.remove_special_filenames {
            special_filenames.remove(&filename);
        }

        let include = if parsed.include.is_empty() {
            DEFAULT_SCAN_PATTERNS
                .iter()
                .map(|value| value.to_string())
                .collect()
        } else {
            parsed.include
        };

        if include.is_empty() {
            bail!("include patterns must not be empty");
        }

        let (rule_applications, frontmatter_rules) = lower_rules(&repository_root, parsed.rules)?;

        Ok(Self {
            repository_root,
            skills_dir,
            plans_dir,
            include,
            exclude: parsed.exclude,
            rule_applications,
            known_extensions,
            special_filenames,
            config_path,
            config_was_explicit,
            frontmatter_rules,
            respect_gitignore: parsed.respect_gitignore,
        })
    }

    /// Derive effective per-file rule state for `relative_path` by applying
    /// all matching `[[rules]]` entries in source order (last-writer-wins).
    ///
    /// - `disable` for opt-in rules (`unresolved-backtick-path`,
    ///   `prefer-links-for-local-paths`, `max_tokens`, `max_lines`) clears
    ///   their state directly so a later `enable` can restore them.
    /// - `disable` for always-on rules (e.g. `unresolved-link-path`) adds to
    ///   `ignored_rules`; always-on rules cannot be re-enabled via `enable`.
    pub fn effective_rule_policy_for_path(
        &self,
        relative_path: &str,
    ) -> Result<EffectiveRulePolicy> {
        let relative_path = Path::new(relative_path);
        let mut ignored_rules: BTreeSet<String> = BTreeSet::new();
        let mut backtick_severity: Option<Severity> = None;
        let mut prefer_links = false;
        let mut max_tokens: Option<BudgetLimit> = None;
        let mut max_lines: Option<BudgetLimit> = None;

        for app in &self.rule_applications {
            if !app.matches(relative_path) {
                continue;
            }

            for rule in &app.disable {
                match rule.as_str() {
                    "unresolved-backtick-path" => backtick_severity = None,
                    "prefer-links-for-local-paths" => prefer_links = false,
                    "max_tokens" => max_tokens = None,
                    "max_lines" => max_lines = None,
                    r => {
                        ignored_rules.insert(r.to_string());
                    }
                }
            }

            for rule in &app.enable {
                match rule.as_str() {
                    "unresolved-backtick-path" => {
                        backtick_severity = Some(app.severity.into());
                    }
                    "prefer-links-for-local-paths" => prefer_links = true,
                    _ => {}
                }
            }

            if let Some(limit) = app.max_tokens {
                max_tokens = Some(limit);
            }
            if let Some(limit) = app.max_lines {
                max_lines = Some(limit);
            }
        }

        Ok(EffectiveRulePolicy {
            ignored_rules,
            backtick_path_severity: backtick_severity,
            prefer_links_for_local_paths: prefer_links,
            max_tokens,
            max_lines,
        })
    }

    pub fn frontmatter_policy_for_path(
        &self,
        relative_path: &str,
    ) -> Result<EffectiveFrontmatterPolicy> {
        let relative_path = Path::new(relative_path);
        let mut policy = EffectiveFrontmatterPolicy::default();
        for rule in &self.frontmatter_rules {
            if !rule.matches(relative_path) {
                continue;
            }
            if !rule.required.is_empty() {
                policy.required = rule.required.clone();
            }
            for (field, max_chars) in &rule.field_max_chars {
                policy.field_max_chars.insert(field.clone(), *max_chars);
            }
        }
        Ok(policy)
    }

    pub fn skills_root(&self) -> PathBuf {
        self.repository_root.join(&self.skills_dir)
    }

    pub fn plans_root(&self) -> PathBuf {
        self.repository_root.join(&self.plans_dir)
    }
}

fn lower_rules(
    root: &Path,
    rules: Vec<RuleConfig>,
) -> Result<(Vec<RuleApplication>, Vec<FrontmatterRule>)> {
    let mut applications: Vec<RuleApplication> = Vec::new();
    let mut frontmatter_rules: Vec<FrontmatterRule> = Vec::new();

    for rule in rules {
        if rule.path.trim().is_empty() {
            bail!("rule path must not be empty");
        }
        if let Some(reason) = &rule.reason
            && reason.trim().is_empty()
        {
            bail!("rule reason must not be empty");
        }
        let pattern = rule.path;
        let exclude = rule.exclude;
        let disabled_rules = rule.disable.unwrap_or_default();
        let enabled_rules = rule.enable.unwrap_or_default();
        let severity = rule.severity.unwrap_or(RuleSeverity::Error);

        if !disabled_rules.is_empty() {
            validate_rule_list("disable", &disabled_rules, is_known_rule)?;
        }
        if !enabled_rules.is_empty() {
            validate_rule_list("enable", &enabled_rules, is_supported_enabled_rule)?;
        }

        let max_tokens = rule
            .max_tokens
            .map(|limit| budget_limit("max_tokens", limit, severity))
            .transpose()?;
        let max_lines = rule
            .max_lines
            .map(|limit| budget_limit("max_lines", limit, severity))
            .transpose()?;

        let has_rule_content = !disabled_rules.is_empty()
            || !enabled_rules.is_empty()
            || max_tokens.is_some()
            || max_lines.is_some();

        if has_rule_content {
            applications.push(RuleApplication {
                target: CompiledRuleTarget::new(root, &pattern, &exclude)?,
                disable: disabled_rules,
                enable: enabled_rules,
                severity,
                max_tokens,
                max_lines,
            });
        }

        if let Some(fm) = rule.frontmatter {
            let mut field_max_chars = BTreeMap::new();
            for (field_name, field_cfg) in fm.fields {
                if field_name.trim().is_empty() {
                    bail!("frontmatter field name must not be empty");
                }
                if let Some(max_chars) = field_cfg.max_chars {
                    if max_chars == 0 {
                        bail!(
                            "frontmatter field `{field_name}` max_chars must be greater than zero"
                        );
                    }
                    field_max_chars.insert(field_name, max_chars);
                }
            }
            if !fm.required.is_empty() || !field_max_chars.is_empty() {
                frontmatter_rules.push(FrontmatterRule::new(
                    root,
                    pattern,
                    exclude,
                    fm.required,
                    field_max_chars,
                )?);
            }
        }
    }
    Ok((applications, frontmatter_rules))
}

fn budget_limit(rule: &str, limit: usize, severity: RuleSeverity) -> Result<BudgetLimit> {
    if limit == 0 {
        bail!("{rule} must be greater than zero");
    }
    Ok(BudgetLimit { limit, severity })
}

fn validate_rule_list(
    field: &str,
    rules: &[String],
    is_supported: impl Fn(&str) -> bool,
) -> Result<()> {
    if rules.is_empty() {
        bail!("rules `{field}` entries must not be empty");
    }
    for rule in rules {
        if rule.trim().is_empty() {
            bail!("rules `{field}` entries must not contain empty rule names");
        }
        if !is_supported(rule) {
            bail!("unsupported rule `{rule}` in `{field}`");
        }
    }
    Ok(())
}

fn is_known_rule(rule: &str) -> bool {
    matches!(
        rule,
        "unresolved-link-path"
            | "unresolved-backtick-path"
            | "prefer-links-for-local-paths"
            | "max_tokens"
            | "max_lines"
    )
}

fn is_supported_enabled_rule(rule: &str) -> bool {
    matches!(
        rule,
        "unresolved-backtick-path" | "prefer-links-for-local-paths"
    )
}

fn compile_rule_matcher(root: &Path, patterns: &[String]) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .with_context(|| format!("invalid rule path pattern {pattern}"))?;
    }
    Ok(builder.build()?)
}

fn matcher_matches(matcher: &Gitignore, relative_path: &Path) -> bool {
    matcher
        .matched_path_or_any_parents(relative_path, false)
        .is_ignore()
}

fn default_respect_gitignore() -> bool {
    true
}

fn default_skills_dir() -> String {
    ".agents/skills".to_string()
}

fn default_plans_dir() -> String {
    "docs/exec-plans".to_string()
}

fn validate_repository_relative_dir(field: &str, value: &str) -> Result<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field} must not be empty");
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        bail!("{field} must be a relative path under the repository root");
    }

    for component in path.components() {
        if matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        ) {
            bail!("{field} must stay under the repository root");
        }
    }

    Ok(path.to_path_buf())
}

fn normalize_extension(value: &str) -> String {
    if value.starts_with('.') {
        value.to_string()
    } else {
        format!(".{value}")
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{BudgetLimit, Config, RuleSeverity};
    use crate::diagnostics::Severity;
    use tempfile::TempDir;

    #[test]
    fn load_applies_embedded_default_when_no_config_found() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert!(config.config_path.is_none());
        assert!(!config.rule_applications.is_empty());
        assert!(!config.frontmatter_rules.is_empty());
        assert_eq!(config.skills_dir, Path::new(".agents/skills"));
        assert_eq!(config.plans_dir, Path::new("docs/exec-plans"));
    }

    #[test]
    fn load_ignores_nested_config_when_root_config_is_absent() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        let nested = repository_root.join("docs");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("docgarden.toml"),
            "[[rules]]\npath = \"docs/**\"\nenable = [\"prefer-links-for-local-paths\"]\n",
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(
            config.repository_root,
            repository_root.canonicalize().unwrap()
        );
        assert!(config.config_path.is_none());
        // embedded default applies — rules come from it, not the nested docs/docgarden.toml
        assert!(!config.rule_applications.is_empty());
        assert_eq!(config.include, vec!["*.md"]);
    }

    #[test]
    fn load_applies_config_keys_and_override_sets() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
respect_gitignore = false
skills_dir = ".codex/skills"
plans_dir = "plans"
extend_extensions = ["proto", ".rego"]
remove_extensions = ["md"]
extend_special_filenames = ["Tiltfile"]
remove_special_filenames = ["LICENSE"]

[[rules]]
path = "docs/generated/**"
disable = ["unresolved-link-path"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(config.config_path, Some(config_path));
        assert!(!config.config_was_explicit);
        assert!(!config.respect_gitignore);
        assert_eq!(config.skills_dir, Path::new(".codex/skills"));
        assert_eq!(config.plans_dir, Path::new("plans"));
        assert!(config.known_extensions.contains(".proto"));
        assert!(config.known_extensions.contains(".rego"));
        assert!(!config.known_extensions.contains(".md"));
        assert!(config.special_filenames.contains("Tiltfile"));
        assert!(!config.special_filenames.contains("LICENSE"));

        let generated_policy = config
            .effective_rule_policy_for_path("docs/generated/output.md")
            .unwrap();
        assert!(
            generated_policy
                .ignored_rules
                .contains("unresolved-link-path")
        );

        let other_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(!other_policy.ignored_rules.contains("unresolved-link-path"));
    }

    #[test]
    fn load_rejects_scope_dirs_outside_repository_root() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        fs::write(
            repository_root.join("docgarden.toml"),
            "skills_dir = \"../skills\"\n",
        )
        .unwrap();
        let err = Config::load(&repository_root, None).unwrap_err();
        assert!(err.to_string().contains("skills_dir must stay under"));

        fs::write(repository_root.join("docgarden.toml"), "plans_dir = \"\"\n").unwrap();
        let err = Config::load(&repository_root, None).unwrap_err();
        assert!(err.to_string().contains("plans_dir must not be empty"));
    }

    #[test]
    fn compiled_rule_target_matches_path_and_respects_excludes() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        let target =
            super::CompiledRuleTarget::new(&repository_root, "**/*.md", &["AGENTS.md".to_string()])
                .unwrap();

        assert!(target.matches(Path::new("docs/guide.md")));
        assert!(!target.matches(Path::new("AGENTS.md")));
        assert!(!target.matches(Path::new("Cargo.toml")));
    }

    #[test]
    fn load_defaults_to_respecting_gitignore() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert!(config.respect_gitignore);
    }

    #[test]
    fn config_debug_reports_stable_summary_fields() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        let config = Config::load(&repository_root, None).unwrap();
        let rendered = format!("{config:?}");

        assert!(rendered.contains("Config"));
        assert!(rendered.contains("repository_root"));
        assert!(rendered.contains("rule_application_count:"));
        assert!(rendered.contains("frontmatter_rule_count:"));
        assert!(rendered.contains("respect_gitignore: true"));
    }

    #[test]
    fn load_lowers_rules_into_effective_config() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "docs/references/**"
disable = ["unresolved-link-path"]
reason = "Imported references may contain source-derived paths."

[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path"]

[[rules]]
path = "README.md"
enable = ["prefer-links-for-local-paths"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let refs_policy = config
            .effective_rule_policy_for_path("docs/references/source.md")
            .unwrap();
        assert!(refs_policy.ignored_rules.contains("unresolved-link-path"));
        assert!(refs_policy.backtick_path_severity.is_some());

        let readme_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(readme_policy.prefer_links_for_local_paths);
        assert!(readme_policy.backtick_path_severity.is_none());

        let docs_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert!(docs_policy.backtick_path_severity.is_some());
        assert!(!docs_policy.prefer_links_for_local_paths);
    }

    #[test]
    fn path_scoped_settings_match_descendants_of_directory_patterns() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(repository_root.join("docs/references")).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "docs/references"
disable = ["unresolved-link-path"]

[[rules]]
path = "docs/references"
enable = ["unresolved-backtick-path"]

[[rules]]
path = "docs/references"
enable = ["prefer-links-for-local-paths"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let policy = config
            .effective_rule_policy_for_path("docs/references/source.md")
            .unwrap();
        assert!(policy.ignored_rules.contains("unresolved-link-path"));
        assert!(policy.backtick_path_severity.is_some());
        assert!(policy.prefer_links_for_local_paths);
    }

    #[test]
    fn load_lowers_context_budget_rules_with_entry_level_severity() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "README.md"
max_tokens = 10
max_lines = 5

[[rules]]
path = "README.md"
max_lines = 20
severity = "warn"

[[rules]]
path = "docs/references/**"
disable = ["max_tokens"]
reason = "References preserve source fidelity."
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let readme = config.effective_rule_policy_for_path("README.md").unwrap();
        assert_eq!(
            readme.max_tokens,
            Some(BudgetLimit {
                limit: 10,
                severity: RuleSeverity::Error,
            })
        );
        assert_eq!(
            readme.max_lines,
            Some(BudgetLimit {
                limit: 20,
                severity: RuleSeverity::Warn,
            })
        );

        let reference = config
            .effective_rule_policy_for_path("docs/references/source.md")
            .unwrap();
        assert_eq!(reference.max_tokens, None);
        assert_eq!(reference.max_lines, None);
    }

    #[test]
    fn load_rejects_zero_context_budget_limits() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
[[rules]]
path = "README.md"
max_tokens = 0
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("max_tokens must be greater than zero"));
    }

    #[test]
    fn load_rejects_removed_config_shapes() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
[[documents]]
name = "docs"
match = "docs/**"
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
match = "docs/**"
disable = ["unresolved-link-path"]
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(&config_path, "[per-file-ignores]\n").unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(&config_path, "report-ambiguous-inline-code = true\n").unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(&config_path, "local-reference-style = \"backticks\"\n").unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(&config_path, "path_style = \"links\"\n").unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));
    }

    #[test]
    fn load_rejects_unknown_rules_and_future_rule_options() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
[[rules]]
path = "docs/**"
disable = ["context-budget"]
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported rule `context-budget` in `disable`"));

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "docs/**"
rule = "context-budget"
max-lines = 500
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "docs/**"
max-tokens = 500
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
scope = "skills"
max_tokens = 500
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "docs/**"
enabled = false
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "docs/**"
path_style = "links"
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));
    }

    #[test]
    fn frontmatter_rules_parse_and_lower_correctly() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
max_chars = 1024

[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let guide_policy = config.frontmatter_policy_for_path("docs/guide.md").unwrap();
        assert_eq!(guide_policy.required, vec!["description".to_string()]);
        assert_eq!(guide_policy.field_max_chars.get("description"), Some(&1024));

        let agents_policy = config.frontmatter_policy_for_path("AGENTS.md").unwrap();
        assert_eq!(agents_policy.required, Vec::<String>::new());
        assert_eq!(
            agents_policy.field_max_chars.get("description"),
            Some(&1024)
        );
    }

    #[test]
    fn frontmatter_policy_merges_multiple_matching_rules_last_wins() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
max_chars = 1024

[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        // Regular .md file: both rules match -> gets both required and max_chars
        let policy = config.frontmatter_policy_for_path("docs/guide.md").unwrap();
        assert_eq!(policy.required, vec!["description".to_string()]);
        assert_eq!(policy.field_max_chars.get("description"), Some(&1024));

        // AGENTS.md: second rule excludes it -> only gets max_chars, not required
        let agents_policy = config.frontmatter_policy_for_path("AGENTS.md").unwrap();
        assert_eq!(agents_policy.required, Vec::<String>::new());
        assert_eq!(
            agents_policy.field_max_chars.get("description"),
            Some(&1024)
        );
    }

    #[test]
    fn frontmatter_rules_reject_unknown_fields() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter]
schema = "agent-skill"
"#,
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
min_chars = 10
"#,
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));
    }

    #[test]
    fn frontmatter_rules_reject_invalid_max_chars() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
max_chars = 0
"#,
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("max_chars must be greater than zero"),
            "got: {error}"
        );
    }

    #[test]
    fn frontmatter_duplicate_field_names_in_single_entry_are_rejected_by_toml() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");

        fs::write(
            &config_path,
            "[[rules]]\npath = \"**/*.md\"\n\n[rules.frontmatter.fields.description]\nmax_chars = 512\n\n[rules.frontmatter.fields.description]\nmax_chars = 1024\n",
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"), "got: {error}");
    }

    #[test]
    fn prefer_links_rule_reports_invalid_path_pattern() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "{docs,README.md"
enable = ["prefer-links-for-local-paths"]
"#,
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid rule path pattern {docs,README.md"));
    }

    #[test]
    fn unresolved_backtick_path_enable_uses_entry_severity() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path"]
severity = "warn"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(config.rule_applications.len(), 1);
        assert_eq!(config.rule_applications[0].severity, RuleSeverity::Warn);

        let docs_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert_eq!(docs_policy.backtick_path_severity, Some(Severity::Warning));

        let readme_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(readme_policy.backtick_path_severity.is_none());
    }

    #[test]
    fn disable_then_enable_restores_unresolved_backtick_path() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"
enable = ["unresolved-backtick-path"]

[[rules]]
path = "**/*.md"
disable = ["unresolved-backtick-path"]

[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path"]
severity = "warn"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        // docs/ matches all three entries: enabled, then disabled, then re-enabled
        let docs_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert_eq!(
            docs_policy.backtick_path_severity,
            Some(Severity::Warning),
            "later enable should override earlier disable"
        );

        // README.md matches only the first two: enabled then disabled
        let readme_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(
            readme_policy.backtick_path_severity.is_none(),
            "disable without later re-enable should leave rule off"
        );
    }

    #[test]
    fn disable_then_enable_restores_prefer_links_for_local_paths() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"
enable = ["prefer-links-for-local-paths"]

[[rules]]
path = "**/*.md"
disable = ["prefer-links-for-local-paths"]

[[rules]]
path = "docs/**"
enable = ["prefer-links-for-local-paths"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let docs_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert!(
            docs_policy.prefer_links_for_local_paths,
            "later enable should override earlier disable"
        );

        let readme_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(
            !readme_policy.prefer_links_for_local_paths,
            "disable without later re-enable should leave rule off"
        );
    }

    #[test]
    fn disable_then_enable_restores_max_tokens() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"
max_tokens = 500

[[rules]]
path = "**/*.md"
disable = ["max_tokens"]

[[rules]]
path = "docs/**"
max_tokens = 200
severity = "warn"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let docs_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert_eq!(
            docs_policy.max_tokens,
            Some(BudgetLimit {
                limit: 200,
                severity: RuleSeverity::Warn,
            }),
            "later max_tokens entry should override earlier disable"
        );

        let readme_policy = config.effective_rule_policy_for_path("README.md").unwrap();
        assert!(
            readme_policy.max_tokens.is_none(),
            "disable without later re-enable should leave max_tokens off"
        );
    }

    #[test]
    fn broad_disable_narrow_enable_with_mixed_path_scopes() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"
disable = ["unresolved-backtick-path"]

[[rules]]
path = "docs/internal/**"
enable = ["unresolved-backtick-path"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let internal_policy = config
            .effective_rule_policy_for_path("docs/internal/spec.md")
            .unwrap();
        assert!(
            internal_policy.backtick_path_severity.is_some(),
            "narrow enable should override broad disable for matching path"
        );

        let other_policy = config
            .effective_rule_policy_for_path("docs/guide.md")
            .unwrap();
        assert!(
            other_policy.backtick_path_severity.is_none(),
            "broad disable without matching narrow enable should leave rule off"
        );
    }
}
