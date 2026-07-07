use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Default)]
pub(crate) struct AnchorCache {
    anchors_by_path: HashMap<PathBuf, HashSet<String>>,
}

pub(crate) fn evaluate_node<'a>(
    context: &NodeRuleContext<'a>,
    node: &'a Node,
    anchor_cache: &mut AnchorCache,
) -> Result<Vec<Finding<'a>>> {
    if let Node::InlineCode(inline) = node {
        lint_inline_code_node(context, inline)
    } else if let Node::Link(link) = node {
        lint_link_node(context, link, anchor_cache)
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
    anchor_cache: &mut AnchorCache,
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
            target_anchor_exists(&exists_path, candidate.anchor.as_deref(), anchor_cache)?
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

fn target_anchor_exists(
    path: &Path,
    anchor: Option<&str>,
    cache: &mut AnchorCache,
) -> Result<bool> {
    let Some(anchor) = anchor else {
        return Ok(true);
    };
    if !path.is_file() {
        return Ok(false);
    }
    if !is_markdown_target(path) {
        return Ok(true);
    }
    if !cache.anchors_by_path.contains_key(path) {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read linked Markdown file {}", path.display()))?;
        let tree = to_mdast(&source, &ParseOptions::gfm()).map_err(|error| {
            anyhow!(
                "failed to parse linked Markdown file {}: {}",
                path.display(),
                error
            )
        })?;
        cache
            .anchors_by_path
            .insert(path.to_path_buf(), collect_anchors(&tree));
    }
    Ok(cache
        .anchors_by_path
        .get(path)
        .is_some_and(|anchors| anchors.contains(anchor)))
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

fn collect_anchors(node: &Node) -> HashSet<String> {
    let mut anchors = HashSet::new();
    collect_anchors_with_slugger(node, &mut Slugger::default(), &mut anchors);
    anchors
}

fn collect_anchors_with_slugger(node: &Node, slugger: &mut Slugger, anchors: &mut HashSet<String>) {
    if let Node::Heading(heading) = node {
        anchors.insert(slugger.slug(&label_text(&heading.children)));
    } else if let Some(children) = node.children() {
        for child in children {
            collect_anchors_with_slugger(child, slugger, anchors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnchorCache, is_markdown_target, target_anchor_exists};
    use tempfile::TempDir;

    #[test]
    fn target_anchor_exists_rejects_anchors_on_directories() {
        let temp = TempDir::new().unwrap();

        let exists = target_anchor_exists(
            temp.path(),
            Some("testing-guidance"),
            &mut AnchorCache::default(),
        )
        .unwrap();

        assert!(!exists);
    }

    #[test]
    fn target_anchor_exists_caches_anchors_by_target_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("guide.md");
        std::fs::write(&path, "# First\n\n# Second\n").unwrap();
        let mut cache = AnchorCache::default();

        assert!(target_anchor_exists(&path, Some("first"), &mut cache).unwrap());
        assert!(target_anchor_exists(&path, Some("second"), &mut cache).unwrap());

        assert_eq!(cache.anchors_by_path.len(), 1);
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
