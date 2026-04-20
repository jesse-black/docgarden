use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::cli::{ColorChoice, colorize_stdout};
use crate::config::Config;
use crate::discover::discover_markdown_files_for_targets;
use crate::frontmatter::{FrontmatterParseResult, YamlValue, parse_from_str};
use crate::paths::repository_relative_path;
use crate::root::{RootMarker, infer_repository_root};
use crate::score::{Candidate, CombinedFieldStats, Field, normalize_text};

struct MatchResult {
    repo_relative_path: String,
    name: String,
    description: Option<String>,
    score: f32,
    matched_terms: u32,
    first_field_hit: Option<Field>,
}

pub(crate) fn execute_match(
    raw_query: Vec<String>,
    config_path: Option<PathBuf>,
    no_gitignore: bool,
    color: ColorChoice,
    limit: Option<usize>,
    path_only: bool,
    explain: bool,
) -> Result<()> {
    let query_str = raw_query.join(" ");
    if query_str.trim().is_empty() {
        bail!("query must contain at least one non-empty word");
    }

    let query_terms = normalize_text(&query_str);
    if query_terms.is_empty() {
        bail!("query must contain at least one non-stopword term");
    }

    let cwd = std::env::current_dir()
        .context("failed to determine current working directory")?
        .canonicalize()
        .context("failed to canonicalize current working directory")?;

    let repository_root = infer_repository_root(
        &[cwd],
        config_path.as_deref(),
        &[
            RootMarker::File("docgarden.toml"),
            RootMarker::Directory(".git"),
        ],
    )?;

    let mut config = Config::load(&repository_root, config_path.as_deref())?;
    if no_gitignore {
        config.respect_gitignore = false;
    }

    let files = discover_markdown_files_for_targets(&config, &[repository_root])?;

    let mut raw: Vec<(String, String, String, Option<String>)> = Vec::new();
    for path in &files {
        let rel = repository_relative_path(&config.repository_root, path)?;
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (frontmatter_name, description) = match parse_from_str(&source) {
            FrontmatterParseResult::Valid(fm) => (
                extract_scalar(&fm, "name"),
                extract_scalar(&fm, "description"),
            ),
            _ => (None, None),
        };
        let name = normalized_match_name(&rel, frontmatter_name);
        raw.push((rel.clone(), path_prefix(&rel), name, description));
    }

    let candidates: Vec<Candidate<'_>> = raw
        .iter()
        .map(|(_path, path_prefix, name, desc)| Candidate {
            name: Some(name.as_str()),
            path_prefix,
            description: desc.as_deref(),
        })
        .collect();
    let stats = CombinedFieldStats::build(&candidates);

    let mut results: Vec<MatchResult> = raw
        .iter()
        .zip(candidates.iter())
        .filter_map(|((path, _path_prefix, name, desc), candidate)| {
            let hit = crate::score::score(&query_terms, candidate, &stats);
            if hit.score <= 0.0 {
                return None;
            }
            Some(MatchResult {
                repo_relative_path: path.clone(),
                name: name.clone(),
                description: desc.clone(),
                score: hit.score,
                matched_terms: hit.matched_terms,
                first_field_hit: hit.first_field_hit,
            })
        })
        .collect();

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(b.matched_terms.cmp(&a.matched_terms))
            .then(field_priority(a.first_field_hit).cmp(&field_priority(b.first_field_hit)))
            .then(a.repo_relative_path.cmp(&b.repo_relative_path))
    });

    if let Some(n) = limit {
        results.truncate(n);
    }

    let style_output = colorize_stdout(color) && !path_only;
    let query_term_set: HashSet<&str> = query_terms.iter().map(String::as_str).collect();

    if explain && !path_only {
        println!("score | relative | coverage | path | name | description");
    }

    let top_score = results.first().map(|result| result.score).unwrap_or(0.0);
    let query_term_count = query_terms.len() as u32;

    for r in &results {
        if path_only {
            println!("{}", r.repo_relative_path);
        } else if explain {
            println!(
                "{}",
                render_explain_row(
                    r,
                    &query_term_set,
                    style_output,
                    top_score,
                    query_term_count
                )
            );
        } else {
            println!("{}", render_default_row(r, &query_term_set, style_output));
        }
    }

    Ok(())
}

