---
description: "Brief guide to tools, setup commands, and environment capabilities available in this repository, Codex Cloud, and Jules; read when choosing local commands, checking tool availability, or understanding agent runtime differences."
---

d# Tools

Brief guide to tooling available to an agent running either in a Codex Cloud environment, a Jules environment, or in this repository's devcontainer.

## Repo-relevant tooling summary


## Setup commands

- Codex Cloud and Jules setup commands should both run `scripts/agent-env-setup.sh`.
- Codex Cloud and Jules maintenance commands should both run `scripts/agent-env-maintenance.sh`.

### Available in both environments

- Core CLI/build tools: `git`, `curl`, `jq`, `ripgrep`, `fd`, `zip/unzip`, `build-essential`
- Rust workflows: `rustup`, `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `cargo llvm-cov`
- Repo dogfooding: prefer `cargo run -- lint ...` and `cargo run -- fix ...` from the repository root instead of assuming an installed `docgarden` binary on `PATH`

### Installed in devcontainer and bootstrapped for Codex Cloud/Jules

- `cargo-llvm-cov` for coverage checks
- `yq` for structured edits/queries in GitHub Actions workflow YAML files and Markdown frontmatter
- `eza` for filesystem inspection
- `shellcheck` and `shfmt` for shell script quality and formatting in `scripts/` and related automation

### Devcontainer-only by default

- `gh` (GitHub CLI) is useful in local/devcontainer workflows, but Codex Cloud/Jules agents rely on native GitHub integration and do not require `gh`.
