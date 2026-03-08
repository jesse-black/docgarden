use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};

use crate::config::Config;
use crate::diagnostics::{Diagnostic, Severity};
use crate::discover::{discover_markdown_files, discover_markdown_files_for_targets};
use crate::lint::{Mode, lint_file, summarize};

#[derive(Parser, Debug)]
#[command(name = "dglint")]
#[command(about = "Doc Gardening Linter")]
pub struct Args {
    #[arg(default_value = ".", num_args = 0..)]
    targets: Vec<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    fix: bool,
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    let mode = if args.fix { Mode::Fix } else { Mode::Check };
    execute(args.targets, args.config, mode, args.json, args.color)
}

fn execute(
    targets: Vec<PathBuf>,
    config_path: Option<PathBuf>,
    mode: Mode,
    json: bool,
    color: ColorChoice,
) -> Result<()> {
    let invocation_targets = if targets.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        targets
    };
    let resolved_targets = canonicalize_targets(&invocation_targets)?;
    let repository_root = infer_repository_root(&resolved_targets, config_path.as_deref())?;
    let config = Config::load(&repository_root, config_path.as_deref())?;
    let files = if resolved_targets.len() == 1 && resolved_targets[0].is_dir() {
        discover_markdown_files(&config)?
    } else {
        discover_markdown_files_for_targets(&config, &resolved_targets)?
    };
    let mut diagnostics = Vec::new();

    for path in files {
        let result = lint_file(&config, &path, mode)?;
        diagnostics.extend(result.diagnostics);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    } else {
        print_diagnostics(&diagnostics, color);
        if mode == Mode::Check {
            print_fix_hint(&config, &repository_root, &invocation_targets, &diagnostics);
        }
    }

    let has_errors = diagnostics
        .iter()
        .any(|value| matches!(value.severity, Severity::Error));
    let has_non_fixable_errors = diagnostics
        .iter()
        .any(|value| matches!(value.severity, Severity::Error) && !value.fixable);
    if mode == Mode::Check && has_errors {
        bail!("violations found");
    }
    if mode == Mode::Fix && has_non_fixable_errors {
        bail!("violations found");
    }
    Ok(())
}

fn print_diagnostics(diagnostics: &[Diagnostic], color: ColorChoice) {
    let colorize = match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stdout().is_terminal(),
    };
    for diagnostic in diagnostics {
        let severity = match (diagnostic.severity.clone(), colorize) {
            (Severity::Error, true) => "\u{1b}[31merror\u{1b}[0m",
            (Severity::Warning, true) => "\u{1b}[33mwarning\u{1b}[0m",
            (Severity::Error, false) => "error",
            (Severity::Warning, false) => "warning",
        };
        if diagnostic.fixable {
            println!(
                "{}:{}:{}  {}  {}  fixable",
                diagnostic.file, diagnostic.line, diagnostic.column, severity, diagnostic.rule
            );
        } else {
            println!(
                "{}:{}:{}  {}  {}",
                diagnostic.file, diagnostic.line, diagnostic.column, severity, diagnostic.rule
            );
        }
        println!("{}", diagnostic.message);
    }
}

fn print_fix_hint(
    config: &Config,
    repository_root: &Path,
    targets: &[PathBuf],
    diagnostics: &[Diagnostic],
) {
    let summary = summarize(diagnostics);
    if summary.fixable_count == 0 {
        return;
    }
    let config_suffix = config
        .config_path
        .as_ref()
        .filter(|_| config.config_was_explicit)
        .map(|path| format!(" --config {}", display_relative(repository_root, path)))
        .unwrap_or_default();
    println!();
    println!(
        "{} fixable violation{} found.",
        summary.fixable_count,
        if summary.fixable_count == 1 { "" } else { "s" }
    );
    println!(
        "Fixable rules in this run: {}",
        summary
            .fixable_rules
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "Run `dglint {} --fix{config_suffix}` to apply fixes.",
        render_targets(targets)
    );
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn render_targets(targets: &[PathBuf]) -> String {
    targets
        .iter()
        .map(|target| shell_escape(target.as_os_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape(value: &std::ffi::OsStr) -> String {
    let rendered = value.to_string_lossy();
    if rendered
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '_' | '-'))
    {
        rendered.into_owned()
    } else {
        format!("'{}'", rendered.replace('\'', "'\\''"))
    }
}

fn canonicalize_targets(targets: &[PathBuf]) -> Result<Vec<PathBuf>> {
    targets
        .iter()
        .map(|target| {
            target
                .canonicalize()
                .with_context(|| format!("failed to canonicalize {}", target.display()))
        })
        .collect()
}

fn infer_repository_root(targets: &[PathBuf], explicit_config: Option<&Path>) -> Result<PathBuf> {
    if let Some(config_path) = explicit_config {
        let config_path = config_path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", config_path.display()))?;
        return Ok(config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/")));
    }

    let start = common_ancestor(
        &targets
            .iter()
            .map(|target| {
                if target.is_dir() {
                    target.clone()
                } else {
                    target.parent().unwrap_or(target.as_path()).to_path_buf()
                }
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or(std::env::current_dir()?);

    if let Some(root) = find_config_root(&start) {
        return Ok(root);
    }

    if let Some(root) = find_git_root(&start) {
        return Ok(root);
    }

    std::env::current_dir().context("failed to read current working directory")
}

fn find_config_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join("dglint.toml").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut components: Vec<OsString> = paths
        .first()?
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();

    for path in &paths[1..] {
        let other: Vec<OsString> = path
            .components()
            .map(|component| component.as_os_str().to_os_string())
            .collect();
        let shared = components
            .iter()
            .zip(other.iter())
            .take_while(|(left, right)| left == right)
            .count();
        components.truncate(shared);
    }

    if components.is_empty() {
        return None;
    }

    let mut ancestor = PathBuf::new();
    for component in components {
        ancestor.push(component);
    }
    Some(ancestor)
}
