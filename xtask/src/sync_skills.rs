use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

use crate::project_root;

pub(crate) fn sync_skills(check: bool) -> Result<()> {
    let repo_root = project_root()?;
    let source_root = repo_root.join("skills");
    let dest_root = repo_root.join(".agents").join("skills");
    sync_skills_at(&source_root, &dest_root, check)
}

/// Testable entry point that accepts explicit source and destination roots.
fn sync_skills_at(source_root: &Path, dest_root: &Path, check: bool) -> Result<()> {
    if !source_root.exists() {
        bail!(
            "source skills directory does not exist: {}",
            source_root.display()
        );
    }
    if !source_root.is_dir() {
        bail!(
            "source skills path is not a directory: {}",
            source_root.display()
        );
    }

    let source_skill_dirs = discover_source_skill_dirs(source_root)?;
    if source_skill_dirs.is_empty() {
        bail!("no skill directories found under {}", source_root.display());
    }

    let mut stale_paths: Vec<String> = Vec::new();

    for source_dir in &source_skill_dirs {
        let skill_name = source_dir
            .file_name()
            .context("skill directory has no name")?
            .to_str()
            .context("skill directory name is not valid UTF-8")?;

        let source_skill_path = source_dir.join("SKILL.md");
        if !source_skill_path.exists() {
            bail!("skill directory {} has no SKILL.md", source_dir.display());
        }

        let source_content = fs::read_to_string(&source_skill_path).with_context(|| {
            format!(
                "failed to read source skill: {}",
                source_skill_path.display()
            )
        })?;

        let translated = translate_skill_commands(&source_content)?;

        if check {
            let dest_skill_path = dest_root.join(skill_name).join("SKILL.md");
            if !dest_skill_path.exists() {
                stale_paths.push(format!(
                    "missing generated file: {}",
                    dest_skill_path.display()
                ));
                continue;
            }
            let dest_content = fs::read_to_string(&dest_skill_path).with_context(|| {
                format!(
                    "failed to read generated skill: {}",
                    dest_skill_path.display()
                )
            })?;
            if dest_content != translated {
                stale_paths.push(format!(
                    "stale generated file: {}",
                    dest_skill_path.display()
                ));
            }
        } else {
            let dest_dir = dest_root.join(skill_name);
            fs::create_dir_all(&dest_dir)
                .with_context(|| format!("failed to create directory: {}", dest_dir.display()))?;
            let dest_skill_path = dest_dir.join("SKILL.md");
            fs::write(&dest_skill_path, &translated).with_context(|| {
                format!(
                    "failed to write generated skill: {}",
                    dest_skill_path.display()
                )
            })?;
        }
    }

    if !stale_paths.is_empty() {
        bail!("sync-skills --check failed:\n{}", stale_paths.join("\n"));
    }

    Ok(())
}

