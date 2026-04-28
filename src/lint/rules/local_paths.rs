use anyhow::Result;
use markdown::mdast::Node;

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
    match node {
        Node::InlineCode(inline) => lint_inline_code_node(context, inline),
        Node::Link(link) => lint_link_node(context, link),
        _ => Ok(Vec::new()),
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
            .repository_root
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
            .repository_root
            .join(&resolved.repo_relative_path);
        let exists = exists_path.exists();
        if !exists && candidate.is_directory_like {
            return Ok(Vec::new());
        }
        if !exists {
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
