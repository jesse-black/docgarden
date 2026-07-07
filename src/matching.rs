use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::analyzer::{analyze_surface_spans, is_separator, normalize_text};
use crate::cli::{ColorChoice, colorize_stdout};
use crate::config::Config;
use crate::diagnostics::PatternMatcher;
use crate::discover::{DirectoryDepth, discover_markdown_files_for_targets};
use crate::documents::{escape_row_field, load_document_metadata_for_paths};
use crate::paths::repository_relative_path;
use crate::root::{RootMarker, infer_repository_root};
use crate::scopes::{Scope, discover_scope_files};
use crate::score::{Candidate, CombinedFieldStats, Field};

struct MatchResult {
    repo_relative_path: String,
    name: String,
    description: Option<String>,
    score: f32,
    matched_terms: u32,
    first_field_hit: Option<Field>,
}

/// Output mode for `docgarden match`.  Exactly one variant is active per
/// invocation; the CLI lowers `--path-only` / `--explain` into this enum
/// before calling `execute_match` so the matcher never reasons about
/// contradictory flag combinations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MatchOutputMode {
    Default,
    PathOnly,
    Explain,
}

/// Whether ANSI styling is applied to rendered output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorRendering {
    Plain,
    Ansi,
}

pub(crate) struct MatchOptions {
    pub(crate) raw_query: Vec<String>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) no_gitignore: bool,
    pub(crate) color: ColorChoice,
    pub(crate) limit: usize,
    pub(crate) output_mode: MatchOutputMode,
    pub(crate) scope: Option<Scope>,
}

pub(crate) fn execute_match(options: MatchOptions) -> Result<()> {
    let query_str = options.raw_query.join(" ");
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
        options.config_path.as_deref(),
        &[
            RootMarker::File("docgarden.toml"),
            RootMarker::Directory(".git"),
        ],
    )?;

    let mut config = Config::load(&repository_root, options.config_path.as_deref())?;
    if options.no_gitignore {
        config.disable_gitignore();
    }

    let files = if let Some(scope) = options.scope {
        discover_scope_files(&config, scope)?
    } else {
        let discovered = discover_markdown_files_for_targets(
            &config,
            std::slice::from_ref(&repository_root),
            DirectoryDepth::Recursive,
        )?;
        apply_match_exclude(&config, discovered)?
    };
    let documents = load_document_metadata_for_paths(&config, &files)?;

    let candidates: Vec<Candidate<'_>> = documents
        .iter()
        .map(|document| Candidate {
            name: Some(document.name.as_str()),
            path_prefix: document.path_prefix.as_str(),
            description: document.description.as_deref(),
        })
        .collect();
    let stats = CombinedFieldStats::build(&candidates);

    let results = {
        let mut r: Vec<MatchResult> = documents
            .iter()
            .zip(candidates.iter())
            .filter_map(|(document, candidate)| {
                let hit = crate::score::score(&query_terms, candidate, &stats);
                if hit.score <= 0.0 {
                    return None;
                }
                Some(MatchResult {
                    repo_relative_path: document.repo_relative_path.clone(),
                    name: document.name.clone(),
                    description: document.description.clone(),
                    score: hit.score,
                    matched_terms: hit.matched_terms,
                    first_field_hit: hit.first_field_hit,
                })
            })
            .collect();

        r.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(b.matched_terms.cmp(&a.matched_terms))
                .then(field_priority(a.first_field_hit).cmp(&field_priority(b.first_field_hit)))
                .then(a.repo_relative_path.cmp(&b.repo_relative_path))
        });

        r.truncate(options.limit);
        r
    };

    let color_rendering =
        if colorize_stdout(options.color) && options.output_mode != MatchOutputMode::PathOnly {
            ColorRendering::Ansi
        } else {
            ColorRendering::Plain
        };
    let query_term_set: HashSet<&str> = query_terms.iter().map(String::as_str).collect();

    if options.output_mode == MatchOutputMode::Explain {
        println!("score | relative | coverage | path | name | description");
    }

    let top_score = results.first().map(|result| result.score).unwrap_or(0.0);
    let query_term_count = query_terms.len() as u32;

    for r in &results {
        match options.output_mode {
            MatchOutputMode::PathOnly => println!("{}", r.repo_relative_path),
            MatchOutputMode::Explain => println!(
                "{}",
                render_explain_row(
                    r,
                    &query_term_set,
                    color_rendering,
                    top_score,
                    query_term_count
                )
            ),
            MatchOutputMode::Default => {
                println!(
                    "{}",
                    render_default_row(r, &query_term_set, color_rendering)
                )
            }
        }
    }

    Ok(())
}

