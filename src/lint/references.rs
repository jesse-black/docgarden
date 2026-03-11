use std::path::{Component, Path, PathBuf};

use markdown::mdast::{InlineCode, Node, Text};

use crate::config::Config;

#[derive(Clone, Debug)]
pub(crate) struct CandidateReference {
    pub(crate) display_text: String,
    pub(crate) uses_relative_syntax: bool,
    pub(crate) uses_workspace_root_syntax: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedReference {
    pub(crate) repo_relative_path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReferenceKind {
    Backtick,
    Link,
}

pub(crate) fn classify_inline_reference(
    config: &Config,
    value: &str,
) -> Option<CandidateReference> {
    if value.is_empty() || is_external(value) || contains_disallowed_backtick_syntax(value) {
        return None;
    }
    let uses_relative_syntax = has_relative_prefix(value);
    let uses_workspace_root_syntax = has_workspace_root_prefix(value);
    if uses_workspace_root_syntax
        || uses_relative_syntax
        || value.ends_with('/')
        || value.ends_with('\\')
    {
        return Some(CandidateReference {
            display_text: value.to_string(),
            uses_relative_syntax,
            uses_workspace_root_syntax,
        });
    }
    let path = Path::new(value);
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        let extension = format!(".{extension}");
        if config.known_extensions.contains(&extension) {
            return Some(CandidateReference {
                display_text: value.to_string(),
                uses_relative_syntax: false,
                uses_workspace_root_syntax: false,
            });
        }
    }
    if config.special_filenames.contains(value) {
        return Some(CandidateReference {
            display_text: value.to_string(),
            uses_relative_syntax: false,
            uses_workspace_root_syntax: false,
        });
    }
    None
}

pub(crate) fn classify_link_reference(config: &Config, value: &str) -> Option<CandidateReference> {
    if value.is_empty() || is_external(value) {
        return None;
    }
    let uses_relative_syntax = has_relative_prefix(value);
    let uses_workspace_root_syntax = has_workspace_root_prefix(value);
    if uses_workspace_root_syntax
        || value.contains('/')
        || value.contains('\\')
        || uses_relative_syntax
    {
        return Some(CandidateReference {
            display_text: value.to_string(),
            uses_relative_syntax,
            uses_workspace_root_syntax,
        });
    }
    let path = Path::new(value);
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        let extension = format!(".{extension}");
        if config.known_extensions.contains(&extension) {
            return Some(CandidateReference {
                display_text: value.to_string(),
                uses_relative_syntax: false,
                uses_workspace_root_syntax: false,
            });
        }
    }
    if config.special_filenames.contains(value) {
        return Some(CandidateReference {
            display_text: value.to_string(),
            uses_relative_syntax: false,
            uses_workspace_root_syntax: false,
        });
    }
    None
}

pub(crate) fn resolve_candidate(
    file: &str,
    candidate: &CandidateReference,
    kind: ReferenceKind,
) -> Option<ResolvedReference> {
    let display_text = candidate.display_text.replace('\\', "/");
    let normalized_display = display_text.trim_start_matches('/');
    let base = match kind {
        ReferenceKind::Backtick => {
            if candidate.uses_workspace_root_syntax {
                Path::new("")
            } else if candidate.uses_relative_syntax {
                Path::new(file).parent().unwrap_or_else(|| Path::new(""))
            } else {
                Path::new("")
            }
        }
        ReferenceKind::Link => {
            if candidate.uses_workspace_root_syntax {
                Path::new("")
            } else {
                Path::new(file).parent().unwrap_or_else(|| Path::new(""))
            }
        }
    };
    let joined = base.join(normalized_display);
    let normalized = normalize_path(joined)?;
    Some(ResolvedReference {
        repo_relative_path: normalized,
    })
}

fn normalize_path(candidate: PathBuf) -> Option<String> {
    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::ParentDir => {
                if parts.is_empty() {
                    return None;
                }
                parts.pop();
            }
            _ => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

pub(crate) fn render_repo_relative(resolved: &ResolvedReference, exists_path: &Path) -> String {
    if exists_path.is_dir() {
        format!("{}/", resolved.repo_relative_path)
    } else {
        resolved.repo_relative_path.clone()
    }
}

pub(crate) fn render_link_destination(
    file: &str,
    candidate: &CandidateReference,
    resolved: &ResolvedReference,
    exists_path: &Path,
) -> String {
    if candidate.uses_workspace_root_syntax {
        return format!("/{}", render_repo_relative(resolved, exists_path));
    }
    let from_dir = Path::new(file).parent().unwrap_or_else(|| Path::new(""));
    render_relative_path(
        from_dir,
        Path::new(&resolved.repo_relative_path),
        exists_path.is_dir(),
    )
}

fn render_relative_path(from_dir: &Path, target: &Path, is_directory: bool) -> String {
    let from_parts: Vec<_> = from_dir
        .components()
        .filter_map(component_to_string)
        .collect();
    let target_parts: Vec<_> = target
        .components()
        .filter_map(component_to_string)
        .collect();

    let mut shared = 0;
    while shared < from_parts.len()
        && shared < target_parts.len()
        && from_parts[shared] == target_parts[shared]
    {
        shared += 1;
    }

    let mut parts = Vec::new();
    for _ in shared..from_parts.len() {
        parts.push("..".to_string());
    }
    for part in &target_parts[shared..] {
        parts.push(part.clone());
    }

    let mut rendered = if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    };
    if is_directory && !rendered.ends_with('/') {
        rendered.push('/');
    }
    rendered
}

