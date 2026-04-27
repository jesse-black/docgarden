use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::diagnostics::{Diagnostic, Severity};
use crate::discover::{
    DirectoryDepth, discover_markdown_files_for_targets,
    discover_markdown_files_for_targets_with_depth,
};
use crate::lint::{Mode, lint_file, summarize};
use crate::listing;
use crate::listing::ListRenderOptions;
use crate::matching;
use crate::root::{RootMarker, infer_repository_root};
use crate::scopes::{discover_scope_files, list_scope_from_switches, scope_from_switches};

const DEFAULT_MATCH_LIMIT: usize = 5;

#[derive(Parser, Debug)]
#[command(name = "docgarden")]
#[command(about = "Repository knowledge tooling for agentic engineering repositories")]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Lint repository knowledge without modifying files")]
    Lint(LintArgs),
    #[command(about = "Apply deterministic safe rewrites")]
    Fix(LintArgs),
    #[command(
        visible_alias = "m",
        about = "Rank repository documents by metadata match",
        long_about = "Rank repository Markdown documents by how well their frontmatter \
                      fields match the given query terms.\n\
                      \n\
                      Output columns (default):\n\
                      \x20 path | name | description\n\
                      \n\
                      Fields are separated by ` | `. A literal `|` in any field is \
                      escaped as `\\|`. Matching query terms are bolded when styled output \
                      is enabled. The `name` column uses frontmatter `name` when \
                      present and otherwise falls back to the filename without its \
                      extension. The `description` column is empty when the document has \
                      no frontmatter `description`."
    )]
    Match(MatchArgs),
    #[command(
        visible_alias = "ls",
        about = "List repository documents by metadata",
        long_about = "List repository Markdown documents with compact frontmatter metadata.\n\
                      \n\
                      Output columns:\n\
                      \x20 path | name | description\n\
                      \n\
                      Directory targets are shallow by default. Use -R, --recurse to \
                      descend into nested directories. Scope switches select configured \
                      document sets and cannot be combined with positional targets. \
                      Fields are separated by ` | `. A literal `|` in any field is \
                      escaped as `\\|`. The `name` column uses frontmatter `name` when \
                      present and otherwise falls back to the filename without its \
                      extension. The `description` column is empty when the document has \
                      no frontmatter `description`."
    )]
    List(ListArgs),
}

#[derive(ClapArgs, Debug)]
struct LintArgs {
    #[arg(
        default_value = ".",
        num_args = 0..,
        help = "Repository root, directories, or Markdown files to lint"
    )]
    targets: Vec<PathBuf>,
    #[arg(long, help = "Use an explicit docgarden.toml configuration file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Ignore .gitignore and related exclude files during discovery"
    )]
    no_gitignore: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        help = "Control colored human-readable output"
    )]
    color: ColorChoice,
}

#[derive(ClapArgs, Debug)]
struct MatchArgs {
    #[arg(
        required = true,
        num_args = 1..,
        help = "Query terms; joined with spaces before tokenization"
    )]
    query: Vec<String>,
    #[arg(long, help = "Use an explicit docgarden.toml configuration file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Ignore .gitignore and related exclude files during discovery"
    )]
    no_gitignore: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        help = "Control colored human-readable output"
    )]
    color: ColorChoice,
    #[arg(
        short = 'n',
        long,
        default_value_t = DEFAULT_MATCH_LIMIT,
        help = "Limit results to the top N matches"
    )]
    limit: usize,
    #[arg(
        short = 'p',
        long,
        help = "Print only repository-relative paths, one per line"
    )]
    path_only: bool,
    #[arg(long, help = "Show diagnostic data explaining each document's ranking")]
    explain: bool,
    #[arg(
        long,
        help = "Restrict matching to configured skills_dir SKILL.md files"
    )]
    skills: bool,
    #[arg(
        long,
        help = "Restrict matching to configured plans_dir Markdown files"
    )]
    plans: bool,
}

