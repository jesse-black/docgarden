use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Deserializer};

use crate::defaults::{DEFAULT_SCAN_PATTERNS, default_extensions, default_special_filenames};
use crate::diagnostics::Severity;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
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
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
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
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RuleConfig {
    pub path: String,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub disable: Vec<Rule>,
    #[serde(default)]
    pub enable: Vec<Rule>,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Rule {
    UnresolvedLinkPath,
    UnresolvedBacktickPath,
    PreferLinksForLocalPaths,
    MaxTokens,
    MaxLines,
    FrontmatterFieldMissing,
    FrontmatterMalformed,
    FrontmatterFieldMaxChars,
}

impl Rule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnresolvedLinkPath => "unresolved-link-path",
            Self::UnresolvedBacktickPath => "unresolved-backtick-path",
            Self::PreferLinksForLocalPaths => "prefer-links-for-local-paths",
            Self::MaxTokens => "max-tokens",
            Self::MaxLines => "max-lines",
            Self::FrontmatterFieldMissing => "frontmatter-field-missing",
            Self::FrontmatterMalformed => "frontmatter-malformed",
            Self::FrontmatterFieldMaxChars => "frontmatter-field-max-chars",
        }
    }

    fn supported_in_enable(self) -> bool {
        matches!(
            self,
            Self::UnresolvedBacktickPath | Self::PreferLinksForLocalPaths
        )
    }
}

impl FromStr for Rule {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "unresolved-link-path" => Ok(Self::UnresolvedLinkPath),
            "unresolved-backtick-path" => Ok(Self::UnresolvedBacktickPath),
            "prefer-links-for-local-paths" => Ok(Self::PreferLinksForLocalPaths),
            "max-tokens" => Ok(Self::MaxTokens),
            "max-lines" => Ok(Self::MaxLines),
            "frontmatter-field-missing" => Ok(Self::FrontmatterFieldMissing),
            "frontmatter-malformed" => Ok(Self::FrontmatterMalformed),
            "frontmatter-field-max-chars" => Ok(Self::FrontmatterFieldMaxChars),
            _ => Err(format!("unsupported rule `{value}`")),
        }
    }
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Rule::from_str(&value).map_err(serde::de::Error::custom)
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
    pub disable: Vec<Rule>,
    pub enable: Vec<Rule>,
    /// Severity used when `enable` contains `Rule::UnresolvedBacktickPath`.
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
    pub ignored_rules: BTreeSet<Rule>,
    pub backtick_path_severity: Option<Severity>,
    pub prefer_links_for_local_paths: bool,
    pub max_tokens: Option<BudgetLimit>,
    pub max_lines: Option<BudgetLimit>,
}

#[derive(Clone)]
pub struct Config {
    repository_root: PathBuf,
    skills_dir: PathBuf,
    plans_dir: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    rule_applications: Vec<RuleApplication>,
    known_extensions: BTreeSet<String>,
    special_filenames: BTreeSet<String>,
    config_path: Option<PathBuf>,
    config_was_explicit: bool,
    frontmatter_rules: Vec<FrontmatterRule>,
    respect_gitignore: bool,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Config {
            repository_root,
            skills_dir,
            plans_dir,
            include,
            exclude,
            rule_applications,
            known_extensions,
            special_filenames,
            config_path,
            config_was_explicit,
            frontmatter_rules,
            respect_gitignore,
        } = self;

        f.debug_struct("Config")
            .field("repository_root", repository_root)
            .field("skills_dir", skills_dir)
            .field("plans_dir", plans_dir)
            .field("include", include)
            .field("exclude", exclude)
            .field("rule_application_count", &rule_applications.len())
            .field("known_extensions", known_extensions)
            .field("special_filenames", special_filenames)
            .field("config_path", config_path)
            .field("config_was_explicit", config_was_explicit)
            .field("frontmatter_rule_count", &frontmatter_rules.len())
            .field("respect_gitignore", respect_gitignore)
            .finish()
    }
}

