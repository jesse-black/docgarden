use anyhow::Result;
use markdown::mdast::Node;

use crate::diagnostics::Severity;
use crate::lint::references::{
    ReferenceKind, classify_inline_reference, classify_link_reference, is_external,
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
        Node::InlineCode(_) => lint_inline_code_node(context, node),
        Node::Link(_) => lint_link_node(context, node),
        _ => Ok(Vec::new()),
    }
}

fn lint_inline_code_node<'a>(
    context: &NodeRuleContext<'a>,
    node: &'a Node,
) -> Result<Vec<Finding<'a>>> {
    let Some(inline) = (match node {
        Node::InlineCode(inline) => Some(inline),
        _ => None,
    }) else {
        return Ok(Vec::new());
    };
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
            if let Some(severity) = context.policy.unresolved_backtick_path_severity {
                return Ok(vec![Finding {
                    payload: DiagnosticPayload {
                        file: context.file,
                        position: inline.position.as_ref(),
                        rule: "unresolved-backtick-path",
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
                    rule: "prefer-links-for-local-paths",
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

fn lint_link_node<'a>(context: &NodeRuleContext<'a>, node: &'a Node) -> Result<Vec<Finding<'a>>> {
    let Some(link) = (match node {
        Node::Link(link) => Some(link),
        _ => None,
    }) else {
        return Ok(Vec::new());
    };
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
            use crate::lint::references::label_text;
            return Ok(vec![Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: link.position.as_ref(),
                    rule: "unresolved-link-path",
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
