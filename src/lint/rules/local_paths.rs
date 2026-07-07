use std::fs;

use anyhow::{Context, Result, anyhow};
use github_slugger::Slugger;
use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

use crate::config::Rule;
use crate::diagnostics::Severity;
use crate::lint::references::{
    ReferenceKind, classify_inline_reference, classify_link_reference, is_external, label_text,
    render_link_destination, resolve_candidate,
};
use crate::lint::reporting::DiagnosticPayload;
use crate::lint::{Finding, edit_from_position};

use super::NodeRuleContext;

pub(crate) fn evaluate_node<'a>(
    context: &NodeRuleContext<'a>,
    node: &'a Node,
) -> Result<Vec<Finding<'a>>> {
    if let Node::InlineCode(inline) = node {
        lint_inline_code_node(context, inline)
    } else if let Node::Link(link) = node {
        lint_link_node(context, link)
    } else {
        Ok(Vec::new())
    }
}

fn lint_inline_code_node<'a>(
    context: &NodeRuleContext<'a>,
    inline: &'a markdown::mdast::InlineCode,
) -> Result<Vec<Finding<'a>>> {
    let value = inline.value.trim();
    if let Some(candidate) = classify_inline_reference(context.config, value)
        && let Some(resolved) = resolve_candidate(context.file, &candidate, ReferenceKind::Backtick)
    {
        let exists_path = context
            .config
            .repository_root()
            .join(&resolved.repo_relative_path);
        let exists = exists_path.exists();
        if !exists && candidate.is_directory_like {
            return Ok(Vec::new());
        }
        if !exists {
            if let Some(severity) = context.policy.backtick_path_severity {
                return Ok(vec![Finding {
                    payload: DiagnosticPayload {
                        file: context.file,
                        position: inline.position.as_ref(),
                        rule: Rule::UnresolvedBacktickPath,
                        message: format!(
                            "Local repository path `{}` does not resolve within the repository.",
                            candidate.display_text
                        ),
                        fixable: false,
                        severity,
                    },
                    edit: None,
                }]);
            }
            return Ok(Vec::new());
        }
        if context.policy.prefer_links_for_local_paths {
            let link_text =
                render_link_destination(context.file, &candidate, &resolved, &exists_path);
            return Ok(vec![Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: inline.position.as_ref(),
                    rule: Rule::PreferLinksForLocalPaths,
                    message: format!(
                        "Local repository path `{}` should use Markdown link syntax.",
                        candidate.display_text
                    ),
                    fixable: true,
                    severity: Severity::Error,
                },
                edit: edit_from_position(
                    inline.position.as_ref(),
                    format!("[{}]({link_text})", candidate.display_text),
                ),
            }]);
        }
    }
    Ok(Vec::new())
}

fn lint_link_node<'a>(
    context: &NodeRuleContext<'a>,
    link: &'a markdown::mdast::Link,
) -> Result<Vec<Finding<'a>>> {
    let destination = link.url.trim();
    if is_external(destination) {
        return Ok(Vec::new());
    }
    if let Some(candidate) = classify_link_reference(context.config, destination)
        && let Some(resolved) = resolve_candidate(context.file, &candidate, ReferenceKind::Link)
    {
        let exists_path = context
            .config
            .repository_root()
            .join(&resolved.repo_relative_path);
        let exists = exists_path.exists();
        let anchor_exists = if exists {
            target_anchor_exists(&exists_path, candidate.anchor.as_deref())?
        } else {
            false
        };
        if !exists && candidate.is_directory_like {
            return Ok(Vec::new());
        }
        if !exists || !anchor_exists {
            return Ok(vec![Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: link.position.as_ref(),
                    rule: Rule::UnresolvedLinkPath,
                    message: format!(
                        "Local repository link `[{}]({})` does not resolve within the repository.",
                        label_text(&link.children),
                        candidate.display_text
                    ),
                    fixable: false,
                    severity: Severity::Error,
                },
                edit: None,
            }]);
        }
    }
    Ok(Vec::new())
}

fn target_anchor_exists(path: &std::path::Path, anchor: Option<&str>) -> Result<bool> {
    let Some(anchor) = anchor else {
        return Ok(true);
    };
    if !path.is_file() {
        return Ok(false);
    }
    if !is_markdown_target(path) {
        return Ok(true);
    }
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read linked Markdown file {}", path.display()))?;
    let tree = to_mdast(&source, &ParseOptions::gfm()).map_err(|error| {
        anyhow!(
            "failed to parse linked Markdown file {}: {}",
            path.display(),
            error
        )
    })?;
    Ok(node_contains_anchor(&tree, anchor))
}

fn is_markdown_target(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md")
                || extension.eq_ignore_ascii_case("markdown")
                || extension.eq_ignore_ascii_case("mdown")
        })
        || path
            .file_name()
            .and_then(|filename| filename.to_str())
            .is_some_and(|filename| filename.eq_ignore_ascii_case("README"))
}

fn node_contains_anchor(node: &Node, anchor: &str) -> bool {
    node_contains_anchor_with_slugger(node, anchor, &mut Slugger::default())
}

fn node_contains_anchor_with_slugger(node: &Node, anchor: &str, slugger: &mut Slugger) -> bool {
    if let Node::Heading(heading) = node {
        slugger.slug(&label_text(&heading.children)) == anchor
    } else {
        node.children().is_some_and(|children| {
            children
                .iter()
                .any(|child| node_contains_anchor_with_slugger(child, anchor, slugger))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{is_markdown_target, target_anchor_exists};
    use tempfile::TempDir;

    #[test]
    fn target_anchor_exists_rejects_anchors_on_directories() {
        let temp = TempDir::new().unwrap();

        let exists = target_anchor_exists(temp.path(), Some("testing-guidance")).unwrap();

        assert!(!exists);
    }

    #[test]
    fn is_markdown_target_accepts_markdown_extensions_and_readme() {
        assert!(is_markdown_target(std::path::Path::new("docs/guide.md")));
        assert!(is_markdown_target(std::path::Path::new(
            "docs/guide.markdown"
        )));
        assert!(is_markdown_target(std::path::Path::new("README")));
        assert!(!is_markdown_target(std::path::Path::new("openapi.yaml")));
    }
}