impl Config {
    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    pub fn include(&self) -> &[String] {
        &self.include
    }

    pub fn exclude(&self) -> &[String] {
        &self.exclude
    }

    pub fn known_extensions(&self) -> &BTreeSet<String> {
        &self.known_extensions
    }

    pub fn special_filenames(&self) -> &BTreeSet<String> {
        &self.special_filenames
    }

    pub fn config_was_explicit(&self) -> bool {
        self.config_was_explicit
    }

    pub fn respect_gitignore(&self) -> bool {
        self.respect_gitignore
    }

    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    pub fn disable_gitignore(&mut self) {
        self.respect_gitignore = false;
    }

    /// Construct a minimal `Config` for unit tests that need a `Config` but do
    /// not require filesystem-backed loading.  Uses default extensions and
    /// special filenames; all other collections are empty.
    #[cfg(test)]
    pub(crate) fn for_testing(repository_root: impl Into<PathBuf>) -> Self {
        use crate::defaults::{default_extensions, default_special_filenames};
        Self {
            repository_root: repository_root.into(),
            skills_dir: PathBuf::from(".agents/skills"),
            plans_dir: PathBuf::from("docs/exec-plans"),
            include: Vec::new(),
            exclude: Vec::new(),
            rule_applications: Vec::new(),
            known_extensions: default_extensions(),
            special_filenames: default_special_filenames(),
            config_path: None,
            config_was_explicit: false,
            frontmatter_rules: Vec::new(),
            respect_gitignore: true,
        }
    }

