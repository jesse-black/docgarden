use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use markdown::mdast::{InlineCode, Link, Node, Text};
use markdown::{ParseOptions, to_mdast};
use mdast_util_to_markdown::to_markdown;

use crate::config::{Config, LocalReferenceStyle};
use crate::diagnostics::{Diagnostic, FixSummary, Severity, ignored_rules_for_path};

mod references;
mod reporting;

use references::{
    ReferenceKind, classify_inline_reference, classify_link_reference, is_external,
    label_equivalent, label_text, looks_path_adjacent, render_link_destination,
    render_repo_relative, resolve_candidate,
};
use reporting::{DiagnosticPayload, push_diagnostic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Check,
    Fix,
}

pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lint_file(config: &Config, path: &Path, mode: Mode) -> Result<LintResult> {
    let relative_path = relative_path(&config.repository_root, path)?;
    let ignored_rules = ignored_rules_for_path(
        &config.repository_root,
        &config.per_file_ignores,
        &relative_path,
    )?;
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut tree = to_mdast(&source, &ParseOptions::gfm())
        .map_err(|error| anyhow!("failed to parse {}: {}", path.display(), error))?;
    let mut diagnostics = Vec::new();
    let mut edits = false;

    walk_node(
        config,
        &relative_path,
        &mut tree,
        &mut diagnostics,
        &ignored_rules,
        mode,
        &mut edits,
    )?;

    if mode == Mode::Fix && edits {
        let rewritten = to_markdown(&tree)
            .map_err(|error| anyhow!("failed to render {}: {}", path.display(), error))?;
        if rewritten != source {
            fs::write(path, rewritten)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    Ok(LintResult { diagnostics })
}

fn walk_node(
    config: &Config,
    file: &str,
    node: &mut Node,
    diagnostics: &mut Vec<Diagnostic>,
    ignored_rules: &std::collections::BTreeSet<String>,
    mode: Mode,
    changed: &mut bool,
) -> Result<()> {
    match node {
        Node::InlineCode(_) => {
            lint_inline_code_node(
                config,
                file,
                node,
                diagnostics,
                ignored_rules,
                mode,
                changed,
            )?;
        }
        Node::Link(_) => {
            lint_link_node(
                config,
                file,
                node,
                diagnostics,
                ignored_rules,
                mode,
                changed,
            )?;
        }
        _ => {}
    }

    if let Some(children) = children_mut(node) {
        for child in children {
            walk_node(
                config,
                file,
                child,
                diagnostics,
                ignored_rules,
                mode,
                changed,
            )?;
        }
    }

    Ok(())
}

fn lint_inline_code_node(
    config: &Config,
    file: &str,
    node: &mut Node,
    diagnostics: &mut Vec<Diagnostic>,
    ignored_rules: &std::collections::BTreeSet<String>,
    mode: Mode,
    changed: &mut bool,
) -> Result<()> {
    let Some(inline) = (match node {
        Node::InlineCode(inline) => Some(inline),
        _ => None,
    }) else {
        return Ok(());
    };
    let value = inline.value.trim();
    if let Some(candidate) = classify_inline_reference(config, value) {
        if let Some(resolved) = resolve_candidate(file, &candidate, ReferenceKind::Backtick) {
            let exists_path = config.repository_root.join(&resolved.repo_relative_path);
            let exists = exists_path.exists();
            if !exists {
                push_diagnostic(
                    diagnostics,
                    ignored_rules,
                    DiagnosticPayload {
                        file,
                        position: inline.position.as_ref(),
                        rule: "unresolved-local-path",
                        message: format!(
                            "Local repository path `{}` does not resolve within the repository.",
                            candidate.display_text
                        ),
                        fixable: false,
                        severity: Severity::Error,
                    },
                );
                return Ok(());
            }
            if config.local_reference_style == LocalReferenceStyle::Links {
                push_diagnostic(
                    diagnostics,
                    ignored_rules,
                    DiagnosticPayload {
                        file,
                        position: inline.position.as_ref(),
                        rule: "prefer-links-for-local-paths",
                        message: format!(
                            "Local repository path `{}` should use Markdown link syntax under the configured style policy.",
                            candidate.display_text
                        ),
                        fixable: true,
                        severity: Severity::Error,
                    },
                );
                if mode == Mode::Fix {
                    let link_text =
                        render_link_destination(file, &candidate, &resolved, &exists_path);
                    *node = Node::Link(Link {
                        children: vec![Node::Text(Text {
                            value: candidate.display_text.clone(),
                            position: inline.position.clone(),
                        })],
                        title: None,
                        url: link_text,
                        position: inline.position.clone(),
                    });
                    *changed = true;
                }
            }
        }
    } else if config.report_ambiguous_inline_code && looks_path_adjacent(value) {
        push_diagnostic(
            diagnostics,
            ignored_rules,
            DiagnosticPayload {
                file,
                position: inline.position.as_ref(),
                rule: "ambiguous-inline-code",
                message: format!(
                    "Inline code `{value}` looks path-adjacent but is not a clear repository-local path."
                ),
                fixable: false,
                severity: Severity::Warning,
            },
        );
    }
    Ok(())
}

fn lint_link_node(
    config: &Config,
    file: &str,
    node: &mut Node,
    diagnostics: &mut Vec<Diagnostic>,
    ignored_rules: &std::collections::BTreeSet<String>,
    mode: Mode,
    changed: &mut bool,
) -> Result<()> {
    let Some(link) = (match node {
        Node::Link(link) => Some(link),
        _ => None,
    }) else {
        return Ok(());
    };
    let destination = link.url.trim();
    if is_external(destination) {
        return Ok(());
    }
    if let Some(candidate) = classify_link_reference(config, destination)
        && let Some(resolved) = resolve_candidate(file, &candidate, ReferenceKind::Link)
    {
        let exists_path = config.repository_root.join(&resolved.repo_relative_path);
        let exists = exists_path.exists();
        if !exists {
            push_diagnostic(
                diagnostics,
                ignored_rules,
                DiagnosticPayload {
                    file,
                    position: link.position.as_ref(),
                    rule: "unresolved-local-path",
                    message: format!(
                        "Local repository path `{}` does not resolve within the repository.",
                        candidate.display_text
                    ),
                    fixable: false,
                    severity: Severity::Error,
                },
            );
            return Ok(());
        }
        if config.local_reference_style == LocalReferenceStyle::Backticks
            && label_equivalent(
                &link.children,
                &candidate.display_text,
                &resolved.repo_relative_path,
            )
        {
            push_diagnostic(
                diagnostics,
                ignored_rules,
                DiagnosticPayload {
                    file,
                    position: link.position.as_ref(),
                    rule: "prefer-backticks-for-local-paths",
                    message: format!(
                        "Local repository link `[{}]({})` should use backticks under the configured style policy.",
                        label_text(&link.children),
                        candidate.display_text
                    ),
                    fixable: true,
                    severity: Severity::Error,
                },
            );
            if mode == Mode::Fix {
                let inline_text = render_repo_relative(&resolved, &exists_path);
                *node = Node::InlineCode(InlineCode {
                    value: inline_text,
                    position: link.position.clone(),
                });
                *changed = true;
            }
        }
    }
    Ok(())
}

fn children_mut(node: &mut Node) -> Option<&mut Vec<Node>> {
    match node {
        Node::Root(root) => Some(&mut root.children),
        Node::Paragraph(paragraph) => Some(&mut paragraph.children),
        Node::Heading(heading) => Some(&mut heading.children),
        Node::Blockquote(blockquote) => Some(&mut blockquote.children),
        Node::List(list) => Some(&mut list.children),
        Node::ListItem(item) => Some(&mut item.children),
        Node::Emphasis(emphasis) => Some(&mut emphasis.children),
        Node::Strong(strong) => Some(&mut strong.children),
        Node::Delete(delete) => Some(&mut delete.children),
        Node::Link(link) => Some(&mut link.children),
        Node::LinkReference(link) => Some(&mut link.children),
        Node::Table(table) => Some(&mut table.children),
        Node::TableRow(row) => Some(&mut row.children),
        Node::TableCell(cell) => Some(&mut cell.children),
        Node::FootnoteDefinition(definition) => Some(&mut definition.children),
        Node::MdxJsxFlowElement(element) => Some(&mut element.children),
        Node::MdxJsxTextElement(element) => Some(&mut element.children),
        _ => None,
    }
}

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is not under {}", path.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/"))
}