fn extract_scalar(fm: &crate::frontmatter::ParsedFrontmatter, key: &str) -> Option<String> {
    match fm.get(key)? {
        YamlValue::Scalar(s) => Some(s.clone()),
        _ => None,
    }
}

fn normalized_match_name(repo_relative_path: &str, frontmatter_name: Option<String>) -> String {
    frontmatter_name.unwrap_or_else(|| fallback_name_from_path(repo_relative_path))
}

fn fallback_name_from_path(repo_relative_path: &str) -> String {
    Path::new(repo_relative_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| repo_relative_path.to_owned())
}

fn path_prefix(repo_relative_path: &str) -> String {
    Path::new(repo_relative_path)
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or("")
        .to_owned()
}

fn escape_pipe(s: &str) -> String {
    s.replace('|', r"\|")
}

fn render_default_row(
    result: &MatchResult,
    query_terms: &HashSet<&str>,
    style_output: bool,
) -> String {
    let path = render_match_field(
        &result.repo_relative_path,
        query_terms,
        style_output,
        FieldRenderMode::Path,
    );
    let name = render_match_field(
        &result.name,
        query_terms,
        style_output,
        FieldRenderMode::Text,
    );
    let description = result
        .description
        .as_deref()
        .map(|description| {
            render_match_field(
                description,
                query_terms,
                style_output,
                FieldRenderMode::Text,
            )
        })
        .unwrap_or_default();

    format!("{path} | {name} | {description}")
}

fn render_explain_row(
    result: &MatchResult,
    query_terms: &HashSet<&str>,
    style_output: bool,
    top_score: f32,
    query_term_count: u32,
) -> String {
    let relative = if top_score > 0.0 {
        (100.0 * (result.score / top_score)).round()
    } else {
        0.0
    };
    let score = render_explain_score(
        result.score,
        style_output,
        top_score,
        result.matched_terms,
        query_term_count,
    );
    let relative = format!("{}% of top", relative as u32);
    let coverage = format!("{}/{} terms", result.matched_terms, query_term_count);
    let path = render_match_field(
        &result.repo_relative_path,
        query_terms,
        style_output,
        FieldRenderMode::Path,
    );
    let name = render_match_field(
        &result.name,
        query_terms,
        style_output,
        FieldRenderMode::Text,
    );
    let description = result
        .description
        .as_deref()
        .map(|description| {
            render_match_field(
                description,
                query_terms,
                style_output,
                FieldRenderMode::Text,
            )
        })
        .unwrap_or_default();

    format!("{score} | {relative} | {coverage} | {path} | {name} | {description}")
}

fn render_explain_score(
    score: f32,
    style_output: bool,
    top_score: f32,
    matched_terms: u32,
    query_term_count: u32,
) -> String {
    let rendered = format!("{score:.2}");
    if !style_output {
        return rendered;
    }

    let code = match explain_score_band(score, top_score, matched_terms, query_term_count) {
        ScoreBand::Low => 31,
        ScoreBand::Medium => 33,
        ScoreBand::High => 32,
    };
    format!("\u{1b}[1;{code}m{rendered}\u{1b}[0m")
}

fn render_match_field(
    input: &str,
    query_terms: &HashSet<&str>,
    style_output: bool,
    mode: FieldRenderMode,
) -> String {
    let mut rendered = String::new();
    let mut token = String::new();

    for ch in input.chars() {
        if is_separator(ch, mode) {
            flush_render_token(&mut rendered, &mut token, query_terms, style_output);
            push_escaped_char(&mut rendered, ch);
        } else {
            token.push(ch);
        }
    }

    flush_render_token(&mut rendered, &mut token, query_terms, style_output);
    rendered
}

