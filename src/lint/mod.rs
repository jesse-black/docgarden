use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

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

#[derive(Debug, Eq, PartialEq)]
struct Edit {
    start_offset: usize,
    end_offset: usize,
    replacement: String,
}

struct Finding<'a> {
    payload: DiagnosticPayload<'a>,
    edit: Option<Edit>,
}

#[derive(Clone, Copy)]
struct FilePolicy {
    local_reference_style: LocalReferenceStyle,
    report_ambiguous_inline_code: bool,
}

struct WalkState<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    ignored_rules: &'a std::collections::BTreeSet<String>,
    mode: Mode,
    edits: &'a mut Vec<Edit>,
}

pub fn lint_file(config: &Config, path: &Path, mode: Mode) -> Result<LintResult> {
    let relative_path = relative_path(&config.repository_root, path)?;
    let ignored_rules = ignored_rules_for_path(
        &config.repository_root,
        &config.per_file_ignores,
        &relative_path,
    )?;
    let policy = FilePolicy {
        local_reference_style: config.local_reference_style_for_path(&relative_path)?,
        report_ambiguous_inline_code: config
            .report_ambiguous_inline_code_for_path(&relative_path)?,
    };
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let tree = to_mdast(&source, &ParseOptions::gfm())
        .map_err(|error| anyhow!("failed to parse {}: {}", path.display(), error))?;
    let mut diagnostics = Vec::new();
    let mut edits = Vec::new();

    let mut state = WalkState {
        diagnostics: &mut diagnostics,
        ignored_rules: &ignored_rules,
        mode,
        edits: &mut edits,
    };
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
    match node {
        Node::InlineCode(_) => {
            lint_inline_code_node(config, policy, file, node, state)?;
        }
        Node::Link(_) => {
            lint_link_node(config, policy, file, node, state)?;
            return Ok(());
        }
        _ => {}
    }

    if let Some(children) = children_mut(node) {
        for child in children {
            walk_node(config, policy, file, child, state)?;
        }
    }

    Ok(())
}

fn lint_inline_code_node(
    config: &Config,
    policy: FilePolicy,
    file: &str,
    node: &Node,
    state: &mut WalkState<'_>,
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
            if !exists && candidate.is_directory_like {
                return Ok(());
            }
            if !exists {
                emit_finding(
                    state.diagnostics,
                    state.ignored_rules,
                    state.mode,
                    state.edits,
                    Finding {
                        payload: DiagnosticPayload {
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
                        edit: None,
                    },
                );
                return Ok(());
            }
            if policy.local_reference_style == LocalReferenceStyle::Links {
                let link_text = render_link_destination(file, &candidate, &resolved, &exists_path);
                emit_finding(
                    state.diagnostics,
                    state.ignored_rules,
                    state.mode,
                    state.edits,
                    Finding {
                        payload: DiagnosticPayload {
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
                        edit: edit_from_position(
                            inline.position.as_ref(),
                            format!("[{}]({link_text})", candidate.display_text),
                        ),
                    },
                );
            }
        }
    } else if policy.report_ambiguous_inline_code && looks_path_adjacent(value) {
        emit_finding(
            state.diagnostics,
            state.ignored_rules,
            state.mode,
            state.edits,
            Finding {
                payload: DiagnosticPayload {
                    file,
                    position: inline.position.as_ref(),
                    rule: "ambiguous-inline-code",
                    message: format!(
                        "Inline code `{value}` looks path-adjacent but is not a clear repository-local path."
                    ),
                    fixable: false,
                    severity: Severity::Warning,
                },
                edit: None,
            },
        );
    }
    Ok(())
}

fn lint_link_node(
    config: &Config,
    policy: FilePolicy,
    file: &str,
    node: &Node,
    state: &mut WalkState<'_>,
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
        if !exists && candidate.is_directory_like {
            return Ok(());
        }
        if !exists {
            emit_finding(
                state.diagnostics,
                state.ignored_rules,
                state.mode,
                state.edits,
                Finding {
                    payload: DiagnosticPayload {
                        file,
                        position: link.position.as_ref(),
                        rule: "unresolved-local-path",
                        message: format!(
                            "Local repository link `[{}]({})` does not resolve within the repository.",
                            label_text(&link.children),
                            candidate.display_text
                        ),
                        fixable: false,
                        severity: Severity::Error,
                    },
                    edit: None,
                },
            );
            return Ok(());
        }
        if policy.local_reference_style == LocalReferenceStyle::Backticks
            && label_equivalent(
                &link.children,
                &candidate.display_text,
                &resolved.repo_relative_path,
            )
        {
            let inline_text = render_repo_relative(&resolved, &exists_path);
            emit_finding(
                state.diagnostics,
                state.ignored_rules,
                state.mode,
                state.edits,
                Finding {
                    payload: DiagnosticPayload {
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
                    edit: edit_from_position(link.position.as_ref(), format!("`{inline_text}`")),
                },
            );
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

fn edit_from_position(
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
    sorted.sort_by(|left, right| right.start_offset.cmp(&left.start_offset));
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
            local_reference_style_overrides: Vec::new(),
            local_reference_style: LocalReferenceStyle::Backticks,
            known_extensions: default_extensions(),
            special_filenames: default_special_filenames(),
            config_path: None,
            config_was_explicit: false,
            ambiguous_inline_code_patterns: Vec::new(),
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
