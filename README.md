# dglint

[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/dglint/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/dglint/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/jesse-black/dglint)](https://github.com/jesse-black/dglint/blob/main/LICENSE)

`dglint` is the `Doc Gardening Linter`, a Rust CLI for enforcing mechanical repository-knowledge invariants in agentic engineering repositories.

Built in Rust, `dglint` is designed for repositories where agents rely on versioned in-repo documentation as working context. It runs entirely locally, parses Markdown into an AST, validates repository-local references against the working tree, and emits deterministic diagnostics and safe autofixes without requiring any model or network access.

Today, the tool focuses on repository-local path correctness and style in Markdown, with CI and recurring doc-gardening workflows as the main operating mode. The broader direction is to enforce repository-knowledge hygiene for agent-first repositories: keeping high-traffic docs small, cross-linked, current, and mechanically legible enough for progressive context loading.

See `docs/PRODUCT.md` for the product framing and `ARCHITECTURE.md` for the current code map and invariants.

## The "Mechanical-First" Philosophy

`dglint` takes a strict stance: **repository-knowledge checks should be deterministic, local, and mechanically enforceable.**

That means the tool is intentionally narrow in what it tries to do. It does not attempt natural-language review, semantic summarization, or model-backed judgment. Instead, it focuses on the part that agents and CI systems should be able to trust absolutely:

- does a repository-local reference resolve
- is it written in the configured style
- does a document satisfy structural rules that can be checked mechanically

This matters more in agentic engineering repositories than in ordinary Markdown-heavy repositories. When agents treat `AGENTS.md`, `ARCHITECTURE.md`, plans, and related docs as executable context, documentation stops being “nice to have” prose and starts functioning as part of the repository agent operating system. Broken paths, oversized entry-point docs, missing ownership, or stale metadata are not just doc quality issues; they directly reduce agent reliability and waste context.

The design consequence is straightforward: if a rule requires judgment that would normally belong to an LLM, `dglint` should not implement that judgment itself. The agent using `dglint` can do the higher-level reasoning. `dglint` should remain the mechanical enforcement layer underneath that workflow.

## Scope Today

The current implementation covers:

- repository-local path detection in Markdown inline code and links
- path resolution against the repository root
- style enforcement for local references using backticks or Markdown links
- safe autofix for deterministic style rewrites
- JSON output and human-readable output for CI and local runs

## Build

From the repository root:

    cargo build

To build and run the CLI in one step:

    cargo run -- --help

## Usage

### CLI Arguments

- `[TARGETS]...`: Repository root, directories, or explicit Markdown files to lint. Defaults to `.`
- `--config <FILE>`: Use an explicit `dglint.toml`
- `--json`: Emit machine-readable diagnostics
- `--fix`: Apply deterministic safe rewrites
- `--color <auto|always|never>`: Control colored human-readable output

### Run

Lint the current repository:

    cargo run -- .

Lint a single file or an explicit file list:

    cargo run -- docs/exec-plans/active/doc-gardening-linter.md
    cargo run -- README.md AGENTS.md

Apply safe autofixes:

    cargo run -- . --fix

Use an explicit config file:

    cargo run -- . --config dglint.toml

Enable optional warnings for path-adjacent inline backticks such as `` `crates/parser` ``:

    report-ambiguous-inline-code = true

## Repository Policy

This repository dogfoods `dglint` with the default `backticks` local-reference style. In practice that means direct local path mentions in prose should usually look like `docs/PLANS.md`, `ARCHITECTURE.md`, or `src/lint/mod.rs`.

There are two intended exceptions. Keep external destinations as normal Markdown links such as [OpenAI API docs](https://platform.openai.com/docs/). Keep local Markdown links only when the label adds real prose value instead of merely repeating the destination, for example `[execution-plan rules](docs/PLANS.md)`.

The root `dglint.toml` excludes `tests/**` from dogfooding so fixture repositories and expected-output samples do not create noise in the main repository lint pass.

### Example CI Usage

Run `dglint` as a repository-quality gate after checkout:

```yaml
- name: Lint repository knowledge
  run: cargo run -- .
```

If you install the binary in CI, the same step becomes:

```yaml
- name: Lint repository knowledge
  run: dglint .
```

## How Does `dglint` Compare?

There are adjacent tools, but the category is still early.

`AGENTS.md` is now a widely used open format for guiding coding agents, and it provides the context standard that many agent-first repositories build around. There are also newer adjacent tools that focus on scoring or linting agent instruction files and prompt surfaces.

`dglint` sits in a different spot. It is not a general agent-quality scorer and it is not just an `AGENTS.md` validator. It is aimed at the broader repository knowledge system around those entry-point files: local references, structural doc rules, metadata, cross-linking, and other deterministic invariants that support CI-enforced doc gardening in agent-first repositories.

## Test

Run the automated test suite:

    cargo test

Run the quiet form used for quick local verification:

    cargo test --quiet

Generate coverage if `cargo-llvm-cov` is installed:

    cargo llvm-cov --summary-only
