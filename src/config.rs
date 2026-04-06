use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ignore::gitignore::GitignoreBuilder;
use serde::Deserialize;

use crate::defaults::{DEFAULT_SCAN_PATTERNS, default_extensions, default_special_filenames};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LocalReferenceStyle {
    Backticks,
    Links,
}

#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default, rename = "extend-extensions", alias = "extend_extensions")]
    pub extend_extensions: Vec<String>,
    #[serde(default, rename = "remove-extensions", alias = "remove_extensions")]
    pub remove_extensions: Vec<String>,
    #[serde(
        default,
        rename = "extend-special-filenames",
        alias = "extend_special_filenames"
    )]
    pub extend_special_filenames: Vec<String>,
    #[serde(
        default,
        rename = "remove-special-filenames",
        alias = "remove_special_filenames"
    )]
    pub remove_special_filenames: Vec<String>,
    #[serde(default, rename = "per-file-ignores", alias = "per_file_ignores")]
    pub per_file_ignores: BTreeMap<String, Vec<String>>,
    #[serde(
        default,
        rename = "local-reference-style",
        alias = "local_reference_style"
    )]
    pub local_reference_style: Option<LocalReferenceStyle>,
    #[serde(
        default,
        rename = "report-ambiguous-inline-code",
        alias = "report_ambiguous_inline_code"
    )]
    pub report_ambiguous_inline_code: bool,
    #[serde(default = "default_respect_gitignore")]
    #[serde(rename = "respect-gitignore", alias = "respect_gitignore")]
    pub respect_gitignore: bool,
    #[serde(default)]
    pub documents: Vec<DocumentConfig>,
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentConfig {
    pub name: String,
    #[serde(rename = "match")]
    pub match_pattern: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    #[serde(rename = "match")]
    pub match_target: String,
    #[serde(default)]
    pub disable: Option<Vec<String>>,
    #[serde(default)]
    pub enable: Option<Vec<String>>,
    #[serde(
        default,
        rename = "local-reference-style",
        alias = "local_reference_style"
    )]
    pub local_reference_style: Option<LocalReferenceStyle>,
    #[serde(default)]
    pub reason: Option<String>,
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
    pub report_ambiguous_inline_code: bool,
    pub ambiguous_inline_code_patterns: Vec<String>,
    pub respect_gitignore: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalReferenceStyleOverride {
    pub pattern: String,
    pub style: LocalReferenceStyle,
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

        let local_reference_style = parsed
            .local_reference_style
            .unwrap_or(LocalReferenceStyle::Backticks);
        let document_patterns = document_patterns(parsed.documents)?;
        let rule_applications = lower_rules(parsed.rules, &document_patterns)?;
        let mut per_file_ignores = parsed.per_file_ignores;
        for (pattern, disabled_rules) in rule_applications.per_file_ignores {
            per_file_ignores
                .entry(pattern)
                .or_default()
                .extend(disabled_rules);
        }

        Ok(Self {
            repository_root,
            include,
            exclude: parsed.exclude,
            per_file_ignores,
            local_reference_style_overrides: rule_applications.local_reference_style_overrides,
            local_reference_style,
            known_extensions,
            special_filenames,
            config_path,
            config_was_explicit,
            report_ambiguous_inline_code: parsed.report_ambiguous_inline_code,
            ambiguous_inline_code_patterns: rule_applications.ambiguous_inline_code_patterns,
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
        if self.report_ambiguous_inline_code {
            return Ok(true);
        }
        for pattern in &self.ambiguous_inline_code_patterns {
            if pattern_matches(&self.repository_root, pattern, relative_path)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Default)]
struct RuleApplications {
    per_file_ignores: BTreeMap<String, Vec<String>>,
    local_reference_style_overrides: Vec<LocalReferenceStyleOverride>,
    ambiguous_inline_code_patterns: Vec<String>,
}

fn document_patterns(documents: Vec<DocumentConfig>) -> Result<BTreeMap<String, String>> {
    let mut patterns = BTreeMap::new();
    for document in documents {
        if document.name.trim().is_empty() {
            bail!("document family name must not be empty");
        }
        if document.match_pattern.trim().is_empty() {
            bail!(
                "document family `{}` match must not be empty",
                document.name
            );
        }
        if let Some(kind) = &document.kind
            && kind.trim().is_empty()
        {
            bail!("document family `{}` kind must not be empty", document.name);
        }
        if patterns
            .insert(document.name.clone(), document.match_pattern)
            .is_some()
        {
            bail!("duplicate document family `{}`", document.name);
        }
    }
    Ok(patterns)
}

fn lower_rules(
    rules: Vec<RuleConfig>,
    document_patterns: &BTreeMap<String, String>,
) -> Result<RuleApplications> {
    let mut applications = RuleApplications::default();
    for rule in rules {
        if rule.match_target.trim().is_empty() {
            bail!("rule match must not be empty");
        }
        if let Some(reason) = &rule.reason
            && reason.trim().is_empty()
        {
            bail!("rule reason must not be empty");
        }
        let pattern = document_patterns
            .get(&rule.match_target)
            .cloned()
            .unwrap_or_else(|| rule.match_target.clone());
        if let Some(disabled_rules) = rule.disable {
            validate_rule_list("disable", &disabled_rules, is_known_rule)?;
            applications
                .per_file_ignores
                .entry(pattern.clone())
                .or_default()
                .extend(disabled_rules);
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
        if let Some(style) = rule.local_reference_style {
            applications
                .local_reference_style_overrides
                .push(LocalReferenceStyleOverride { pattern, style });
        }
    }
    Ok(applications)
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
    )
}

fn is_supported_enabled_rule(rule: &str) -> bool {
    rule == "ambiguous-inline-code"
}

fn pattern_matches(root: &Path, pattern: &str, relative_path: &str) -> Result<bool> {
    let mut builder = GitignoreBuilder::new(root);
    builder
        .add_line(None, pattern)
        .with_context(|| format!("invalid rule match pattern {pattern}"))?;
    let matcher = builder.build()?;
    Ok(matcher.matched(relative_path, false).is_ignore())
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

    use super::{Config, LocalReferenceStyle};
    use tempfile::TempDir;

    #[test]
    fn load_ignores_nested_config_when_root_config_is_absent() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        let nested = repository_root.join("docs");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("docgarden.toml"),
            "local-reference-style = \"links\"\n",
        )
        .unwrap();

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
    fn load_applies_alias_keys_and_override_sets() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
local_reference_style = "links"
report_ambiguous_inline_code = true
respect_gitignore = false
extend_extensions = ["proto", ".rego"]
remove_extensions = ["md"]
extend_special_filenames = ["Tiltfile"]
remove_special_filenames = ["LICENSE"]

[per_file_ignores]
"docs/generated/**" = ["ambiguous-inline-code"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(config.config_path, Some(config_path));
        assert!(!config.config_was_explicit);
        assert_eq!(config.local_reference_style, LocalReferenceStyle::Links);
        assert!(config.report_ambiguous_inline_code);
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
    fn load_expands_document_family_rules_into_effective_config() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[documents]]
name = "references"
match = "docs/references/**"
kind = "reference"

[[rules]]
match = "references"
disable = ["unresolved-local-path"]
reason = "Imported references may contain source-derived paths."

[[rules]]
match = "docs/**"
enable = ["ambiguous-inline-code"]

[[rules]]
match = "README.md"
local-reference-style = "links"
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
    fn load_rejects_duplicate_document_families() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[documents]]
name = "docs"
match = "docs/**"

[[documents]]
name = "docs"
match = "other/**"
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("duplicate document family `docs`"));
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
match = "docs/**"
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
match = "docs/**"
rule = "context-budget"
max-lines = 500
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));
    }
}
