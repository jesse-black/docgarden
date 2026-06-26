use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::diagnostics::{Diagnostic, Severity};
use crate::discover::{DirectoryDepth, discover_markdown_files_for_targets};
use crate::lint::{Mode, lint_file, summarize};
use crate::listing;
use crate::matching;
use crate::root::{RootMarker, infer_repository_root};
use crate::scopes::{Scope, discover_scope_files};

const DEFAULT_MATCH_LIMIT: usize = 5;

#[derive(Parser, Debug)]
#[command(name = "docgarden")]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Lint repository Markdown without changing files")]
    Lint(LintArgs),
    #[command(about = "Fix Markdown lint findings where possible")]
    Fix(LintArgs),
    #[command(
        visible_alias = "m",
        about = "Find repository documents by description",
        long_about = "Find repository Markdown documents whose descriptions match the \
                      query.\n\
                      \n\
                      Output columns (default):\n\
                      \x20 path | name | description\n\
                      \n\
                      Fields are separated by ` | `. A literal `|` in any field is \
                      escaped as `\\|`. Matching query terms are bolded when color is \
                      enabled. The `name` column uses the document name when present \
                      and otherwise falls back to the filename without its extension. \
                      The `description` column is empty when the document has no \
                      description."
    )]
    Match(MatchArgs),
    #[command(
        visible_alias = "ls",
        about = "List repository documents and descriptions",
        long_about = "List repository Markdown documents with their descriptions.\n\
                      \n\
                      Output columns:\n\
                      \x20 path | name | description\n\
                      \n\
                      Directory targets are shallow by default. Use -R, --recurse to \
                      descend into nested directories. Scope switches select configured \
                      document sets and cannot be combined with positional targets. \
                      Fields are separated by ` | `. A literal `|` in any field is \
                      escaped as `\\|`. The `name` column uses the document name when \
                      present and otherwise falls back to the filename without its \
                      extension. The `description` column is empty when the document has \
                      no description."
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
    #[arg(long, help = "Read configuration from this docgarden.toml file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Include files ignored by .gitignore and related exclude files"
    )]
    no_gitignore: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        help = "When to use color in text output"
    )]
    color: ColorChoice,
}

#[derive(ClapArgs, Debug)]
#[command(group(ArgGroup::new("match-scope").args(["skills", "plans"]).multiple(false)))]
struct MatchArgs {
    #[arg(
        required = true,
        num_args = 1..,
        help = "Query terms to match"
    )]
    query: Vec<String>,
    #[arg(long, help = "Read configuration from this docgarden.toml file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Include files ignored by .gitignore and related exclude files"
    )]
    no_gitignore: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        help = "When to use color in text output"
    )]
    color: ColorChoice,
    #[arg(
        short = 'n',
        long,
        default_value_t = DEFAULT_MATCH_LIMIT,
        help = "Show at most N matches"
    )]
    limit: usize,
    #[arg(
        short = 'p',
        long,
        help = "Print only repository-relative paths, one per line"
    )]
    path_only: bool,
    #[arg(
        long,
        conflicts_with = "path_only",
        help = "Show scoring details for each result"
    )]
    explain: bool,
    #[arg(long, help = "Search only SKILL.md files under skills-dir")]
    skills: bool,
    #[arg(long, help = "Search only Markdown files under plans-dir")]
    plans: bool,
}

#[derive(ClapArgs, Debug)]
#[command(
    group(ArgGroup::new("list-scope").args(["skills", "plans", "active_plans", "completed_plans"]).multiple(false))
)]
struct ListArgs {
    #[arg(
        num_args = 0..,
        help = "Markdown files or directories to list; defaults to the current directory"
    )]
    targets: Vec<PathBuf>,
    #[arg(long, help = "Read configuration from this docgarden.toml file")]
    config: Option<PathBuf>,
    #[arg(
        long,
        help = "Include files ignored by .gitignore and related exclude files"
    )]
    no_gitignore: bool,
    #[arg(short = 'R', long, help = "Recurse into nested directory targets")]
    recurse: bool,
    #[arg(long, help = "List SKILL.md files under skills-dir")]
    skills: bool,
    #[arg(long, help = "List Markdown files under plans-dir")]
    plans: bool,
    #[arg(long, help = "List Markdown files under plans-dir/active")]
    active_plans: bool,
    #[arg(long, help = "List Markdown files under plans-dir/completed")]
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
        Command::Match(args) => {
            let scope = args.scope();
            let output_mode = if args.path_only {
                matching::MatchOutputMode::PathOnly
            } else if args.explain {
                matching::MatchOutputMode::Explain
            } else {
                matching::MatchOutputMode::Default
            };
            matching::execute_match(matching::MatchOptions {
                raw_query: args.query,
                config_path: args.config,
                no_gitignore: args.no_gitignore,
                color: args.color,
                limit: args.limit,
                output_mode,
                scope,
            })
        }
        Command::List(args) => execute_list(args),
    }
}

impl MatchArgs {
    fn scope(&self) -> Option<Scope> {
        if self.skills {
            Some(Scope::Skills)
        } else if self.plans {
            Some(Scope::Plans)
        } else {
            None
        }
    }
}

impl ListArgs {
    fn scope(&self) -> Option<Scope> {
        if self.skills {
            Some(Scope::Skills)
        } else if self.plans {
            Some(Scope::Plans)
        } else if self.active_plans {
            Some(Scope::ActivePlans)
        } else if self.completed_plans {
            Some(Scope::CompletedPlans)
        } else {
            None
        }
    }
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
    let scope = args.scope();
    if scope.is_some() && !args.targets.is_empty() {
        bail!("scope switches cannot be combined with targets");
    }

    let cwd = std::env::current_dir()
        .context("failed to determine current working directory")?
        .canonicalize()
        .context("failed to canonicalize current working directory")?;

    let resolved_targets = if scope.is_some() || args.targets.is_empty() {
        vec![cwd]
    } else {
        canonicalize_targets(&args.targets)?
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
        config.disable_gitignore();
    }

    let files = if let Some(scope) = scope {
        discover_scope_files(&config, scope)?
    } else {
        let depth = if args.recurse {
            DirectoryDepth::Recursive
        } else {
            DirectoryDepth::Shallow
        };
        discover_markdown_files_for_targets(&config, &resolved_targets, depth)?
    };

    let explicit_file_targets: Vec<PathBuf> = if scope.is_none() {
        resolved_targets
            .iter()
            .filter(|target| target.is_file())
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    listing::print_list_rows(&config, files, &explicit_file_targets)
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
        config.disable_gitignore();
    }
    let files =
        discover_markdown_files_for_targets(&config, &resolved_targets, DirectoryDepth::Recursive)?;
    let mut diagnostics = Vec::new();

    for path in files {
        diagnostics.extend(lint_file(&config, &path, mode)?);
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
        .config_path()
        .filter(|_| config.config_was_explicit())
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
