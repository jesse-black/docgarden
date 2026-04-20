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

    let colorize = colorize_stdout(color) && !path_only;
    let separator = render_match_separator(colorize);
    for r in &results {
        if path_only {
            println!("{}", r.repo_relative_path);
        } else {
            let name = render_match_name(&escape_pipe(&r.name), colorize);
            let desc = r
                .description
                .as_deref()
                .map(escape_pipe)
                .unwrap_or_default();
            println!(
                "{}{}{}{}{}{}{}",
                render_score(r.score, colorize),
                separator,
                r.repo_relative_path,
                separator,
                name,
                separator,
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

fn render_score(score: f32, colorize: bool) -> String {
    let rendered = format!("{score:.2}");
    if !colorize {
        return rendered;
    }

    let code = match score_band(score) {
        ScoreBand::Low => 31,
        ScoreBand::Medium => 33,
        ScoreBand::High => 32,
    };
    format!("\u{1b}[1;{code}m{rendered}\u{1b}[0m")
}

fn render_match_separator(colorize: bool) -> &'static str {
    if colorize {
        "\u{1b}[1m | \u{1b}[0m"
    } else {
        " | "
    }
}

fn render_match_name(name: &str, colorize: bool) -> String {
    if colorize {
        format!("\u{1b}[1m{name}\u{1b}[0m")
    } else {
        name.to_string()
    }
}

fn score_band(score: f32) -> ScoreBand {
    if score < 1.25 {
        ScoreBand::Low
    } else if score < 2.5 {
        ScoreBand::Medium
    } else {
        ScoreBand::High
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

#[cfg(test)]
mod tests {
    use super::{field_priority, normalized_match_name, path_prefix};
    use crate::score::Field;

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
        assert_eq!(field_priority(Some(Field::Path)), 1);
        assert_eq!(field_priority(None), 3);
    }
}