fn flush_render_token(
    rendered: &mut String,
    token: &mut String,
    query_terms: &HashSet<&str>,
    style_output: bool,
) {
    if token.is_empty() {
        return;
    }

    let normalized = token.to_lowercase();
    let escaped = escape_pipe(token);
    if style_output && query_terms.contains(normalized.as_str()) {
        rendered.push_str("\u{1b}[1m");
        rendered.push_str(&escaped);
        rendered.push_str("\u{1b}[0m");
    } else {
        rendered.push_str(&escaped);
    }
    token.clear();
}

fn push_escaped_char(rendered: &mut String, ch: char) {
    if ch == '|' {
        rendered.push_str(r"\|");
    } else {
        rendered.push(ch);
    }
}

fn is_separator(ch: char, mode: FieldRenderMode) -> bool {
    match mode {
        FieldRenderMode::Text => ch.is_whitespace() || ch.is_ascii_punctuation(),
        FieldRenderMode::Path => {
            matches!(ch, '/' | '_' | '-' | '.') || ch.is_whitespace() || ch.is_ascii_punctuation()
        }
    }
}

fn explain_score_band(
    score: f32,
    top_score: f32,
    matched_terms: u32,
    query_term_count: u32,
) -> ScoreBand {
    let relative = if top_score > 0.0 {
        score / top_score
    } else {
        0.0
    };
    let coverage = if query_term_count > 0 {
        matched_terms as f32 / query_term_count as f32
    } else {
        0.0
    };

    if relative >= 0.75 && coverage >= 0.75 {
        ScoreBand::High
    } else if relative >= 0.35 || coverage >= 0.5 {
        ScoreBand::Medium
    } else {
        ScoreBand::Low
    }
}

enum ScoreBand {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy)]
enum FieldRenderMode {
    Text,
    Path,
}

fn field_priority(field: Option<Field>) -> u8 {
    match field {
        Some(Field::Name) => 0,
        Some(Field::Description) => 1,
        Some(Field::Path) => 2,
        None => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FieldRenderMode, field_priority, normalized_match_name, path_prefix, render_match_field,
    };
    use crate::score::Field;
    use std::collections::HashSet;

    #[test]
    fn normalized_match_name_prefers_frontmatter_value() {
        assert_eq!(
            normalized_match_name("docs/scoring-guide.md", Some("Scoring Guide".to_string())),
            "Scoring Guide"
        );
    }

    #[test]
    fn normalized_match_name_falls_back_to_file_stem() {
        assert_eq!(
            normalized_match_name("docs/no-frontmatter.md", None),
            "no-frontmatter"
        );
    }

    #[test]
    fn normalized_match_name_handles_path_without_file_stem() {
        assert_eq!(normalized_match_name("", None), "");
    }

    #[test]
    fn path_prefix_is_empty_for_repo_root_documents() {
        assert_eq!(path_prefix("root-guide.md"), "");
    }

    #[test]
    fn path_prefix_uses_directory_only() {
        assert_eq!(path_prefix("docs/skills/SKILL.md"), "docs/skills");
    }

    #[test]
    fn field_priority_orders_path_and_none_after_named_fields() {
        assert_eq!(field_priority(Some(Field::Description)), 1);
        assert_eq!(field_priority(Some(Field::Path)), 2);
        assert_eq!(field_priority(None), 3);
    }

    #[test]
    fn render_match_field_highlights_matching_text_terms() {
        let query_terms: HashSet<&str> = HashSet::from(["review"]);
        let rendered = render_match_field(
            "Review the active plan",
            &query_terms,
            true,
            FieldRenderMode::Text,
        );
        assert!(rendered.contains("\u{1b}[1mReview\u{1b}[0m"));
        assert!(rendered.contains("active"));
    }

    #[test]
    fn render_match_field_highlights_matching_path_terms() {
        let query_terms: HashSet<&str> = HashSet::from(["scoring"]);
        let rendered = render_match_field(
            "docs/active-scoring.md",
            &query_terms,
            true,
            FieldRenderMode::Path,
        );
        assert!(rendered.contains("\u{1b}[1mscoring\u{1b}[0m"));
    }
}
