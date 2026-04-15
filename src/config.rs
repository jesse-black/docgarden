use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::gitignore::GitignoreBuilder;
use serde::Deserialize;

use crate::defaults::{DEFAULT_SCAN_PATTERNS, default_extensions, default_special_filenames};
use crate::diagnostics::Severity;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LocalReferenceStyle {
    Backticks,
    Links,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
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
    pub path_style: Option<LocalReferenceStyle>,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    pub path: String,
    #[serde(default)]
    pub disable: Option<Vec<String>>,
    #[serde(default)]
    pub enable: Option<Vec<String>>,
    #[serde(default)]
    pub path_style: Option<LocalReferenceStyle>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub max_lines: Option<usize>,
    #[serde(default)]
    pub severity: Option<RuleSeverity>,
    #[serde(default)]
    pub reason: Option<String>,
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

#[derive(Clone, Debug)]
pub struct Config {
    pub repository_root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub per_file_ignores: BTreeMap<String, Vec<String>>,
    pub local_reference_style_overrides: Vec<LocalReferenceStyleOverride>,
    pub local_reference_style: LocalReferenceStyle,
    pub known_extensions: BTreeSet<String>,
    pub special_filenames: BTreeSet<String>,
    pub config_path: Option<PathBuf>,
    pub config_was_explicit: bool,
    pub ambiguous_inline_code_patterns: Vec<String>,
    pub context_budget_rules: Vec<ContextBudgetRule>,
    pub respect_gitignore: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalReferenceStyleOverride {
    pub pattern: String,
    pub style: LocalReferenceStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetLimit {
    pub limit: usize,
    pub severity: RuleSeverity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBudgetRule {
    pub pattern: String,
    pub max_tokens: Option<BudgetLimit>,
    pub max_lines: Option<BudgetLimit>,
    pub disable: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EffectiveContextBudgets {
    pub max_tokens: Option<BudgetLimit>,
    pub max_lines: Option<BudgetLimit>,
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
            FileConfig {
                respect_gitignore: default_respect_gitignore(),
                ..FileConfig::default()
            }
        };

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

        let local_reference_style = parsed.path_style.unwrap_or(LocalReferenceStyle::Backticks);
        let rule_applications = lower_rules(parsed.rules)?;

        Ok(Self {
            repository_root,
            include,
            exclude: parsed.exclude,
            per_file_ignores: rule_applications.per_file_ignores,
            local_reference_style_overrides: rule_applications.local_reference_style_overrides,
            local_reference_style,
            known_extensions,
            special_filenames,
            config_path,
            config_was_explicit,
            ambiguous_inline_code_patterns: rule_applications.ambiguous_inline_code_patterns,
            context_budget_rules: rule_applications.context_budget_rules,
            respect_gitignore: parsed.respect_gitignore,
        })
    }

    pub fn local_reference_style_for_path(
        &self,
        relative_path: &str,
    ) -> Result<LocalReferenceStyle> {
        let mut style = self.local_reference_style;
        for entry in &self.local_reference_style_overrides {
            if pattern_matches(&self.repository_root, &entry.pattern, relative_path)? {
                style = entry.style;
            }
        }
        Ok(style)
    }

    pub fn report_ambiguous_inline_code_for_path(&self, relative_path: &str) -> Result<bool> {
        for pattern in &self.ambiguous_inline_code_patterns {
            if pattern_matches(&self.repository_root, pattern, relative_path)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn context_budgets_for_path(&self, relative_path: &str) -> Result<EffectiveContextBudgets> {
        let mut budgets = EffectiveContextBudgets::default();
        for entry in &self.context_budget_rules {
            if !pattern_matches(&self.repository_root, &entry.pattern, relative_path)? {
                continue;
            }
            if entry.disable.contains("max_tokens") {
                budgets.max_tokens = None;
            }
            if entry.disable.contains("max_lines") {
                budgets.max_lines = None;
            }
            if let Some(limit) = entry.max_tokens {
                budgets.max_tokens = Some(limit);
            }
            if let Some(limit) = entry.max_lines {
                budgets.max_lines = Some(limit);
            }
        }
        Ok(budgets)
    }
}

#[derive(Default)]
struct RuleApplications {
    per_file_ignores: BTreeMap<String, Vec<String>>,
    local_reference_style_overrides: Vec<LocalReferenceStyleOverride>,
    ambiguous_inline_code_patterns: Vec<String>,
    context_budget_rules: Vec<ContextBudgetRule>,
}

fn lower_rules(rules: Vec<RuleConfig>) -> Result<RuleApplications> {
    let mut applications = RuleApplications::default();
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
        let disabled_rules = rule.disable.unwrap_or_default();
        if !disabled_rules.is_empty() {
            validate_rule_list("disable", &disabled_rules, is_known_rule)?;
            let global_ignored_rules: Vec<String> = disabled_rules
                .iter()
                .filter(|rule| !matches!(rule.as_str(), "max_tokens" | "max_lines"))
                .cloned()
                .collect();
            if !global_ignored_rules.is_empty() {
                applications
                    .per_file_ignores
                    .entry(pattern.clone())
                    .or_default()
                    .extend(global_ignored_rules);
            }
        }
        if let Some(enabled_rules) = rule.enable {
            validate_rule_list("enable", &enabled_rules, is_supported_enabled_rule)?;
            if enabled_rules
                .iter()
                .any(|rule| rule == "ambiguous-inline-code")
            {
                applications
                    .ambiguous_inline_code_patterns
                    .push(pattern.clone());
            }
        }
        if let Some(style) = rule.path_style {
            applications
                .local_reference_style_overrides
                .push(LocalReferenceStyleOverride {
                    pattern: pattern.clone(),
                    style,
                });
        }
        if rule.max_tokens.is_some() || rule.max_lines.is_some() || !disabled_rules.is_empty() {
            let severity = rule.severity.unwrap_or(RuleSeverity::Error);
            let max_tokens = rule
                .max_tokens
                .map(|limit| budget_limit("max_tokens", limit, severity))
                .transpose()?;
            let max_lines = rule
                .max_lines
                .map(|limit| budget_limit("max_lines", limit, severity))
                .transpose()?;
            let disabled_budget_rules: BTreeSet<String> = disabled_rules
                .into_iter()
                .filter(|rule| matches!(rule.as_str(), "max_tokens" | "max_lines"))
                .collect();
            if max_tokens.is_some() || max_lines.is_some() || !disabled_budget_rules.is_empty() {
                applications.context_budget_rules.push(ContextBudgetRule {
                    pattern,
                    max_tokens,
                    max_lines,
                    disable: disabled_budget_rules,
                });
            }
        }
    }
    Ok(applications)
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
        "unresolved-local-path"
            | "prefer-links-for-local-paths"
            | "prefer-backticks-for-local-paths"
            | "ambiguous-inline-code"
            | "max_tokens"
            | "max_lines"
    )
}

fn is_supported_enabled_rule(rule: &str) -> bool {
    rule == "ambiguous-inline-code"
}

fn pattern_matches(root: &Path, pattern: &str, relative_path: &str) -> Result<bool> {
    let mut builder = GitignoreBuilder::new(root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid rule path pattern {pattern}"))?;
    let matcher = builder.build()?;
    Ok(matcher
        .matched_path_or_any_parents(Path::new(relative_path), false)
        .is_ignore())
}

fn default_respect_gitignore() -> bool {
    true
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
    use std::fs;

    use crate::diagnostics::ignored_rules_for_path;

    use super::{BudgetLimit, Config, LocalReferenceStyle, RuleSeverity};
    use tempfile::TempDir;

    #[test]
    fn load_ignores_nested_config_when_root_config_is_absent() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        let nested = repository_root.join("docs");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("docgarden.toml"), "path_style = \"links\"\n").unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(
            config.repository_root,
            repository_root.canonicalize().unwrap()
        );
        assert!(config.config_path.is_none());
        assert_eq!(config.local_reference_style, LocalReferenceStyle::Backticks);
        assert_eq!(
            config.include,
            vec!["docs/**", "README.md", "AGENTS.md", "*.md"]
        );
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
extend_extensions = ["proto", ".rego"]
remove_extensions = ["md"]
extend_special_filenames = ["Tiltfile"]
remove_special_filenames = ["LICENSE"]
path_style = "links"

[[rules]]
path = "docs/generated/**"
disable = ["ambiguous-inline-code"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(config.config_path, Some(config_path));
        assert!(!config.config_was_explicit);
        assert_eq!(config.local_reference_style, LocalReferenceStyle::Links);
        assert!(!config.respect_gitignore);
        assert!(config.known_extensions.contains(".proto"));
        assert!(config.known_extensions.contains(".rego"));
        assert!(!config.known_extensions.contains(".md"));
        assert!(config.special_filenames.contains("Tiltfile"));
        assert!(!config.special_filenames.contains("LICENSE"));
        assert_eq!(
            config.per_file_ignores.get("docs/generated/**").unwrap(),
            &vec!["ambiguous-inline-code".to_string()]
        );
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
    fn load_lowers_rules_into_effective_config() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "docs/references/**"
disable = ["unresolved-local-path"]
reason = "Imported references may contain source-derived paths."

[[rules]]
path = "docs/**"
enable = ["ambiguous-inline-code"]

[[rules]]
path = "README.md"
path_style = "links"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(
            config.per_file_ignores.get("docs/references/**").unwrap(),
            &vec!["unresolved-local-path".to_string()]
        );
        assert_eq!(
            config.ambiguous_inline_code_patterns,
            vec!["docs/**".to_string()]
        );
        assert_eq!(
            config.local_reference_style_for_path("README.md").unwrap(),
            LocalReferenceStyle::Links
        );
        assert_eq!(
            config
                .local_reference_style_for_path("docs/guide.md")
                .unwrap(),
            LocalReferenceStyle::Backticks
        );
        assert!(
            config
                .report_ambiguous_inline_code_for_path("docs/guide.md")
                .unwrap()
        );
        assert!(
            !config
                .report_ambiguous_inline_code_for_path("README.md")
                .unwrap()
        );
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
disable = ["unresolved-local-path"]

[[rules]]
path = "docs/references"
enable = ["ambiguous-inline-code"]

[[rules]]
path = "docs/references"
path_style = "links"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(
            ignored_rules_for_path(
                &config.repository_root,
                &config.per_file_ignores,
                "docs/references/source.md",
            )
            .unwrap(),
            ["unresolved-local-path".to_string()].into_iter().collect()
        );
        assert!(
            config
                .report_ambiguous_inline_code_for_path("docs/references/source.md")
                .unwrap()
        );
        assert_eq!(
            config
                .local_reference_style_for_path("docs/references/source.md")
                .unwrap(),
            LocalReferenceStyle::Links
        );
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

        let readme = config.context_budgets_for_path("README.md").unwrap();
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
            .context_budgets_for_path("docs/references/source.md")
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
disable = ["unresolved-local-path"]
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
    }

    #[test]
    fn local_reference_style_reports_invalid_path_pattern() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "{docs,README.md"
path_style = "links"
"#,
        )
        .unwrap();
        let config = Config::load(&repository_root, None).unwrap();

        let error = config
            .local_reference_style_for_path("README.md")
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid rule path pattern {docs,README.md"));
    }
}