    pub fn load(repository_root: &Path, explicit_config: Option<&Path>) -> Result<Self> {
        let repository_root = repository_root
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", repository_root.display()))?;
        let config_was_explicit = explicit_config.is_some();
        let config_path = resolve_config_path(&repository_root, explicit_config);
        let parsed = load_file_config(config_path.as_deref())?;

        let skills_dir = validate_repository_relative_dir("skills-dir", &parsed.skills_dir)?;
        let plans_dir = validate_repository_relative_dir("plans-dir", &parsed.plans_dir)?;

        let known_extensions = merge_extensions(parsed.extend_extensions, parsed.remove_extensions);
        let special_filenames = merge_special_filenames(
            parsed.extend_special_filenames,
            parsed.remove_special_filenames,
        );
        let include = include_patterns_or_default(parsed.include);

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
    ///   `prefer-links-for-local-paths`, `max-tokens`, `max-lines`) clears
    ///   their state directly so a later `enable` can restore them.
    /// - `disable` for always-on rules (e.g. `unresolved-link-path`) adds to
    ///   `ignored_rules`; always-on rules cannot be re-enabled via `enable`.
    pub fn effective_rule_policy_for_path(&self, relative_path: &str) -> EffectiveRulePolicy {
        let relative_path = Path::new(relative_path);
        let mut policy = EffectiveRulePolicy::default();

        for app in &self.rule_applications {
            if !app.matches(relative_path) {
                continue;
            }

            for rule in &app.disable {
                match rule {
                    Rule::UnresolvedBacktickPath => policy.backtick_path_severity = None,
                    Rule::PreferLinksForLocalPaths => policy.prefer_links_for_local_paths = false,
                    Rule::MaxTokens => policy.max_tokens = None,
                    Rule::MaxLines => policy.max_lines = None,
                    Rule::UnresolvedLinkPath
                    | Rule::FrontmatterFieldMissing
                    | Rule::FrontmatterMalformed
                    | Rule::FrontmatterFieldMaxChars => {
                        policy.ignored_rules.insert(*rule);
                    }
                }
            }

            for rule in &app.enable {
                match rule {
                    Rule::UnresolvedBacktickPath => {
                        policy.backtick_path_severity = Some(app.severity.into());
                    }
                    Rule::PreferLinksForLocalPaths => policy.prefer_links_for_local_paths = true,
                    Rule::UnresolvedLinkPath
                    | Rule::MaxTokens
                    | Rule::MaxLines
                    | Rule::FrontmatterFieldMissing
                    | Rule::FrontmatterMalformed
                    | Rule::FrontmatterFieldMaxChars => {}
                }
            }

            if let Some(limit) = app.max_tokens {
                policy.max_tokens = Some(limit);
            }
            if let Some(limit) = app.max_lines {
                policy.max_lines = Some(limit);
            }
        }

        policy
    }

    pub fn frontmatter_policy_for_path(&self, relative_path: &str) -> EffectiveFrontmatterPolicy {
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
        policy
    }

    pub fn skills_root(&self) -> PathBuf {
        self.repository_root.join(&self.skills_dir)
    }

    pub fn plans_root(&self) -> PathBuf {
        self.repository_root.join(&self.plans_dir)
    }
}

fn resolve_config_path(repository_root: &Path, explicit_config: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit_config {
        return Some(path.to_path_buf());
    }

    let candidate = repository_root.join("docgarden.toml");
    candidate.exists().then_some(candidate)
}

fn load_file_config(config_path: Option<&Path>) -> Result<FileConfig> {
    if let Some(path) = config_path {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str::<FileConfig>(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    } else {
        toml::from_str::<FileConfig>(include_str!("data/default-config.toml"))
            .context("failed to parse embedded default config")
    }
}

fn merge_extensions(extend: Vec<String>, remove: Vec<String>) -> BTreeSet<String> {
    let mut known_extensions = default_extensions();
    for extension in extend {
        known_extensions.insert(normalize_extension(&extension));
    }
    for extension in remove {
        known_extensions.remove(&normalize_extension(&extension));
    }
    known_extensions
}

fn merge_special_filenames(extend: Vec<String>, remove: Vec<String>) -> BTreeSet<String> {
    let mut special_filenames = default_special_filenames();
    for filename in extend {
        special_filenames.insert(filename);
    }
    for filename in remove {
        special_filenames.remove(&filename);
    }
    special_filenames
}

fn include_patterns_or_default(include: Vec<String>) -> Vec<String> {
    if include.is_empty() {
        DEFAULT_SCAN_PATTERNS
            .iter()
            .map(|value| value.to_string())
            .collect()
    } else {
        include
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
        if let Some(application) = lower_rule_application(root, &rule)? {
            applications.push(application);
        }

        let pattern = rule.path;
        let exclude = rule.exclude;
        if let Some(frontmatter_rule) =
            lower_frontmatter_rule(root, pattern, exclude, rule.frontmatter)?
        {
            frontmatter_rules.push(frontmatter_rule);
        }
    }
    Ok((applications, frontmatter_rules))
}

fn lower_rule_application(root: &Path, rule: &RuleConfig) -> Result<Option<RuleApplication>> {
    for enabled_rule in &rule.enable {
        if !enabled_rule.supported_in_enable() {
            bail!("unsupported rule `{}` in `enable`", enabled_rule.as_str());
        }
    }

    let severity = rule.severity.unwrap_or(RuleSeverity::Error);
    let max_tokens = rule
        .max_tokens
        .map(|limit| budget_limit("max-tokens", limit, severity))
        .transpose()?;
    let max_lines = rule
        .max_lines
        .map(|limit| budget_limit("max-lines", limit, severity))
        .transpose()?;

    let has_rule_content = !rule.disable.is_empty()
        || !rule.enable.is_empty()
        || max_tokens.is_some()
        || max_lines.is_some();

    if !has_rule_content {
        return Ok(None);
    }

    Ok(Some(RuleApplication {
        target: CompiledRuleTarget::new(root, &rule.path, &rule.exclude)?,
        disable: rule.disable.clone(),
        enable: rule.enable.clone(),
        severity,
        max_tokens,
        max_lines,
    }))
}

fn lower_frontmatter_rule(
    root: &Path,
    pattern: String,
    exclude: Vec<String>,
    frontmatter: Option<FrontmatterRuleConfig>,
) -> Result<Option<FrontmatterRule>> {
    let Some(frontmatter) = frontmatter else {
        return Ok(None);
    };

    let mut field_max_chars = BTreeMap::new();
    for (field_name, field_cfg) in frontmatter.fields {
        if field_name.trim().is_empty() {
            bail!("frontmatter field name must not be empty");
        }
        if let Some(max_chars) = field_cfg.max_chars {
            if max_chars == 0 {
                bail!("frontmatter field `{field_name}` max-chars must be greater than zero");
            }
            field_max_chars.insert(field_name, max_chars);
        }
    }

    if frontmatter.required.is_empty() && field_max_chars.is_empty() {
        return Ok(None);
    }

    Ok(Some(FrontmatterRule::new(
        root,
        pattern,
        exclude,
        frontmatter.required,
        field_max_chars,
    )?))
}

fn budget_limit(rule: &str, limit: usize, severity: RuleSeverity) -> Result<BudgetLimit> {
    if limit == 0 {
        bail!("{rule} must be greater than zero");
    }
    Ok(BudgetLimit { limit, severity })
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
    use std::{fs, path::Path, str::FromStr};

    use super::{BudgetLimit, Config, Rule, RuleSeverity};
    use crate::diagnostics::Severity;
    use tempfile::TempDir;

    #[test]
    fn load_applies_embedded_default_when_no_config_found() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert!(config.config_path().is_none());
        let agents_policy = config.effective_rule_policy_for_path("AGENTS.md");
        assert!(
            agents_policy.max_lines.is_some() || agents_policy.max_tokens.is_some(),
            "embedded default config should include rules"
        );
        assert!(
            config
                .frontmatter_policy_for_path("README.md")
                .field_max_chars
                .contains_key("description"),
            "embedded default config should include frontmatter rules"
        );
        assert_eq!(
            config.skills_root(),
            config.repository_root().join(".agents/skills")
        );
        assert_eq!(
            config.plans_root(),
            config.repository_root().join("docs/exec-plans")
        );
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
            config.repository_root(),
            repository_root.canonicalize().unwrap()
        );
        assert!(config.config_path().is_none());
        let agents_policy = config.effective_rule_policy_for_path("AGENTS.md");
        assert!(
            agents_policy.max_lines.is_some() || agents_policy.max_tokens.is_some(),
            "embedded default config should include rules"
        );
        assert_eq!(config.include(), &["*.md".to_string()]);
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
respect-gitignore = false
skills-dir = ".codex/skills"
plans-dir = "plans"
extend-extensions = ["proto", ".rego"]
remove-extensions = ["md"]
extend-special-filenames = ["Tiltfile"]
remove-special-filenames = ["LICENSE"]

[[rules]]
path = "docs/generated/**"
disable = ["unresolved-link-path"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        assert_eq!(config.config_path(), Some(config_path.as_path()));
        assert!(!config.config_was_explicit());
        assert!(!config.respect_gitignore());
        assert_eq!(
            config.skills_root(),
            config.repository_root().join(".codex/skills")
        );
        assert_eq!(config.plans_root(), config.repository_root().join("plans"));
        assert!(config.known_extensions().contains(".proto"));
        assert!(config.known_extensions().contains(".rego"));
        assert!(!config.known_extensions().contains(".md"));
        assert!(config.special_filenames().contains("Tiltfile"));
        assert!(!config.special_filenames().contains("LICENSE"));

        let generated_policy = config.effective_rule_policy_for_path("docs/generated/output.md");
        assert!(
            generated_policy
                .ignored_rules
                .contains(&Rule::UnresolvedLinkPath)
        );

        let other_policy = config.effective_rule_policy_for_path("README.md");
        assert!(
            !other_policy
                .ignored_rules
                .contains(&Rule::UnresolvedLinkPath)
        );
    }

    #[test]
    fn load_uses_explicit_config_and_non_default_include_patterns() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&repository_root).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("docgarden.toml");
        fs::write(
            &config_path,
            r#"
include = ["docs/**/*.md"]

[[rules]]
path = "**/*.md"
disable = ["max-lines"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, Some(&config_path)).unwrap();

        assert_eq!(config.config_path(), Some(config_path.as_path()));
        assert!(config.config_was_explicit());
        assert_eq!(config.include(), &["docs/**/*.md".to_string()]);
        let policy = config.effective_rule_policy_for_path("README.md");
        assert_eq!(policy.max_lines, None);
    }

