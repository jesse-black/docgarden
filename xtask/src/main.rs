use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use semver::Version;
use toml_edit::{DocumentMut, Item};

fn main() -> Result<()> {
    match Cli::parse().command {
        Task::Validate => validate(),
        Task::Clippy => clippy(),
        Task::ReleaseVersion { version } => release_version(&version),
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    Validate,
    Clippy,
    ReleaseVersion { version: String },
}

fn validate() -> Result<()> {
    Runner::new("cargo").args(["fmt", "--check"]).run()?;
    clippy()?;
    Runner::new("cargo")
        .args([
            "llvm-cov",
            "--json",
            "--output-path",
            "target/coverage.json",
        ])
        .run()?;
    Runner::new("covgate")
        .args(["check", "target/coverage.json"])
        .run()?;
    Ok(())
}

fn clippy() -> Result<()> {
    Runner::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ])
        .run()
}

fn release_version(version: &str) -> Result<()> {
    let parsed =
        Version::parse(version).with_context(|| format!("invalid SemVer version `{version}`"))?;
    let repo_root = project_root()?;
    let manifest_path = repo_root.join("Cargo.toml");
    let lockfile_path = repo_root.join("Cargo.lock");
    let mut summary = update_root_package_version(&manifest_path, &parsed)?;

    if !summary.manifest_changed {
        eprintln!("release-version: no files changed");
        return Ok(());
    }

    let lockfile_before = read_optional_file(&lockfile_path)?;
    Runner::new("cargo")
        .args(["generate-lockfile"])
        .current_dir(&repo_root)
        .run()?;
    let lockfile_after = read_optional_file(&lockfile_path)?;
    summary.lockfile_changed = lockfile_before != lockfile_after;

    let changed_files = summary.changed_files();
    eprintln!("release-version: updated {}", changed_files.join(", "));
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct ReleaseVersionSummary {
    manifest_changed: bool,
    lockfile_changed: bool,
}

impl ReleaseVersionSummary {
    fn changed_files(&self) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if self.manifest_changed {
            changed.push("Cargo.toml");
        }
        if self.lockfile_changed {
            changed.push("Cargo.lock");
        }
        changed
    }
}

fn update_root_package_version(
    manifest_path: &Path,
    version: &Version,
) -> Result<ReleaseVersionSummary> {
    let original = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read manifest: {}", manifest_path.display()))?;
    let mut document = original
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse manifest: {}", manifest_path.display()))?;

    let package = document
        .as_table_mut()
        .get_mut("package")
        .and_then(Item::as_table_mut)
        .context("Cargo.toml has no [package].version")?;
    let version_item = package
        .get_mut("version")
        .context("Cargo.toml has no [package].version")?;
    let current = version_item
        .as_str()
        .context("Cargo.toml has non-string [package].version")?;

    if current == version.to_string() {
        return Ok(ReleaseVersionSummary {
            manifest_changed: false,
            lockfile_changed: false,
        });
    }

    let decor = version_item
        .as_value()
        .context("Cargo.toml has non-string [package].version")?
        .decor()
        .clone();
    let mut updated_value = toml_edit::Value::from_str(&format!("\"{version}\""))
        .context("failed to encode SemVer version for Cargo.toml")?;
    *updated_value.decor_mut() = decor;
    *version_item = Item::Value(updated_value);

    let updated = document.to_string();
    if updated != original {
        std::fs::write(manifest_path, updated)
            .with_context(|| format!("failed to write manifest: {}", manifest_path.display()))?;
    }

    Ok(ReleaseVersionSummary {
        manifest_changed: true,
        lockfile_changed: false,
    })
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn project_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .context("xtask manifest has no parent directory")
}

#[derive(Debug)]
struct Runner<'a> {
    program: &'a str,
    args: Vec<&'a str>,
    current_dir: Option<&'a Path>,
}

impl<'a> Runner<'a> {
    fn new(program: &'a str) -> Self {
        Self {
            program,
            args: Vec::new(),
            current_dir: None,
        }
    }

    fn args(mut self, args: impl IntoIterator<Item = &'a str>) -> Self {
        self.args.extend(args);
        self
    }

    fn current_dir(mut self, dir: &'a Path) -> Self {
        self.current_dir = Some(dir);
        self
    }

    fn run(self) -> Result<()> {
        eprintln!("> {} {}", self.program, self.args.join(" "));
        let mut command = Command::new(self.program);
        command.args(&self.args);
        if let Some(dir) = self.current_dir {
            command.current_dir(dir);
        }

        let status = command
            .status()
            .with_context(|| format!("failed to execute `{}`", self.program))?;

        if !status.success() {
            bail!(
                "command `{} {}` failed with status {status}",
                self.program,
                self.args.join(" ")
            );
        }

        Ok(())
    }
}