pub fn summarize(diagnostics: &[Diagnostic]) -> FixSummary {
    let mut summary = FixSummary::default();
    for diagnostic in diagnostics {
        summary.record(diagnostic);
    }
    summary
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::config::{Config, LocalReferenceStyle};
    use crate::defaults::{default_extensions, default_special_filenames};

    use super::references::{
        classify_inline_reference, contains_disallowed_backtick_syntax, looks_path_adjacent,
    };

    fn test_config() -> Config {
        Config {
            repository_root: PathBuf::from("/tmp/repo"),
            include: Vec::new(),
            exclude: Vec::new(),
            per_file_ignores: BTreeMap::new(),
            local_reference_style: LocalReferenceStyle::Backticks,
            known_extensions: default_extensions(),
            special_filenames: default_special_filenames(),
            config_path: None,
            config_was_explicit: false,
            report_ambiguous_inline_code: false,
        }
    }

    #[test]
    fn inline_reference_accepts_relative_and_workspace_root_paths() {
        let config = test_config();

        let relative = classify_inline_reference(&config, "./docs/guide.md").unwrap();
        assert_eq!(relative.display_text, "./docs/guide.md");
        assert!(relative.uses_relative_syntax);
        assert!(!relative.uses_workspace_root_syntax);

        let workspace_root = classify_inline_reference(&config, "/docs/guide.md").unwrap();
        assert_eq!(workspace_root.display_text, "/docs/guide.md");
        assert!(!workspace_root.uses_relative_syntax);
        assert!(workspace_root.uses_workspace_root_syntax);
    }

    #[test]
    fn inline_reference_accepts_directory_suffixes_and_known_filenames() {
        let config = test_config();

        let directory = classify_inline_reference(&config, "docs/").unwrap();
        assert_eq!(directory.display_text, "docs/");

        let readme = classify_inline_reference(&config, "README.md").unwrap();
        assert_eq!(readme.display_text, "README.md");

        let agents = classify_inline_reference(&config, "AGENTS.md").unwrap();
        assert_eq!(agents.display_text, "AGENTS.md");
    }

    #[test]
    fn inline_reference_rejects_disallowed_backtick_syntax() {
        let config = test_config();

        for value in [
            "",
            "https://example.com/docs",
            "docs/**/*.md",
            "C:/tmp/file.txt",
            "/Users/alice/...",
            "//foo",
            "// test test_name",
            "docs/(draft).md",
        ] {
            assert!(
                classify_inline_reference(&config, value).is_none(),
                "{value}"
            );
            assert!(
                contains_disallowed_backtick_syntax(value)
                    || value.is_empty()
                    || value.starts_with("https://")
            );
        }
    }

    #[test]
    fn path_adjacent_detection_only_flags_ambiguous_patterns() {
        assert!(looks_path_adjacent("crates/base_db"));
        assert!(looks_path_adjacent("./docs/guide"));
        assert!(looks_path_adjacent("docs.guide"));

        assert!(!looks_path_adjacent("base_db"));
        assert!(!looks_path_adjacent("docs/**/*.md"));
        assert!(!looks_path_adjacent("//foo"));
        assert!(!looks_path_adjacent("/Users/alice/..."));
    }
}
