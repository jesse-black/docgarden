use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::frontmatter::{FrontmatterParseResult, YamlValue, parse_from_str};
use crate::paths::repository_relative_path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentMetadata {
    pub repo_relative_path: String,
    pub path_prefix: String,
    pub name: String,
    pub description: Option<String>,
}

pub(crate) fn load_document_metadata(config: &Config, path: &Path) -> Result<DocumentMetadata> {
    let repo_relative_path = repository_relative_path(config.repository_root(), path)?;
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let (frontmatter_name, description) = match parse_from_str(&source) {
        FrontmatterParseResult::Valid(fm) => (
            extract_scalar(&fm, "name"),
            extract_scalar(&fm, "description"),
        ),
        FrontmatterParseResult::None | FrontmatterParseResult::Malformed { .. } => (None, None),
    };
    let name = frontmatter_name.unwrap_or_else(|| fallback_name_from_path(&repo_relative_path));
    let path_prefix = path_prefix(&repo_relative_path);

    Ok(DocumentMetadata {
        repo_relative_path,
        path_prefix,
        name,
        description,
    })
}

pub(crate) fn load_document_metadata_for_paths(
    config: &Config,
    paths: &[PathBuf],
) -> Result<Vec<DocumentMetadata>> {
    paths
        .iter()
        .map(|path| load_document_metadata(config, path))
        .collect()
}

pub(crate) fn fallback_name_from_path(repo_relative_path: &str) -> String {
    Path::new(repo_relative_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| repo_relative_path.to_owned())
}

pub(crate) fn path_prefix(repo_relative_path: &str) -> String {
    Path::new(repo_relative_path)
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("")
        .to_owned()
}

pub(crate) fn escape_row_field(s: &str) -> String {
    s.chars()
        .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect::<String>()
        .replace('|', r"\|")
}

pub(crate) fn render_metadata_row(document: &DocumentMetadata) -> String {
    let description = document.description.as_deref().unwrap_or_default();
    format!(
        "{} | {} | {}",
        escape_row_field(&document.repo_relative_path),
        escape_row_field(&document.name),
        escape_row_field(description)
    )
}

fn extract_scalar(fm: &crate::frontmatter::ParsedFrontmatter, key: &str) -> Option<String> {
    if let YamlValue::Scalar(s) = fm.get(key)? {
        Some(s.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentMetadata, fallback_name_from_path, path_prefix, render_metadata_row};

    #[test]
    fn fallback_name_uses_file_stem() {
        assert_eq!(
            fallback_name_from_path("docs/no-frontmatter.md"),
            "no-frontmatter"
        );
    }

    #[test]
    fn fallback_name_handles_path_without_file_stem() {
        assert_eq!(fallback_name_from_path(""), "");
    }

    #[test]
    fn path_prefix_is_empty_for_repo_root_documents() {
        assert_eq!(path_prefix("root-guide.md"), "");
    }

    #[test]
    fn path_prefix_uses_directory_only() {
        assert_eq!(path_prefix("docs/skills/SKILL.md"), "docs/skills");
    }

    #[test]
    fn render_metadata_row_normalizes_newlines_in_description() {
        let row = render_metadata_row(&DocumentMetadata {
            repo_relative_path: "docs/guide.md".to_string(),
            path_prefix: "docs".to_string(),
            name: "Guide".to_string(),
            description: Some("First paragraph.\nSecond paragraph.".to_string()),
        });

        assert_eq!(
            row,
            "docs/guide.md | Guide | First paragraph. Second paragraph."
        );
    }
}
