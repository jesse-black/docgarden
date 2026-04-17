use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

use crate::config::{BudgetLimit, Config};
use crate::diagnostics::{Diagnostic, FixSummary, Severity};
use crate::paths::repository_relative_path;

mod references;
mod reporting;
mod rules;

use reporting::{DiagnosticPayload, push_diagnostic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Check,
    Fix,
}

pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Edit {
    pub(crate) start_offset: usize,
    pub(crate) end_offset: usize,
    pub(crate) replacement: String,
}

pub(crate) struct Finding<'a> {
    pub(crate) payload: DiagnosticPayload<'a>,
    pub(crate) edit: Option<Edit>,
}

#[derive(Clone, Copy)]
pub(crate) struct FilePolicy {
    pub(crate) unresolved_backtick_path_severity: Option<Severity>,
    pub(crate) prefer_links_for_local_paths: bool,
    pub(crate) max_tokens: Option<BudgetLimit>,
    pub(crate) max_lines: Option<BudgetLimit>,
}

struct WalkState<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    ignored_rules: &'a std::collections::BTreeSet<String>,
    mode: Mode,
    edits: &'a mut Vec<Edit>,
}

pub fn lint_file(config: &Config, path: &Path, mode: Mode) -> Result<LintResult> {
    let relative_path = repository_relative_path(&config.repository_root, path)?;
    let rule_policy = config.effective_rule_policy_for_path(&relative_path)?;
    let ignored_rules = rule_policy.ignored_rules;
    let policy = FilePolicy {
        unresolved_backtick_path_severity: rule_policy.backtick_path_severity,
        prefer_links_for_local_paths: rule_policy.prefer_links_for_local_paths,
        max_tokens: rule_policy.max_tokens,
        max_lines: rule_policy.max_lines,
    };
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let tree = to_mdast(&source, &ParseOptions::gfm())
        .map_err(|error| anyhow!("failed to parse {}: {}", path.display(), error))?;
    let mut diagnostics = Vec::new();
    let mut edits = Vec::new();
    let ignored_rules = &ignored_rules;

    let mut state = WalkState {
        diagnostics: &mut diagnostics,
        ignored_rules,
        mode,
        edits: &mut edits,
    };
    let file_context = rules::file::FileRuleContext {
        config,
        policy,
        file: &relative_path,
        source: &source,
    };
    emit_findings(&mut state, rules::file::evaluate_file_rules(&file_context)?);
    let fm_context = rules::frontmatter::FrontmatterRuleContext {
        config,
        file: &relative_path,
        source: &source,
    };
    emit_findings(
        &mut state,
        rules::frontmatter::evaluate_frontmatter_rules(&fm_context)?,
    );
    walk_node(config, policy, &relative_path, &tree, &mut state)?;

    if mode == Mode::Fix && !edits.is_empty() {
        let rewritten = apply_edits(&source, &edits)?;
        if rewritten != source {
            fs::write(path, rewritten)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    Ok(LintResult { diagnostics })
}

fn walk_node(
    config: &Config,
    policy: FilePolicy,
    file: &str,
    node: &Node,
    state: &mut WalkState<'_>,
) -> Result<()> {
    let context = rules::NodeRuleContext {
        config,
        policy,
        file,
    };
    emit_findings(state, rules::local_paths::evaluate_node(&context, node)?);

    if matches!(node, Node::Link(_)) {
        return Ok(());
    }

    if let Some(children) = children_mut(node) {
        for child in children {
            walk_node(config, policy, file, child, state)?;
        }
    }

    Ok(())
}

fn emit_finding(
    diagnostics: &mut Vec<Diagnostic>,
    ignored_rules: &std::collections::BTreeSet<String>,
    mode: Mode,
    edits: &mut Vec<Edit>,
    finding: Finding<'_>,
) {
    if ignored_rules.contains(finding.payload.rule) {
        return;
    }
    let edit = finding.edit;
    push_diagnostic(diagnostics, finding.payload);
    if mode == Mode::Fix
        && let Some(edit) = edit
    {
        edits.push(edit);
    }
}

fn emit_findings(state: &mut WalkState<'_>, findings: Vec<Finding<'_>>) {
    for finding in findings {
        emit_finding(
            state.diagnostics,
            state.ignored_rules,
            state.mode,
            state.edits,
            finding,
        );
    }
}

fn children_mut(node: &Node) -> Option<&Vec<Node>> {
    match node {
        Node::Root(root) => Some(&root.children),
        Node::Paragraph(paragraph) => Some(&paragraph.children),
        Node::Heading(heading) => Some(&heading.children),
        Node::Blockquote(blockquote) => Some(&blockquote.children),
        Node::List(list) => Some(&list.children),
        Node::ListItem(item) => Some(&item.children),
        Node::Emphasis(emphasis) => Some(&emphasis.children),
        Node::Strong(strong) => Some(&strong.children),
        Node::Delete(delete) => Some(&delete.children),
        Node::Link(link) => Some(&link.children),
        Node::LinkReference(link) => Some(&link.children),
        Node::Table(table) => Some(&table.children),
        Node::TableRow(row) => Some(&row.children),
        Node::TableCell(cell) => Some(&cell.children),
        Node::FootnoteDefinition(definition) => Some(&definition.children),
        Node::MdxJsxFlowElement(element) => Some(&element.children),
        Node::MdxJsxTextElement(element) => Some(&element.children),
        _ => None,
    }
}

pub(crate) fn edit_from_position(
    position: Option<&markdown::unist::Position>,
    replacement: String,
) -> Option<Edit> {
    let position = position?;
    Some(Edit {
        start_offset: position.start.offset,
        end_offset: position.end.offset,
        replacement,
    })
}

fn apply_edits(source: &str, edits: &[Edit]) -> Result<String> {
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|edit| std::cmp::Reverse(edit.start_offset));
    let mut rewritten = source.to_string();

    for window in sorted.windows(2) {
        let earlier = window[1];
        let later = window[0];
        if earlier.end_offset > later.start_offset {
            return Err(anyhow!(
                "overlapping fix edits at byte offsets {}..{} and {}..{}",
                earlier.start_offset,
                earlier.end_offset,
                later.start_offset,
                later.end_offset
            ));
        }
    }

    for edit in sorted {
        rewritten.replace_range(edit.start_offset..edit.end_offset, &edit.replacement);
    }

    Ok(rewritten)
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
    use std::path::PathBuf;

    use crate::config::Config;
    use crate::defaults::{default_extensions, default_special_filenames};

    use super::references::{classify_inline_reference, contains_disallowed_backtick_syntax};

    fn test_config() -> Config {
        Config {
            repository_root: PathBuf::from("/tmp/repo"),
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
}