fn apply_match_exclude(config: &Config, files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let patterns = config.match_exclude();
    if patterns.is_empty() {
        return Ok(files);
    }

    let matcher = PatternMatcher::new(patterns)?;
    let repo_root = config.repository_root();

    let filtered: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| {
            if let Ok(relative) = repository_relative_path(repo_root, path) {
                !matcher.is_match(&relative, false)
            } else {
                true
            }
        })
        .collect();

    Ok(filtered)
}

fn render_default_row(
    result: &MatchResult,
    query_terms: &HashSet<&str>,
    color: ColorRendering,
) -> String {
    let path = render_match_field(&result.repo_relative_path, query_terms, color);
    let name = render_match_field(&result.name, query_terms, color);
    let description = result
        .description
        .as_deref()
        .map(|description| render_match_field(description, query_terms, color))
        .unwrap_or_default();

    format!("{path} | {name} | {description}")
}

fn render_explain_row(
    result: &MatchResult,
    query_terms: &HashSet<&str>,
    color: ColorRendering,
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
        color,
        top_score,
        result.matched_terms,
        query_term_count,
    );
    let relative = format!("{}% of top", relative as u32);
    let coverage = format!("{}/{} terms", result.matched_terms, query_term_count);
    let path = render_match_field(&result.repo_relative_path, query_terms, color);
    let name = render_match_field(&result.name, query_terms, color);
    let description = result
        .description
        .as_deref()
        .map(|description| render_match_field(description, query_terms, color))
        .unwrap_or_default();

    format!("{score} | {relative} | {coverage} | {path} | {name} | {description}")
}

fn render_explain_score(
    score: f32,
    color: ColorRendering,
    top_score: f32,
    matched_terms: u32,
    query_term_count: u32,
) -> String {
    let rendered = format!("{score:.2}");
    if color == ColorRendering::Plain {
        return rendered;
    }

    let code = match explain_score_band(score, top_score, matched_terms, query_term_count) {
        ScoreBand::Low => 31,
        ScoreBand::Medium => 33,
        ScoreBand::High => 32,
    };
    format!("\x1b[1;{code}m{rendered}\x1b[0m")
}

fn render_match_field(input: &str, query_terms: &HashSet<&str>, color: ColorRendering) -> String {
    let mut rendered = String::new();
    let mut token = String::new();

    for ch in input.chars() {
        if is_separator(ch) {
            flush_render_token(&mut rendered, &mut token, query_terms, color);
            push_escaped_char(&mut rendered, ch);
        } else {
            token.push(ch);
        }
    }

    flush_render_token(&mut rendered, &mut token, query_terms, color);
    rendered
}

fn flush_render_token(
    rendered: &mut String,
    token: &mut String,
    query_terms: &HashSet<&str>,
    color: ColorRendering,
) {
    if token.is_empty() {
        return;
    }

    for span in analyze_surface_spans(token) {
        let escaped = escape_row_field(span.surface);
        if color == ColorRendering::Ansi
            && span
                .terms
                .iter()
                .any(|term| query_terms.contains(term.as_str()))
        {
            rendered.push_str("\u{1b}[1m");
            rendered.push_str(&escaped);
            rendered.push_str("\u{1b}[0m");
        } else {
            rendered.push_str(&escaped);
        }
    }
    token.clear();
}