fn discover_source_skill_dirs(source_root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in source_root
        .read_dir()
        .with_context(|| format!("failed to read directory: {}", source_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

pub(crate) fn translate_skill_commands(source: &str) -> Result<String> {
    let tree = to_mdast(source, &ParseOptions::gfm())
        .map_err(|error| anyhow!("failed to parse Markdown: {error}"))?;

    let edits = collect_translation_edits(source, &tree);
    apply_edits(source, &edits)
}

#[derive(Debug, PartialEq, Eq)]
struct Edit {
    start_offset: usize,
    end_offset: usize,
    replacement: String,
}

fn collect_translation_edits(source: &str, node: &Node) -> Vec<Edit> {
    let mut edits = Vec::new();
    collect_edits(source, node, &mut edits);
    edits
}

fn collect_edits(source: &str, node: &Node, edits: &mut Vec<Edit>) {
    match node {
        Node::InlineCode(inline) => {
            if let Some(pos) = &inline.position
                && is_translatable_command(&inline.value)
                && let Some(edit) = inline_code_translation_edit(source, pos, &inline.value)
            {
                edits.push(edit);
            }
        }
        Node::Code(code) => {
            if let Some(pos) = &code.position
                && is_shell_fenced_block(code.lang.as_deref())
            {
                edits.extend(code_block_translation_edits(source, pos, &code.value));
            }
        }
        _ => {}
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_edits(source, child, edits);
        }
    }
}

fn is_translatable_command(value: &str) -> bool {
    let rest = match value.strip_prefix("docgarden ") {
        Some(rest) => rest,
        None => return false,
    };
    // Must have a subcommand followed by at least one more token.
    let parts: Vec<&str> = rest.split_whitespace().collect();
    parts.len() >= 2
}

fn is_shell_fenced_block(lang: Option<&str>) -> bool {
    match lang {
        None => true,
        Some("sh") | Some("bash") | Some("shell") => true,
        Some(_) => false,
    }
}

fn inline_code_translation_edit(
    source: &str,
    pos: &markdown::unist::Position,
    value: &str,
) -> Option<Edit> {
    let backtick_count = count_leading_backticks(source, pos.start.offset);
    let value_start = pos.start.offset + backtick_count;
    let value_end = pos.end.offset - backtick_count;
    let actual = &source[value_start..value_end];
    if actual != value {
        return None;
    }
    let replacement = value.replacen("docgarden ", "cargo run -- ", 1);
    Some(Edit {
        start_offset: pos.start.offset,
        end_offset: pos.end.offset,
        replacement: render_inline_code(&replacement, backtick_count),
    })
}

fn code_block_translation_edits(
    source: &str,
    pos: &markdown::unist::Position,
    value: &str,
) -> Vec<Edit> {
    let block_source = &source[pos.start.offset..pos.end.offset];
    let first_newline = match block_source.find('\n') {
        Some(n) => n,
        None => return Vec::new(),
    };
    let content_start = pos.start.offset + first_newline + 1;

    let last_newline = match block_source.rfind('\n') {
        Some(n) => n,
        None => return Vec::new(),
    };
    let content_end = pos.start.offset + last_newline;

    let mut edits = Vec::new();
    let mut line_start = content_start;
    for line in value.lines() {
        if should_translate_fenced_line(line) {
            let replacement = line.replacen("docgarden ", "cargo run -- ", 1);
            edits.push(Edit {
                start_offset: line_start,
                end_offset: line_start + line.len(),
                replacement,
            });
        }
        line_start += line.len() + 1; // +1 for newline
        if line_start > content_end {
            break;
        }
    }
    edits
}

fn should_translate_fenced_line(line: &str) -> bool {
    is_translatable_command(line)
}

fn count_leading_backticks(source: &str, offset: usize) -> usize {
    source[offset..].chars().take_while(|&c| c == '`').count()
}

fn render_inline_code(value: &str, backtick_count: usize) -> String {
    let delim = "`".repeat(backtick_count.max(1));
    format!("{delim}{value}{delim}")
}

fn apply_edits(source: &str, edits: &[Edit]) -> Result<String> {
    if edits.is_empty() {
        return Ok(source.to_string());
    }

    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|edit| std::cmp::Reverse(edit.start_offset));

    for window in sorted.windows(2) {
        let later = window[0];
        let earlier = window[1];
        if earlier.end_offset > later.start_offset {
            return Err(anyhow!(
                "overlapping sync edits at byte offsets {}..{} and {}..{}",
                earlier.start_offset,
                earlier.end_offset,
                later.start_offset,
                later.end_offset
            ));
        }
    }

    let mut result = source.to_string();
    for edit in &sorted {
        result.replace_range(edit.start_offset..edit.end_offset, &edit.replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_inline_command_with_args() {
        let source = "Run `docgarden match <query>` to search.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, "Run `cargo run -- match <query>` to search.\n");
    }

    #[test]
    fn translate_inline_command_with_flags() {
        let source = "Use `docgarden match --explain <query>` for details.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(
            result,
            "Use `cargo run -- match --explain <query>` for details.\n"
        );
    }

    #[test]
    fn translate_inline_command_with_multiple_args() {
        let source = "Run `docgarden lint <changed-files> --color never`.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(
            result,
            "Run `cargo run -- lint <changed-files> --color never`.\n"
        );
    }

    #[test]
    fn preserve_conceptual_reference_without_args() {
        let source = "`docgarden match` is metadata-first discovery.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn preserve_conceptual_reference_in_description() {
        let source = "Use `docgarden match` for routing.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn translate_in_fenced_code_block() {
        let source = "```\ndocgarden match query\n```\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, "```\ncargo run -- match query\n```\n");
    }

    #[test]
    fn translate_in_fenced_shell_block() {
        let source = "```sh\ndocgarden lint .\n```\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, "```sh\ncargo run -- lint .\n```\n");
    }

    #[test]
    fn preserve_non_shell_fenced_block() {
        let source = "```yaml\ndescription: \"`docgarden match` routing\"\n```\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn preserve_double_backtick_inline() {
        let source = "Example: ``docgarden match``.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn remove_leading_docgarden_no_space() {
        let source = "`docgarden` is the tool name.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn translate_complex_inline_with_angle_brackets() {
        let source = "Run `docgarden match <query>`.\n";
        let result = translate_skill_commands(source).unwrap();
        assert_eq!(result, "Run `cargo run -- match <query>`.\n");
    }

    #[test]
    fn is_translatable_command_detection() {
        assert!(!is_translatable_command("docgarden match"));
        assert!(!is_translatable_command("docgarden lint"));
        assert!(is_translatable_command("docgarden match <query>"));
        assert!(is_translatable_command("docgarden lint . --color never"));
        assert!(is_translatable_command("docgarden match --explain <query>"));
        assert!(!is_translatable_command("something else"));
        assert!(!is_translatable_command("docgarden"));
        assert!(!is_translatable_command("docgarden "));
    }

    #[test]
    fn inline_code_translation_preserves_backtick_count() {
        use markdown::unist::{Point, Position};

        let source = "``docgarden match <query>``";
        let source_len = source.len();

        let pos = Position {
            start: Point {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: Point {
                line: 1,
                column: source_len + 1,
                offset: source_len,
            },
        };
        let value = "docgarden match <query>";
        let edit = inline_code_translation_edit(source, &pos, value).unwrap();
        assert_eq!(edit.start_offset, 0);
        assert_eq!(edit.end_offset, source_len);
        assert_eq!(edit.replacement, "``cargo run -- match <query>``");
    }

    #[test]
    fn apply_edits_rejects_overlapping_edits() {
        let source = "abcdef";
        let edits = vec![
            Edit {
                start_offset: 0,
                end_offset: 4,
                replacement: "X".into(),
            },
            Edit {
                start_offset: 2,
                end_offset: 6,
                replacement: "Y".into(),
            },
        ];
        let result = apply_edits(source, &edits);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overlapping"));
    }

    #[test]
    fn check_detects_missing_generated_file() {
        let temp = tempfile::tempdir().unwrap();
        let source_root = temp.path().join("skills");
        let dest_root = temp.path().join(".agents").join("skills");

        fs::create_dir_all(source_root.join("builder")).unwrap();
        fs::write(
            source_root.join("builder").join("SKILL.md"),
            "---\nname: builder\ndescription: Build skill.\n---\n\nRun `docgarden match <query>`.\n",
        )
        .unwrap();

        // Dest does not exist yet → check should detect missing file
        let result = sync_skills_at(&source_root, &dest_root, true);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("missing generated file"),
            "expected missing-file error, got: {msg}"
        );
    }

    #[test]
    fn check_detects_stale_generated_file() {
        let temp = tempfile::tempdir().unwrap();
        let source_root = temp.path().join("skills");
        let dest_root = temp.path().join(".agents").join("skills");

        fs::create_dir_all(source_root.join("builder")).unwrap();
        fs::create_dir_all(dest_root.join("builder")).unwrap();
        fs::write(
            source_root.join("builder").join("SKILL.md"),
            "---\nname: builder\ndescription: Build skill.\n---\n\nRun `docgarden match <query>`.\n",
        )
        .unwrap();
        fs::write(
            dest_root.join("builder").join("SKILL.md"),
            "---\nname: builder\ndescription: Old content that does not match.\n---\n\nStale.\n",
        )
        .unwrap();

        let result = sync_skills_at(&source_root, &dest_root, true);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("stale generated file"),
            "expected stale-file error, got: {msg}"
        );
    }

    #[test]
    fn sync_preserves_unrelated_destination_directories() {
        let temp = tempfile::tempdir().unwrap();
        let source_root = temp.path().join("skills");
        let dest_root = temp.path().join(".agents").join("skills");

        // Pre-populate dest with an unrelated directory
        let unrelated_dir = dest_root.join("other-agent-skill");
        fs::create_dir_all(unrelated_dir.join("sub")).unwrap();
        fs::write(
            unrelated_dir.join("SKILL.md"),
            "---\nname: other\ndescription: Unrelated skill.\n---\n# Unrelated\n",
        )
        .unwrap();

        // Source has only one skill
        fs::create_dir_all(source_root.join("builder")).unwrap();
        fs::write(
            source_root.join("builder").join("SKILL.md"),
            "---\nname: builder\ndescription: Build skill.\n---\n\nRun `docgarden match <query>`.\n",
        )
        .unwrap();

        // Run sync
        sync_skills_at(&source_root, &dest_root, false).unwrap();

        // Generated file should exist
        let generated = dest_root.join("builder").join("SKILL.md");
        assert!(
            generated.exists(),
            "generated file should exist: {}",
            generated.display()
        );

        // Unrelated directory should still exist and be untouched
        let unrelated_file = unrelated_dir.join("SKILL.md");
        assert!(
            unrelated_file.exists(),
            "unrelated file should still exist: {}",
            unrelated_file.display()
        );
        let unrelated_content = fs::read_to_string(&unrelated_file).unwrap();
        assert!(unrelated_content.contains("Unrelated skill"));
    }
}