    #[test]
    fn load_reports_explicit_config_read_errors_with_path() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let missing_config = temp.path().join("missing.toml");

        let error = Config::load(&repository_root, Some(&missing_config))
            .unwrap_err()
            .to_string();

        assert!(error.contains("failed to read"));
        assert!(error.contains("missing.toml"));
    }

    #[test]
    fn load_rejects_scope_dirs_outside_repository_root() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();

        fs::write(
            repository_root.join("docgarden.toml"),
            "skills-dir = \"../skills\"\n",
        )
        .unwrap();
        let err = Config::load(&repository_root, None).unwrap_err();
        assert!(err.to_string().contains("skills-dir must stay under"));

        fs::write(repository_root.join("docgarden.toml"), "plans-dir = \"\"\n").unwrap();
        let err = Config::load(&repository_root, None).unwrap_err();
        assert!(err.to_string().contains("plans-dir must not be empty"));
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

        assert!(config.respect_gitignore());
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

        let refs_policy = config.effective_rule_policy_for_path("docs/references/source.md");
        assert!(
            refs_policy
                .ignored_rules
                .contains(&Rule::UnresolvedLinkPath)
        );
        assert!(refs_policy.backtick_path_severity.is_some());

        let readme_policy = config.effective_rule_policy_for_path("README.md");
        assert!(readme_policy.prefer_links_for_local_paths);
        assert!(readme_policy.backtick_path_severity.is_none());

        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
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

        let policy = config.effective_rule_policy_for_path("docs/references/source.md");
        assert!(policy.ignored_rules.contains(&Rule::UnresolvedLinkPath));
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
max-tokens = 10
max-lines = 5