#[derive(ClapArgs, Debug)]
struct ListArgs {
    #[arg(
        num_args = 0..,
        help = "Markdown files or directories to list; defaults to the current directory"
    )]
    targets: Vec<PathBuf>,
    #[arg(long, help = "Use an explicit docgarden.toml configuration file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Ignore .gitignore and related exclude files during discovery"
    )]
    no_gitignore: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        help = "Control colored human-readable output"
    )]
    color: ColorChoice,
    #[arg(short = 'R', long, help = "Recurse into nested directory targets")]
    recurse: bool,
    #[arg(long, help = "List configured skills_dir SKILL.md files")]
    skills: bool,
    #[arg(long, help = "List configured plans_dir Markdown files")]
    plans: bool,
    #[arg(long, help = "List Markdown files under plans_dir/active")]
    active_plans: bool,
    #[arg(long, help = "List Markdown files under plans_dir/completed")]
    completed_plans: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Lint(args) => execute_lint(args, Mode::Check),
        Command::Fix(args) => execute_lint(args, Mode::Fix),
        Command::Match(args) => matching::execute_match(matching::MatchOptions {
            raw_query: args.query,
            config_path: args.config,
            no_gitignore: args.no_gitignore,
            color: args.color,
            limit: args.limit,
            path_only: args.path_only,
            explain: args.explain,
            scope: match_scope(args.skills, args.plans)?,
        }),
        Command::List(args) => execute_list(args),
    }
}

fn match_scope(skills: bool, plans: bool) -> Result<Option<crate::scopes::Scope>> {
    if skills && plans {
        bail!("match scope switches are mutually exclusive");
    }
    Ok(scope_from_switches(skills, plans))
}

fn list_scope(args: &ListArgs) -> Result<Option<crate::scopes::Scope>> {
    let selected = [
        args.skills,
        args.plans,
        args.active_plans,
        args.completed_plans,
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if selected > 1 {
        bail!("list scope switches are mutually exclusive");
    }
    Ok(list_scope_from_switches(
        args.skills,
        args.plans,
        args.active_plans,
        args.completed_plans,
    ))
}

fn execute_lint(args: LintArgs, mode: Mode) -> Result<()> {
    execute(
        args.targets,
        args.config,
        mode,
        args.no_gitignore,
        args.color,
    )
}

fn execute_list(args: ListArgs) -> Result<()> {
    let scope = list_scope(&args)?;
    if scope.is_some() && !args.targets.is_empty() {
        bail!("scope switches cannot be combined with targets");
    }

    let cwd = std::env::current_dir()
        .context("failed to determine current working directory")?
        .canonicalize()
        .context("failed to canonicalize current working directory")?;
    let invocation_targets = if scope.is_none() {
        if args.targets.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            args.targets
        }
    } else {
        Vec::new()
    };
    let resolved_targets = if invocation_targets.is_empty() {
        vec![cwd]
    } else {
        canonicalize_targets(&invocation_targets)?
    };

    let repository_root = infer_repository_root(
        &resolved_targets,
        args.config.as_deref(),
        &[
            RootMarker::File("docgarden.toml"),
            RootMarker::Directory(".git"),
        ],
    )?;
    let mut config = Config::load(&repository_root, args.config.as_deref())?;
    if args.no_gitignore {
        config.respect_gitignore = false;
    }

    let files = if let Some(scope) = scope {
        discover_scope_files(&config, scope)?
    } else {
        let depth = if args.recurse {
            DirectoryDepth::Recursive
        } else {
            DirectoryDepth::Shallow
        };
        discover_markdown_files_for_targets_with_depth(&config, &resolved_targets, depth)?
    };

    let mut render_options = ListRenderOptions::default();
    if scope.is_none() {
        for target in &resolved_targets {
            if target.is_file() {
                render_options
                    .allow_undescribed_paths
                    .insert(target.to_path_buf());
            }
        }
    }

    let _color = args.color;
    listing::print_list_rows(&config, files, &render_options)
}

fn execute(
    targets: Vec<PathBuf>,
    config_path: Option<PathBuf>,
    mode: Mode,
    no_gitignore: bool,
    color: ColorChoice,
) -> Result<()> {
    let invocation_targets = if targets.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        targets
    };
    let resolved_targets = canonicalize_targets(&invocation_targets)?;
    let repository_root = infer_repository_root(
        &resolved_targets,
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
    let files = discover_markdown_files_for_targets(&config, &resolved_targets)?;
    let mut diagnostics = Vec::new();

    for path in files {
        let result = lint_file(&config, &path, mode)?;
        diagnostics.extend(result.diagnostics);
    }

    print_diagnostics(&diagnostics, color);
    if mode == Mode::Check {
        print_fix_hint(&config, &repository_root, &invocation_targets, &diagnostics);
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
    let colorize = colorize_stdout(color);
    for diagnostic in diagnostics {
        let severity = match (diagnostic.severity, colorize) {
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

pub(crate) fn colorize_stdout(color: ColorChoice) -> bool {
    match color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stdout().is_terminal(),
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
        "Run `docgarden fix {}{config_suffix}` to apply fixes.",
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