fn push_escaped_char(rendered: &mut String, ch: char) {
    if ch == '|' {
        rendered.push_str(r"\|");
    } else if ch == '\n' || ch == '\r' {
        rendered.push(' ');
    } else {
        rendered.push(ch);
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
    use super::{ColorRendering, field_priority, render_match_field};
    use crate::score::Field;
    use std::collections::HashSet;

    #[test]
    fn field_priority_orders_path_and_none_after_named_fields() {
        assert_eq!(field_priority(Some(Field::Description)), 1);
        assert_eq!(field_priority(Some(Field::Path)), 2);
        assert_eq!(field_priority(None), 3);
    }

    #[test]
    fn render_match_field_highlights_matching_text_terms() {
        let query_terms: HashSet<&str> = HashSet::from(["review"]);
        let rendered =
            render_match_field("Review the active plan", &query_terms, ColorRendering::Ansi);
        assert!(rendered.contains("\u{1b}[1mReview\u{1b}[0m"));
        assert!(rendered.contains("active"));
    }

    #[test]
    fn render_match_field_highlights_matching_path_terms() {
        let query_terms: HashSet<&str> = HashSet::from(["score"]);
        let rendered =
            render_match_field("docs/active-scoring.md", &query_terms, ColorRendering::Ansi);
        assert!(rendered.contains("\u{1b}[1mscoring\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_highlights_surface_token_by_stem() {
        let query_terms: HashSet<&str> = HashSet::from(["plan"]);
        let rendered = render_match_field(
            "Review the active plans",
            &query_terms,
            ColorRendering::Ansi,
        );
        assert!(rendered.contains("\u{1b}[1mplans\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_does_not_highlight_stopword_surface_token() {
        let query_terms: HashSet<&str> = HashSet::from(["the"]);
        let rendered = render_match_field("the plan", &query_terms, ColorRendering::Ansi);
        assert!(!rendered.contains("\u{1b}[1mthe\u{1b}[0m"));
        assert!(rendered.contains("the"));
    }

    #[test]
    fn render_match_field_highlights_camel_case_halves() {
        let plan_terms: HashSet<&str> = HashSet::from(["plan"]);
        let plan_rendered = render_match_field("ExecPlan", &plan_terms, ColorRendering::Ansi);
        assert!(plan_rendered.contains("Exec\u{1b}[1mPlan\u{1b}[0m"));
        assert!(!plan_rendered.contains("\u{1b}[1mExec\u{1b}[0mPlan"));

        let exec_terms: HashSet<&str> = HashSet::from(["exec"]);
        let exec_rendered = render_match_field("ExecPlan", &exec_terms, ColorRendering::Ansi);
        assert!(exec_rendered.contains("\u{1b}[1mExec\u{1b}[0mPlan"));
    }

    #[test]
    fn render_match_field_highlights_possessive_base_only() {
        let query_terms: HashSet<&str> = HashSet::from(["jim"]);
        let rendered = render_match_field("Jim's notebook", &query_terms, ColorRendering::Ansi);
        assert!(rendered.contains("\u{1b}[1mJim\u{1b}[0m's"));
    }

    #[test]
    fn render_match_field_does_not_match_inside_internal_apostrophe() {
        let query_terms: HashSet<&str> = HashSet::from(["the"]);
        let rendered = render_match_field("there's", &query_terms, ColorRendering::Ansi);
        assert!(!rendered.contains("\u{1b}[1mthe\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_normalizes_newlines() {
        let query_terms = HashSet::new();
        let rendered = render_match_field(
            "First paragraph.\nSecond paragraph.",
            &query_terms,
            ColorRendering::Plain,
        );

        assert_eq!(rendered, "First paragraph. Second paragraph.");
    }

    #[test]
    fn render_match_field_highlights_compound_dictionary_parts() {
        let query_terms: HashSet<&str> = HashSet::from(["plan"]);
        let rendered = render_match_field("execplan", &query_terms, ColorRendering::Ansi);
        assert_eq!(rendered, "exec\u{1b}[1mplan\u{1b}[0m");
    }
}