[[rules]]
path = "README.md"
max-lines = 20
severity = "warn"

[[rules]]
path = "docs/references/**"
disable = ["max-tokens"]
reason = "References preserve source fidelity."
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let readme = config.effective_rule_policy_for_path("README.md");
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

        let reference = config.effective_rule_policy_for_path("docs/references/source.md");
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
max-tokens = 0
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("max-tokens must be greater than zero"));
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
        assert!(error.contains("failed to parse"));

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
token-budget = 500
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
    fn rule_names_accept_only_canonical_kebab_spellings() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        let config_path = repository_root.join("docgarden.toml");

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "**/*.md"
disable = ["unresolved-link-path", "max-tokens"]

[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path", "prefer-links-for-local-paths"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert!(
            docs_policy
                .ignored_rules
                .contains(&Rule::UnresolvedLinkPath)
        );
        assert!(docs_policy.backtick_path_severity.is_some());
        assert!(docs_policy.prefer_links_for_local_paths);
        assert_eq!(docs_policy.max_tokens, None);

        fs::write(
            &config_path,
            r#"
[[rules]]
path = "**/*.md"
disable = ["unresolved_link_path"]
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to parse"));
    }

    #[test]
    fn rule_names_cover_diagnostic_only_identifiers() {
        assert_eq!(
            Rule::from_str("frontmatter-field-missing").unwrap(),
            Rule::FrontmatterFieldMissing
        );
        assert_eq!(
            Rule::from_str("frontmatter-malformed").unwrap(),
            Rule::FrontmatterMalformed
        );
        assert_eq!(
            Rule::from_str("frontmatter-field-max-chars").unwrap(),
            Rule::FrontmatterFieldMaxChars
        );
        assert!(Rule::from_str("frontmatter_malformed").is_err());
        assert!(Rule::from_str("frontmatter_field_max_chars").is_err());
        assert_eq!(
            Rule::FrontmatterFieldMaxChars.as_str(),
            "frontmatter-field-max-chars"
        );
        assert_eq!(Rule::MaxTokens.as_str(), "max-tokens");
        assert_eq!(Rule::MaxLines.as_str(), "max-lines");
    }

    #[test]
    fn enable_rejects_always_on_rules() {
        let temp = TempDir::new().unwrap();
        let repository_root = temp.path().join("repo");
        fs::create_dir_all(&repository_root).unwrap();
        fs::write(
            repository_root.join("docgarden.toml"),
            r#"
[[rules]]
path = "**/*.md"
enable = ["unresolved-link-path"]
"#,
        )
        .unwrap();

        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported rule `unresolved-link-path` in `enable`"));
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
max-chars = 1024

[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let guide_policy = config.frontmatter_policy_for_path("docs/guide.md");
        assert_eq!(guide_policy.required, vec!["description".to_string()]);
        assert_eq!(guide_policy.field_max_chars.get("description"), Some(&1024));

        let agents_policy = config.frontmatter_policy_for_path("AGENTS.md");
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
max-chars = 1024

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
        let policy = config.frontmatter_policy_for_path("docs/guide.md");
        assert_eq!(policy.required, vec!["description".to_string()]);
        assert_eq!(policy.field_max_chars.get("description"), Some(&1024));

        // AGENTS.md: second rule excludes it -> only gets max_chars, not required
        let agents_policy = config.frontmatter_policy_for_path("AGENTS.md");
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
max-chars = 0
"#,
        )
        .unwrap();
        let error = Config::load(&repository_root, None)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("max-chars must be greater than zero"),
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
            "[[rules]]\npath = \"**/*.md\"\n\n[rules.frontmatter.fields.description]\nmax-chars = 512\n\n[rules.frontmatter.fields.description]\nmax-chars = 1024\n",
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

        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert_eq!(docs_policy.backtick_path_severity, Some(Severity::Warning));

        let readme_policy = config.effective_rule_policy_for_path("README.md");
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
        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert_eq!(
            docs_policy.backtick_path_severity,
            Some(Severity::Warning),
            "later enable should override earlier disable"
        );

        // README.md matches only the first two: enabled then disabled
        let readme_policy = config.effective_rule_policy_for_path("README.md");
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

        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert!(
            docs_policy.prefer_links_for_local_paths,
            "later enable should override earlier disable"
        );

        let readme_policy = config.effective_rule_policy_for_path("README.md");
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
max-tokens = 500

[[rules]]
path = "**/*.md"
disable = ["max-tokens"]

[[rules]]
path = "docs/**"
max-tokens = 200
severity = "warn"
"#,
        )
        .unwrap();

        let config = Config::load(&repository_root, None).unwrap();

        let docs_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert_eq!(
            docs_policy.max_tokens,
            Some(BudgetLimit {
                limit: 200,
                severity: RuleSeverity::Warn,
            }),
            "later max_tokens entry should override earlier disable"
        );

        let readme_policy = config.effective_rule_policy_for_path("README.md");
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

        let internal_policy = config.effective_rule_policy_for_path("docs/internal/spec.md");
        assert!(
            internal_policy.backtick_path_severity.is_some(),
            "narrow enable should override broad disable for matching path"
        );

        let other_policy = config.effective_rule_policy_for_path("docs/guide.md");
        assert!(
            other_policy.backtick_path_severity.is_none(),
            "broad disable without matching narrow enable should leave rule off"
        );
    }
}
