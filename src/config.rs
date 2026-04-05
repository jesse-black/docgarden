use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
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
}

#[derive(Clone, Debug)]
pub struct Config {
    pub repository_root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub per_file_ignores: BTreeMap<String, Vec<String>>,
    pub local_reference_style: LocalReferenceStyle,
    pub known_extensions: BTreeSet<String>,
    pub special_filenames: BTreeSet<String>,
    pub config_path: Option<PathBuf>,
    pub config_was_explicit: bool,
    pub report_ambiguous_inline_code: bool,
    pub respect_gitignore: bool,
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

        Ok(Self {
            repository_root,
            include,
            exclude: parsed.exclude,
            per_file_ignores: parsed.per_file_ignores,
            local_reference_style,
            known_extensions,
            special_filenames,
            config_path,
            config_was_explicit,
            report_ambiguous_inline_code: parsed.report_ambiguous_inline_code,
            respect_gitignore: parsed.respect_gitignore,
        })
    }
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
}
