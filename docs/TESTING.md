---
description: "Canonical testing workflow and philosophy for `docgarden`: principles for unit tests, integration tests, fixture repositories, TDD, validation commands, and documentation linting; read when adding features, fixing regressions, addressing review findings, reviewing test changes, or deciding how to verify repository-local behavior."
---

# Testing

This document defines the canonical testing process for `docgarden`. Treat it as the default workflow for feature work, regression fixes, and review follow-ups.

This document is structured like `docs/CODESTYLE.md`: a small philosophy followed by worked examples. The philosophy is durable; the rules in practice are illustrative. New testing rules must derive from an existing principle, or motivate a sharpening of one. The principle set is the spine of the document.

## Testing Philosophy

Five principles, ordered by how often they show up at review. Each rule below cites one or more of them.

1. **Test at the boundary that owns the behavior.** Pure parsing, classification, normalization, ranking, config merging, and reporting logic should be tested as deterministic Rust behavior. Filesystem traversal, repository-root discovery, config lookup, Git ignore handling, command execution, autofix rewrites, and CLI output should be tested through integration tests that exercise the compiled binary.

2. **Fixtures are contracts.** A fixture repository's name and contents are load-bearing documentation. A fixture named or used as an "active plan", "discovery repo", "gitignore-sensitive", or "fixable" scenario must actually demonstrate that capability. If a fixture cannot express the behavior yet, do not pretend it does; surface the gap.

3. **Gaps are surfaced, not papered over.** A test that appears to exercise a capability while silently accepting the wrong outcome is worse than no test. When a matcher behavior, lint rule, config path, or fixture repository cannot yet support a scenario, make that limitation visible in the plan, issue, fixture name, or test name instead of hiding it behind a compensating assertion.

4. **Live repository scenarios matter.** `docgarden` is a repository-local CLI. End-to-end tests should use temporary directories, copied fixture repositories, real Markdown files, real config files, and actual command execution when the behavior depends on repository state. Synthetic in-memory values are for pure logic, not for workflows whose risk is in walking and mutating a repo.

5. **Tests assert behavior, not implementation.** A test should fail when a caller-visible behavior regresses, not when an internal helper, field, constant, or formula is rearranged. Test-only production shapes are rejected; code shape rules for that live in `docs/CODESTYLE.md`.

When a finding does not cleanly cite one of these, the philosophy is the lever, not the rule list.

## Rules in practice

Worked examples grouped by topic. Each rule tags the principle(s) it expresses. Rules are illustrative; when a rule's example pattern no longer appears in the codebase, the rule has done its job and can be deleted. The philosophy stays.

### Place tests where their boundary dictates

*Principle 1.*

- ALWAYS put pure Rust behavior tests alongside the implementation under `src/` when they construct values directly in memory and do not need a real repository.
- ALWAYS put CLI behavior under `tests/` when the test depends on files, directories, config discovery, repository walking, Git ignore handling, process invocation, or user-visible output.
- NEVER expand an inline unit test into a miniature repository scenario. If the behavior needs real files or command execution to be meaningful, move it to an integration test.

Good unit-test targets in this repository include:

- inline reference classification and path-adjacent heuristics in `src/lint/`
- path normalization and rendering helpers in `src/lint/references.rs`
- ignore-pattern matching and fix-summary behavior in `src/diagnostics.rs`
- config parsing and merge behavior in `src/config.rs`

Good integration-test targets include:

- filesystem discovery and traversal behavior
- repository-root inference
- config-file discovery and explicit `--config` handling
- `--no-gitignore` and gitignore-sensitive scanning
- human-readable and JSON CLI output
- subcommand help text and flag exposure
- autofix behavior and rerun safety

### Keep unit tests pure

*Principles 1, 4.*

- NEVER perform network operations in unit tests.
- NEVER depend on repository-external tools or shell commands in unit tests.
- ALWAYS prefer in-memory values for deterministic parsing, classification, normalization, reporting, and config-merging logic.
- ALWAYS move real filesystem traversal and mutable on-disk scenarios to `tests/` unless the filesystem itself is the smallest meaningful unit of behavior.

`docgarden` is repository-local and deterministic. Tests should preserve that model by avoiding live network access entirely.

### Test live repository workflows through the CLI

*Principles 1, 4.*

Every user-visible CLI switch, subcommand, or repository-walking behavior must have at least one end-to-end integration test. Prefer one integration test per user-visible workflow boundary, with unit tests covering rule permutations beneath it. Add duplicate end-to-end coverage only for regressions, risky wiring, or behavior not observable through lower-level tests.

- ALWAYS execute the compiled `docgarden` binary with `assert_cmd` for CLI integration behavior.
- ALWAYS copy checked-in fixture repositories into a temporary directory before mutation so each test gets an isolated working tree.
- ALWAYS assert both the rewritten file contents and a second lint pass when fix behavior is involved, so idempotence is proven.
- ALWAYS use `cargo run -- lint ...` for repository dogfooding in this repo instead of assuming the `docgarden` binary is already installed on `PATH`.
- PREFER temporary directories created inside the test when the scenario is small, highly specific, or easier to express procedurally than as a checked-in fixture.
- PREFER checked-in fixture repositories when the scenario represents a realistic repository state that should stay easy to inspect.

