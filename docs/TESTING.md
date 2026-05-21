---
description: "Canonical testing workflow and philosophy for `docgarden`: unit-versus-integration boundaries, fixture repository expectations, TDD, validation commands, and documentation linting."
---

# Testing

This document defines the canonical testing process for `docgarden`. Treat it as the default workflow for feature work, regression fixes, and review follow-ups.

## Testing Philosophy

Five principles, ordered by how often they show up at review.

1. **Test at the boundary that owns the behavior.** Pure parsing, classification, normalization, ranking, config merging, and reporting logic should be tested as deterministic Rust behavior. Filesystem traversal, repository-root discovery, config lookup, Git ignore handling, command execution, autofix rewrites, and CLI output should be tested through integration tests that exercise the compiled binary.

2. **Fixtures are contracts.** A fixture repository's name and contents are load-bearing documentation. If a fixture cannot express the scenario being tested, do not pretend it does; surface the gap.

3. **Gaps are surfaced, not papered over.** A test that appears to exercise a capability while silently accepting the wrong outcome is worse than no test. When a matcher behavior, lint rule, config path, or fixture repository cannot yet support a scenario, make that limitation visible in the plan, issue, fixture name, or test name instead of hiding it behind a compensating assertion.

4. **Live repository scenarios matter.** `docgarden` is a repository-local CLI. End-to-end tests should use temporary directories, copied fixture repositories, real Markdown files, real config files, and actual command execution when the behavior depends on repository state. Synthetic in-memory values are for pure logic, not for workflows whose risk is in walking and mutating a repo.

5. **Tests assert behavior, not implementation.** A test should fail when a caller-visible behavior regresses, not when an internal helper, field, constant, or formula is rearranged. Test-only production shapes are rejected; code shape rules for that live in `docs/CODESTYLE.md`.

## Test Placement

Put tests at the narrowest boundary that can observe the behavior without distorting the code.

- Use `src/` unit tests for pure Rust behavior and for private or `pub(crate)` items. Top-level `tests/` files compile as separate crates and should not drive production visibility changes.
- Use top-level `tests/` for compiled-binary behavior: CLI flags, user-visible output, fixture repositories, command execution, and end-to-end repository workflows.
- Do not make modules or helpers public only so a test can move to `tests/`.
- Keep small filesystem-backed unit tests under `src` when the filesystem is the smallest meaningful boundary of the library function under test, such as root inference or config loading.
- Move full repository workflows, fixture-repository scenarios, and compiled command execution to `tests/`.

## Unit Tests

Unit tests should stay deterministic and local.

- Do not depend on repository-external tools or shell commands.
- Prefer in-memory values for parsing, classification, normalization, reporting, ranking, and config-merging logic.
- Assert observable behavior. Avoid tests that only re-derive a formula from the same constants or inspect implementation-only state.

`docgarden` is repository-local and deterministic. Tests should preserve that model by keeping external dependencies out of the inner loop.

## Integration Tests

Every user-visible CLI switch, subcommand, or repository-walking behavior must have at least one end-to-end integration test. Prefer one integration test per user-visible workflow boundary, with unit tests covering rule permutations beneath it. Add duplicate end-to-end coverage only for regressions, risky wiring, or behavior not observable through lower-level tests.

- Execute the compiled `docgarden` binary with `assert_cmd`.
- Copy checked-in fixture repositories into a temporary directory before mutation so each test gets an isolated working tree.
- Assert both rewritten file contents and a second lint pass when fix behavior is involved.
- Prefer checked-in fixtures for realistic repository states and procedural temporary directories for small, focused scenarios.
- Keep shared CLI harness code in `tests/common/`.

Fixture repositories are not just setup code. Keep fixture contents aligned with the scenario their names promise, and do not accept a wrong result to make an ill-fitting fixture pass.

## TDD for Bugs and Review Findings

When a bug report or review finding arrives:

1. Add a failing test that reproduces the reported behavior.
2. Implement the fix.
3. Re-run the targeted test and then the relevant broader suite until they pass.
4. Run `cargo xtask validate` before shipping Rust behavior changes.

Do not ship a bug fix or review-follow-up behavior change without the reproducer test.

## Core Process

Use the narrowest relevant test command while iterating.

- For pure library behavior, prefer targeted Rust tests such as `cargo test lint::tests::...`, `cargo test config::tests::...`, or `cargo test root::tests::...`.
- For CLI behavior, prefer targeted integration tests such as `cargo test --test cli <exact-test-name>`, `cargo test --test config <exact-test-name>`, or `cargo test --test path_behavior <exact-test-name>`.

Run `cargo xtask validate` from the repository root before considering Rust behavior changes complete. Documentation-only, CI-only, metadata-only, and lint/config-only changes may close with focused checks instead when those checks cover the touched surface.

## Documentation Check

After updating this document or other repository docs, dogfood the current CLI shape explicitly from the repository root: run `cargo run -- lint <changed-files> --color never` so stale repository-local references and example-path mistakes are caught locally.

Use `cargo run -- fix <targets> --color never` only when you intend to apply safe rewrites.
