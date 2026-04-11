---
description: "Canonical testing workflow for `docgarden`, including validation commands, unit-versus-integration guidance, and fixture rules; read when adding features, fixing regressions, addressing review findings, or deciding how to verify repository-local behavior."
---

# Testing

This document defines the canonical testing process for `docgarden`. Treat it as the default workflow for feature work, regression fixes, and review follow-ups.

## Core Process

Use the narrowest relevant test command while iterating.

- For pure library behavior, prefer targeted Rust tests such as `cargo test lint::tests::...`, `cargo test config::tests::...`, or `cargo test root::tests::...`.
- For CLI behavior, prefer targeted integration tests such as `cargo test --test cli <exact-test-name>`, `cargo test --test config <exact-test-name>`, or `cargo test --test path_behavior <exact-test-name>`.

Run `cargo xtask validate` from the repository root before considering work complete. It performs formatting checks, Clippy, the full Rust test suite under coverage, dependency checks, and policy validation.

## Testing Philosophy

Keep the test suite split by responsibility.

- Unit tests must be pure. They should exercise deterministic parsing, classification, normalization, reporting, and config-merging logic without depending on real filesystem layout, network access, or external processes.
- Any behavior that fundamentally depends on filesystem traversal, repository-root discovery, config-file lookup, Git ignore handling, command execution, or end-to-end CLI output belongs in integration tests.
- `docgarden` is repository-local and deterministic. Tests should preserve that model by avoiding live network access entirely.

In practice, this means:

- Pure unit tests live alongside the implementation under `src/` and should construct values directly in memory whenever possible.
- CLI integration tests live under `tests/` and should execute the compiled `docgarden` binary with `assert_cmd`.
- Checked-in fixture repositories under `tests/test-repos/` should be copied into a temporary directory before mutation so each test gets an isolated working tree.
- Shared CLI harness code should live in `tests/common/` so new integration coverage reuses the same fixture-copy and setup patterns.

## Unit Test Rules

All unit tests must be "pure".

- Do not perform network operations in unit tests.
- Do not depend on repository-external tools or shell commands in unit tests.
- Avoid real filesystem traversal and mutable on-disk scenarios in unit tests when the behavior can be expressed as a pure function or in-memory value transformation.
- If a test needs real files, directories, config discovery, or command invocation to be meaningful, move it to `tests/` as an integration test instead of expanding the unit-test surface.

Good unit-test targets in this repository include:

- inline reference classification and path-adjacent heuristics in `src/lint/`
- path normalization and rendering helpers in `src/lint/references.rs`
- ignore-pattern matching and fix-summary behavior in `src/diagnostics.rs`
- config parsing and merge behavior in `src/config.rs`

## CLI Integration Coverage

Every user-visible CLI switch, subcommand, or repository-walking behavior must have at least one end-to-end integration test.

- Cover command-surface behavior in `tests/cli.rs` and `tests/config.rs`.
- Cover path-resolution and repository-layout scenarios in `tests/path_behavior.rs`.
- Prefer copied fixtures from `tests/test-repos/` when the scenario represents a realistic repository state that should stay easy to inspect.
- Prefer temporary directories created inside the test when the scenario is small, highly specific, or easier to express procedurally than as a checked-in fixture.
- When fix behavior is involved, assert both the rewritten file contents and a second lint pass so idempotence is proven. For repository dogfooding in this repo, use `cargo run -- lint ...` rather than assuming the `docgarden` binary is already installed on `PATH`.

CLI integration tests are the right home for:

- filesystem discovery and traversal behavior
- repository-root inference
- config-file discovery and explicit `--config` handling
- `--no-gitignore` and gitignore-sensitive scanning
- human-readable and JSON CLI output
- subcommand help text and flag exposure
- autofix behavior and rerun safety

## TDD for Bugs and Review Findings

When a bug report or review finding arrives, always follow TDD:

1. Add a failing test that reproduces the reported behavior.
2. Implement the fix.
3. Re-run the targeted test and then the relevant broader suite until they pass.
4. Run `cargo xtask validate` before shipping.

Do not ship a bug fix or review-follow-up behavior change without the reproducer test.

As a rule of thumb:

- If the bug is in pure logic, start with a unit test near the affected module.
- If the bug is visible through the CLI, depends on files on disk, or involves repository scanning, start with an integration test under `tests/`.

## Documentation Check

After updating this document or other repository docs, dogfood the current CLI shape explicitly from the repository root: run `cargo run -- lint <changed-files> --color never` so stale repository-local references and example-path mistakes are caught locally, and use `cargo run -- fix <targets> --color never` only when you intend to apply safe rewrites.
