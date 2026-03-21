use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(task) = args.next() else {
        bail!("usage: cargo xtask <task>");
    };

    match task.as_str() {
        "validate" => validate(),
        _ => bail!("unknown xtask `{task}`"),
    }
}

fn validate() -> Result<()> {
    run("cargo", &["fmt", "--check"])?;
    run("cargo", &["clippy"])?;
    run(
        "cargo",
        &[
            "llvm-cov",
            "--json",
            "--output-path",
            "target/coverage.json",
            "--fail-under-regions=88",
        ],
    )?;
    run("covgate", &["check", "target/coverage.json"])?;
    run("cargo-machete", &["."])?;
    run("cargo-deny", &["check"])?;
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    eprintln!("> {} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute `{program}`"))?;

    if !status.success() {
        bail!(
            "command `{program} {}` failed with status {status}",
            args.join(" ")
        );
    }

    Ok(())
}
