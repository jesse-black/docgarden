use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::cli::{ColorChoice, colorize_stdout};
use crate::config::Config;
use crate::discover::discover_markdown_files_for_targets;
use crate::frontmatter::{FrontmatterParseResult, YamlValue, parse_from_str};
use crate::paths::repository_relative_path;
use crate::root::{RootMarker, infer_repository_root};
use crate::score::{Candidate, Field, IdfTable, normalize_text};

struct MatchResult {
    repo_relative_path: String,
    name: Option<String>,
    description: Option<String>,
    score: i32,
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
) -> Result<()> {
    let query_str = raw_query.join(" ");
    let query_terms = normalize_text(&query_str);
    if query_terms.is_empty() {
        bail!("query must contain at least one non-empty word");
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

    // Owned metadata for each discovered file.
    let mut raw: Vec<(String, Option<String>, Option<String>)> = Vec::new();
    for path in &files {
        let rel = repository_relative_path(&config.repository_root, path)?;
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (name, description) = match parse_from_str(&source) {
            FrontmatterParseResult::Valid(fm) => (
                extract_scalar(&fm, "name"),
                extract_scalar(&fm, "description"),
            ),
            _ => (None, None),
        };
        raw.push((rel, name, description));
    }

    // Build IDF from the full corpus.
    let candidates: Vec<Candidate<'_>> = raw
        .iter()
        .map(|(path, name, desc)| Candidate {
            name: name.as_deref(),
            description: desc.as_deref(),
            repo_relative_path: path.as_str(),
        })
        .collect();
    let idf = IdfTable::build(&candidates);

    // Score, drop zero-score rows, sort, truncate.
    let mut results: Vec<MatchResult> = raw
        .iter()
        .zip(candidates.iter())
        .filter_map(|((path, name, desc), candidate)| {
            let hit = crate::score::score(&query_terms, candidate, &idf);
            if hit.score == 0 {
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
            .cmp(&a.score)
            .then(b.matched_terms.cmp(&a.matched_terms))
            .then(field_priority(a.first_field_hit).cmp(&field_priority(b.first_field_hit)))
            .then(a.repo_relative_path.cmp(&b.repo_relative_path))
    });

    if let Some(n) = limit {
        results.truncate(n);
    }

    let colorize = colorize_stdout(color) && !path_only;
    for r in &results {
        if path_only {
            println!("{}", r.repo_relative_path);
        } else {
            let name = r.name.as_deref().map(escape_pipe).unwrap_or_default();
            let desc = r
                .description
                .as_deref()
                .map(escape_pipe)
                .unwrap_or_default();
            println!(
                "{} | {} | {} | {}",
                render_score(r.score, colorize),
                r.repo_relative_path,
                name,
                desc
            );
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

fn escape_pipe(s: &str) -> String {
    s.replace('|', r"\|")
}

fn render_score(score: i32, colorize: bool) -> String {
    if !colorize {
        return score.to_string();
    }

    let code = match score_band(score) {
        ScoreBand::Low => 31,
        ScoreBand::Medium => 33,
        ScoreBand::High => 32,
    };
    format!("\u{1b}[{code}m{score}\u{1b}[0m")
}

fn score_band(score: i32) -> ScoreBand {
    match score {
        0..=24 => ScoreBand::Low,
        25..=59 => ScoreBand::Medium,
        _ => ScoreBand::High,
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
        Some(Field::Path) => 1,
        Some(Field::Description) => 2,
        None => 3,
    }
}
