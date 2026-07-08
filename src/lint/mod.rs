use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

use crate::config::{Config, EffectiveRulePolicy, Rule};
use crate::diagnostics::{Diagnostic, FixSummary};
use crate::paths::repository_relative_path;

mod references;
mod reporting;
mod rules;

use reporting::{DiagnosticPayload, push_diagnostic};
pub(crate) use rules::local_paths::AnchorCache;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Check,
    Fix,
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

struct WalkState<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    ignored_rules: &'a std::collections::BTreeSet<Rule>,
    mode: Mode,
    edits: &'a mut Vec<Edit>,
}

pub(crate) fn lint_file(
    config: &Config,
    path: &Path,
    mode: Mode,
    anchor_cache: &mut AnchorCache,
) -> Result<Vec<Diagnostic>> {
    let relative_path = repository_relative_path(config.repository_root(), path)?;
    let rule_policy = config.effective_rule_policy_for_path(&relative_path);
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let tree = to_mdast(&source, &ParseOptions::gfm())
        .map_err(|error| anyhow!("failed to parse {}: {}", path.display(), error))?;
    let mut diagnostics = Vec::new();
    let mut edits = Vec::new();

    let mut state = WalkState {
        diagnostics: &mut diagnostics,
        ignored_rules: &rule_policy.ignored_rules,
        mode,
        edits: &mut edits,
    };
    let file_context = rules::file::FileRuleContext {
        policy: &rule_policy,
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
    walk_node(
        config,
        &rule_policy,
        &relative_path,
        &tree,
        anchor_cache,
        &mut state,
    )?;

    if mode == Mode::Fix && !edits.is_empty() {
        let rewritten = apply_edits(&source, &edits)?;
        if rewritten != source {
            fs::write(path, rewritten)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    Ok(diagnostics)
}

fn walk_node(
    config: &Config,
    policy: &EffectiveRulePolicy,
    file: &str,
    node: &Node,
    anchor_cache: &mut AnchorCache,
    state: &mut WalkState<'_>,
) -> Result<()> {
    let context = rules::NodeRuleContext {
        config,
        policy,
        file,
    };
    emit_findings(
        state,
        rules::local_paths::evaluate_node(&context, node, anchor_cache)?,
    );

    if matches!(node, Node::Link(_)) {
        return Ok(());
    }

    if let Some(children) = children_mut(node) {
        for child in children {
            walk_node(config, policy, file, child, anchor_cache, state)?;
        }
    }

    Ok(())
}

fn emit_finding(state: &mut WalkState<'_>, finding: Finding<'_>) {
    if state.ignored_rules.contains(&finding.payload.rule) {
        return;
    }
    let edit = finding.edit;
    push_diagnostic(state.diagnostics, finding.payload);
    if state.mode == Mode::Fix
        && let Some(edit) = edit
    {
        state.edits.push(edit);
    }
}

fn emit_findings(state: &mut WalkState<'_>, findings: Vec<Finding<'_>>) {
    for finding in findings {
        emit_finding(state, finding);
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
        Node::MdxjsEsm(_)
        | Node::Toml(_)
        | Node::Yaml(_)
        | Node::Break(_)
        | Node::InlineCode(_)
        | Node::InlineMath(_)
        | Node::MdxTextExpression(_)
        | Node::FootnoteReference(_)
        | Node::Html(_)
        | Node::Image(_)
        | Node::ImageReference(_)
        | Node::Text(_)
        | Node::Code(_)
        | Node::Math(_)
        | Node::MdxFlowExpression(_)
        | Node::ThematicBreak(_)
        | Node::Definition(_) => None,
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
    let sorted = {
        let mut s: Vec<_> = edits.iter().collect();
        s.sort_by_key(|edit| std::cmp::Reverse(edit.start_offset));
        s
    };

    for [later, earlier] in sorted.array_windows::<2>() {
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

    let rewritten = {
        let mut r = source.to_string();
        for edit in &sorted {
            r.replace_range(edit.start_offset..edit.end_offset, &edit.replacement);
        }
        r
    };

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

    use crate::config::Config;
    use crate::lint::references::ReferenceSyntax;

    use super::references::{classify_inline_reference, classify_link_reference};

    fn test_config() -> Config {
        Config::for_testing("/tmp/repo")
    }

    #[test]
    fn inline_reference_accepts_relative_and_workspace_root_paths() {
        let config = test_config();

        let relative = classify_inline_reference(&config, "./docs/guide.md").unwrap();
        assert_eq!(relative.display_text, "./docs/guide.md");
        assert_eq!(relative.syntax, ReferenceSyntax::Relative);

        let workspace_root = classify_inline_reference(&config, "/docs/guide.md").unwrap();
        assert_eq!(workspace_root.display_text, "/docs/guide.md");
        assert_eq!(workspace_root.syntax, ReferenceSyntax::WorkspaceRoot);
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
        }
    }

    #[test]
    fn link_reference_accepts_relative_and_known_repo_paths() {
        let config = test_config();

        let relative = classify_link_reference(&config, "../README.md").unwrap();
        assert_eq!(relative.display_text, "../README.md");
        assert_eq!(relative.syntax, ReferenceSyntax::Relative);

        let nested = classify_link_reference(&config, "docs/guide").unwrap();
        assert_eq!(nested.display_text, "docs/guide");
        assert_eq!(nested.syntax, ReferenceSyntax::Standard);

        let readme = classify_link_reference(&config, "README.md").unwrap();
        assert_eq!(readme.display_text, "README.md");
    }

    #[test]
    fn link_reference_rejects_empty_and_external_destinations() {
        let config = test_config();

        for value in ["", "https://example.com/docs", "mailto:hi@example.com"] {
            assert!(classify_link_reference(&config, value).is_none(), "{value}");
        }
    }

    #[test]
    fn link_reference_accepts_same_document_anchor() {
        let config = test_config();

        let anchor = classify_link_reference(&config, "#section").unwrap();

        assert_eq!(anchor.display_text, "#section");
        assert_eq!(anchor.anchor.as_deref(), Some("section"));
    }
}