fn component_to_string(component: Component<'_>) -> Option<String> {
    match component {
        Component::Normal(value) => Some(value.to_string_lossy().to_string()),
        _ => None,
    }
}

pub(crate) fn is_external(value: &str) -> bool {
    matches!(
        value,
        v if v.starts_with("http://")
            || v.starts_with("https://")
            || v.starts_with("mailto:")
            || v.starts_with('#')
    )
}

pub(crate) fn looks_path_adjacent(value: &str) -> bool {
    if contains_disallowed_backtick_syntax(value) {
        return false;
    }
    has_workspace_root_prefix(value)
        || has_relative_prefix(value)
        || value.chars().any(|ch| matches!(ch, '/' | '.'))
}

pub(crate) fn contains_disallowed_backtick_syntax(value: &str) -> bool {
    value.contains("//")
        || value.contains("...")
        || value.chars().any(|ch| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '*' | '?' | '[' | '{' | ':' | '(' | ')' | '<' | '>' | '"' | '\''
                )
        })
}

fn has_workspace_root_prefix(value: &str) -> bool {
    value.starts_with('/')
}

fn has_relative_prefix(value: &str) -> bool {
    value.starts_with("./") || value.starts_with("../")
}

pub(crate) fn label_equivalent(
    children: &[Node],
    destination: &str,
    repo_relative_path: &str,
) -> bool {
    let text = label_text(children);
    text == destination
        || text == repo_relative_path
        || text
            == Path::new(destination)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        || text
            == Path::new(repo_relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
}

pub(crate) fn label_text(children: &[Node]) -> String {
    let mut text = String::new();
    for child in children {
        match child {
            Node::Text(Text { value, .. }) => text.push_str(value),
            Node::InlineCode(InlineCode { value, .. }) => text.push_str(value),
            Node::Link(link) => text.push_str(&label_text(&link.children)),
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateReference, ReferenceKind, label_equivalent, render_link_destination,
        render_repo_relative, resolve_candidate,
    };
    use markdown::mdast::{InlineCode, Link, Node, Text};
    use tempfile::TempDir;

    #[test]
    fn resolve_candidate_normalizes_relative_segments_and_separators() {
        let candidate = CandidateReference {
            display_text: "./nested\\..\\real.md".to_string(),
            uses_relative_syntax: true,
            uses_workspace_root_syntax: false,
        };

        let resolved =
            resolve_candidate("docs/guide.md", &candidate, ReferenceKind::Backtick).unwrap();

        assert_eq!(resolved.repo_relative_path, "docs/real.md");
    }

    #[test]
    fn resolve_candidate_rejects_escape_above_repository_root() {
        let candidate = CandidateReference {
            display_text: "../../../secret.md".to_string(),
            uses_relative_syntax: true,
            uses_workspace_root_syntax: false,
        };

        assert!(resolve_candidate("docs/guide.md", &candidate, ReferenceKind::Backtick).is_none());
        assert!(resolve_candidate("docs/guide.md", &candidate, ReferenceKind::Link).is_none());
    }

    #[test]
    fn render_link_destination_keeps_workspace_root_and_directory_suffix() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        std::fs::create_dir(&docs).unwrap();
        let candidate = CandidateReference {
            display_text: "/docs".to_string(),
            uses_relative_syntax: false,
            uses_workspace_root_syntax: true,
        };
        let resolved = resolve_candidate("README.md", &candidate, ReferenceKind::Link).unwrap();

        let rendered = render_link_destination("README.md", &candidate, &resolved, &docs);

        assert_eq!(rendered, "/docs/");
        assert_eq!(render_repo_relative(&resolved, &docs), "docs/");
    }

    #[test]
    fn label_equivalent_accepts_filename_only_and_inline_code_text() {
        let children = vec![Node::InlineCode(InlineCode {
            value: "PLANS.md".to_string(),
            position: None,
        })];
        assert!(label_equivalent(
            &children,
            "docs/PLANS.md",
            "docs/PLANS.md"
        ));

        let nested_children = vec![Node::Link(Link {
            children: vec![Node::Text(Text {
                value: "docs/PLANS.md".to_string(),
                position: None,
            })],
            title: None,
            url: "docs/PLANS.md".to_string(),
            position: None,
        })];
        assert!(label_equivalent(
            &nested_children,
            "docs/PLANS.md",
            "docs/PLANS.md"
        ));
    }
}
