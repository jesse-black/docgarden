use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::analyzer::{analyze_token, is_separator, normalize_text, split_camel_case};
use crate::cli::{ColorChoice, colorize_stdout};
use crate::config::Config;
use crate::discover::{DirectoryDepth, discover_markdown_files_for_targets};
use crate::documents::{escape_pipe, load_document_metadata_for_paths};
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

pub(crate) struct MatchOptions {
    pub(crate) raw_query: Vec<String>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) no_gitignore: bool,
    pub(crate) color: ColorChoice,
    pub(crate) limit: usize,
    pub(crate) path_only: bool,
    pub(crate) explain: bool,
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
        config.respect_gitignore = false;
    }

    let files = if let Some(scope) = options.scope {
        discover_scope_files(&config, scope)?
    } else {
        discover_markdown_files_for_targets(&config, &[repository_root], DirectoryDepth::Recursive)?
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

    let mut results: Vec<MatchResult> = documents
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

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(b.matched_terms.cmp(&a.matched_terms))
            .then(field_priority(a.first_field_hit).cmp(&field_priority(b.first_field_hit)))
            .then(a.repo_relative_path.cmp(&b.repo_relative_path))
    });

    results.truncate(options.limit);

    let style_output = colorize_stdout(options.color) && !options.path_only;
    let query_term_set: HashSet<&str> = query_terms.iter().map(String::as_str).collect();

    if options.explain && !options.path_only {
        println!("score | relative | coverage | path | name | description");
    }

    let top_score = results.first().map(|result| result.score).unwrap_or(0.0);
    let query_term_count = query_terms.len() as u32;

    for r in &results {
        if options.path_only {
            println!("{}", r.repo_relative_path);
        } else if options.explain {
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

fn render_default_row(
    result: &MatchResult,
    query_terms: &HashSet<&str>,
    style_output: bool,
) -> String {
    let path = render_match_field(&result.repo_relative_path, query_terms, style_output);
    let name = render_match_field(&result.name, query_terms, style_output);
    let description = result
        .description
        .as_deref()
        .map(|description| render_match_field(description, query_terms, style_output))
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
    let path = render_match_field(&result.repo_relative_path, query_terms, style_output);
    let name = render_match_field(&result.name, query_terms, style_output);
    let description = result
        .description
        .as_deref()
        .map(|description| render_match_field(description, query_terms, style_output))
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

fn render_match_field(input: &str, query_terms: &HashSet<&str>, style_output: bool) -> String {
    let mut rendered = String::new();
    let mut token = String::new();

    for ch in input.chars() {
        if is_separator(ch) {
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

    for part in render_token_parts(token) {
        let escaped = escape_pipe(part);
        if style_output
            && analyze_token(part)
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

fn render_token_parts(token: &str) -> Vec<&str> {
    let (base, possessive_suffix) = split_possessive_suffix(token);
    let mut parts = split_camel_case(base);
    if let Some(suffix) = possessive_suffix {
        parts.push(suffix);
    }
    parts
}

fn split_possessive_suffix(token: &str) -> (&str, Option<&str>) {
    let lower = token.to_lowercase();
    if lower.ends_with("'s") {
        let (base, suffix) = token.split_at(token.len() - 2);
        (base, Some(suffix))
    } else if lower.ends_with("s'") {
        let (base, suffix) = token.split_at(token.len() - 1);
        (base, Some(suffix))
    } else {
        (token, None)
    }
}

fn push_escaped_char(rendered: &mut String, ch: char) {
    if ch == '|' {
        rendered.push_str(r"\|");
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
    use super::{field_priority, render_match_field};
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
        let rendered = render_match_field("Review the active plan", &query_terms, true);
        assert!(rendered.contains("\u{1b}[1mReview\u{1b}[0m"));
        assert!(rendered.contains("active"));
    }

    #[test]
    fn render_match_field_highlights_matching_path_terms() {
        let query_terms: HashSet<&str> = HashSet::from(["score"]);
        let rendered = render_match_field("docs/active-scoring.md", &query_terms, true);
        assert!(rendered.contains("\u{1b}[1mscoring\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_highlights_surface_token_by_stem() {
        let query_terms: HashSet<&str> = HashSet::from(["plan"]);
        let rendered = render_match_field("Review the active plans", &query_terms, true);
        assert!(rendered.contains("\u{1b}[1mplans\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_does_not_highlight_stopword_surface_token() {
        let query_terms: HashSet<&str> = HashSet::from(["the"]);
        let rendered = render_match_field("the plan", &query_terms, true);
        assert!(!rendered.contains("\u{1b}[1mthe\u{1b}[0m"));
        assert!(rendered.contains("the"));
    }

    #[test]
    fn render_match_field_highlights_camel_case_halves() {
        let plan_terms: HashSet<&str> = HashSet::from(["plan"]);
        let plan_rendered = render_match_field("ExecPlan", &plan_terms, true);
        assert!(plan_rendered.contains("Exec\u{1b}[1mPlan\u{1b}[0m"));
        assert!(!plan_rendered.contains("\u{1b}[1mExec\u{1b}[0mPlan"));

        let exec_terms: HashSet<&str> = HashSet::from(["exec"]);
        let exec_rendered = render_match_field("ExecPlan", &exec_terms, true);
        assert!(exec_rendered.contains("\u{1b}[1mExec\u{1b}[0mPlan"));
    }

    #[test]
    fn render_match_field_highlights_possessive_base_only() {
        let query_terms: HashSet<&str> = HashSet::from(["jim"]);
        let rendered = render_match_field("Jim's notebook", &query_terms, true);
        assert!(rendered.contains("\u{1b}[1mJim\u{1b}[0m's"));
    }

    #[test]
    fn render_match_field_does_not_match_inside_internal_apostrophe() {
        let query_terms: HashSet<&str> = HashSet::from(["the"]);
        let rendered = render_match_field("there's", &query_terms, true);
        assert!(!rendered.contains("\u{1b}[1mthe\u{1b}[0m"));
    }

    #[test]
    fn render_match_field_highlights_compound_surface_token() {
        let query_terms: HashSet<&str> = HashSet::from(["plan"]);
        let rendered = render_match_field("execplan", &query_terms, true);
        assert_eq!(rendered, "\u{1b}[1mexecplan\u{1b}[0m");
    }
}