- Cover command-surface behavior in `tests/cli.rs` and `tests/config.rs`.
- Cover path-resolution and repository-layout scenarios in `tests/path_behavior.rs`.
- Keep shared CLI harness code in `tests/common/` so new integration coverage reuses the same fixture-copy and setup patterns.

### Treat fixture repositories as load-bearing documentation

*Principles 2, 3.*

Fixture repositories are not just setup code. Their names and checked-in files tell reviewers what scenario they represent.

- ALWAYS keep fixture contents aligned with the scenario the fixture name promises.
- ALWAYS prefer a checked-in fixture repository when a scenario is broad enough that future readers should inspect the full repo shape.
- NEVER reuse a fixture for a scenario it only accidentally supports. Create a focused fixture or build the scenario procedurally in a temp directory.
- NEVER make a test pass by accepting an obviously wrong outcome from a fixture. Rename the test, change the fixture, or record the missing behavior explicitly.

This pattern is wrong:

```rust
// WRONG: fixture name promises an active plan, but the test accepts none.
let (_temp, root) = fixture_repo("discovery-repo");
let matches = run_match(&root, "active plan");
assert!(matches.is_empty());
```

If the discovery fixture cannot match the active-plan query, the correct fix is one of:

- update the fixture so it contains the active-plan material the set promises
- use or create a fixture whose name accurately describes the missing-active-plan scenario
- record the missing behavior in an issue, ExecPlan, or `docs/TODO.md`

Docgarden does not currently maintain a generated fixture matrix. Do not add matrix-style testing language unless the codebase grows a real set of scenario-compatible fixtures that intentionally opt in or out of shared test cases.

### Test observable behavior, not internals

*Principle 5.*

Tests should assert what callers can see, not the shape of the implementation that produces it. Two anti-patterns recur and both should be rejected at review:

- NEVER write tests that assert an implementation matches its own formula. If a test re-derives the BM25 score from constants and asserts the function produces the same number, the test only fails when the constants drift out of sync with themselves. Replace it with assertions about ranking order, monotonicity, or other externally observable properties, such as "rare term outranks common term" or "longer combined length is penalized at fixed combined frequency".
- NEVER fence production fields with `#[cfg(test)]` so tests can read them. A struct that has different fields in `cargo test` versus `cargo build` is two types pretending to be one, and any assertion against the test-only fields is a white-box check that locks in implementation. Expose deliberately-test-visible state through `pub(crate) fn` accessors that exist in both profiles, or more often replace the white-box test with a behavioral one.

When tempted to introduce either pattern, restate the test as "what would a caller observe if this regressed?" If the answer is "nothing", the test is not earning its keep.

### Use TDD for bugs and review findings

*Principles 1, 3, 5.*

When a bug report or review finding arrives, always follow TDD:

1. Add a failing test that reproduces the reported behavior.
2. Implement the fix.
3. Re-run the targeted test and then the relevant broader suite until they pass.
4. Run `cargo xtask validate` before shipping Rust behavior changes.

Do not ship a bug fix or review-follow-up behavior change without the reproducer test.

As a rule of thumb:

- If the bug is in pure logic, start with a unit test near the affected module.
- If the bug is visible through the CLI, depends on files on disk, or involves repository scanning, start with an integration test under `tests/`.

## Core Process

Use the narrowest relevant test command while iterating.

- For pure library behavior, prefer targeted Rust tests such as `cargo test lint::tests::...`, `cargo test config::tests::...`, or `cargo test root::tests::...`.
- For CLI behavior, prefer targeted integration tests such as `cargo test --test cli <exact-test-name>`, `cargo test --test config <exact-test-name>`, or `cargo test --test path_behavior <exact-test-name>`.

Run `cargo xtask validate` from the repository root before considering Rust behavior changes complete. It performs formatting checks, Clippy, the full Rust test suite under coverage, dependency checks, and policy validation.

Documentation-only, CI-only, metadata-only, and lint/config-only changes may close with focused checks instead when those checks cover the touched surface.

## Documentation Check

After updating this document or other repository docs, dogfood the current CLI shape explicitly from the repository root: run `cargo run -- lint <changed-files> --color never` so stale repository-local references and example-path mistakes are caught locally.

Use `cargo run -- fix <targets> --color never` only when you intend to apply safe rewrites.

## Growing this document

This document is intended to stabilize, not accumulate. When a review surfaces a testing smell that is not covered:

1. **Find the principle that catches it.** If one of the five principles, taken seriously, would have prevented the smell, the principle has done its job and the rule list does not need a new entry. Cite the principle in the review and move on.
2. **If no principle fits, sharpen one before adding a new rule.** A novel finding usually means an existing principle is not crisply stated. Sharpening is preferred to expansion; the principle set should grow only when the existing five genuinely cannot generate the new rule.
3. **If a rule is required, write it as a worked example.** Tag the principle it expresses. Keep it concrete: name the pattern and the alternative. Avoid restating the principle as a rule.
4. **Prune aggressively.** When a rule's example pattern no longer appears in the codebase, the rule has stopped earning its place. Delete it. The principles persist; the examples are scaffolding.
5. **Cap the principle count.** Aim for five to seven. A growing principle list is a sign the principles have stopped being generative.

The test for whether this document is healthy is not "does it cover everything?" but "can a reviewer derive the right answer from the philosophy alone?" When the answer is yes, the rule list is doing what it should: illustrating, not legislating.
