---
description: "Address the code smells and defensive programming recommendations documented in docs/investigations/review_findings.md, including Config field encapsulation, CLI output-mode conflicts, Debug destructuring, mutability blocks, and boolean parameter refactoring."
---

# Address Codebase Review Findings

## Goal
- Resolve all codebase smells and defensive programming issues identified in `docs/investigations/review_findings.md`.

## Scope
- In:
  - Destructuring `self` completely in manual `Debug` implementation for `Config` in `src/config.rs`.
  - Restricting mutability of variables to localized initialization blocks for `sorted`/`rewritten` in `apply_edits` (`src/lint/mod.rs`) and `results` in `execute_match` (`src/matching.rs`).
  - Making all fields of `Config` private, exposing read-only getter methods plus an intent-named `disable_gitignore()` mutator, and updating all usage locations in `src/` and tests.
  - Preventing `docgarden match --path-only --explain ...` at the CLI boundary with a clap conflict, then lowering validated flags into an internal `MatchOutputMode` enum before calling matcher code.
  - Refactoring the `style_output` boolean parameter in `src/matching.rs` to a `ColorRendering` enum.
  - Refactoring the boolean parameters in `CandidateReference::new` in `src/lint/references.rs` to a `ReferenceSyntax` enum.
- Out:
  - Replacing `FrontmatterParseResult::Malformed { .. }` with `Malformed { line: _ }`; this conflicts with `clippy::unneeded_field_pattern`, and the current match intentionally ignores malformed-frontmatter details.
  - Slicing and pattern matching changes to `is_camel_boundary` in `src/analyzer.rs` (decided that the original implementation is cleaner and safer).

## Relevant Areas
- `src/config.rs` — `Config` struct fields, getters, setters, and `Debug` implementation.
- `src/lint/mod.rs` — mutability of `sorted`/`rewritten` in `apply_edits`.
- `src/matching.rs` — mutability of `results` in `execute_match`, and `style_output` boolean parameter.
- `src/lint/references.rs` — `CandidateReference::new` parameters.
- `src/cli.rs` — update to use `Config` getters/mutator and reject conflicting `match` output-mode flags.
- `src/discover.rs` — update to use `Config` getters.
- `src/scopes.rs` — update to use `Config` getters.
- `src/listing.rs` — update to use `Config` getters.
- `tests/cli.rs` — add compiled-binary coverage for conflicting `--path-only` / `--explain` flags.

## Open Questions
- None yet

## Steps
- [x] **Step 1: Refactor `Config` Debug Implementation**
  - Destructure `self` completely inside the `std::fmt::Debug` implementation for `Config` in `src/config.rs`.
- [x] **Step 2: Restrict Mutability in `apply_edits` and `execute_match`**
  - In `src/lint/mod.rs` (`apply_edits`), restrict `sorted` and `rewritten` mutability to local blocks.
  - In `src/matching.rs` (`execute_match`), restrict `results` mutability to a local block.
- [x] **Step 3: Encapsulate `Config` Fields**
  - Make all fields of `Config` private in `src/config.rs`.
  - Add getter methods for all read-only fields accessed outside of `src/config.rs`, including `repository_root`, `skills_dir`, `plans_dir`, `include`, `exclude`, `rule_applications`, `known_extensions`, `special_filenames`, `config_path`, `config_was_explicit`, `frontmatter_rules`, and `respect_gitignore`.
  - Add `disable_gitignore(&mut self)` on `Config` for the `--no-gitignore` call sites.
  - Update all caller sites in `src/cli.rs`, `src/discover.rs`, `src/documents.rs`, `src/scopes.rs`, `src/listing.rs`, `src/lint/mod.rs`, and tests to use these getters/mutator instead of direct field access or struct literals.
- [x] **Step 4: Model Match Output Modes Explicitly**
  - Add a clap conflict between `MatchArgs::path_only` and `MatchArgs::explain` so the invalid user-facing flag combination fails during argument parsing.
  - Create an internal enum such as `MatchOutputMode::{Default, PathOnly, Explain}` in `src/matching.rs`.
  - Replace `MatchOptions { path_only: bool, explain: bool }` with `MatchOptions { output_mode: MatchOutputMode }`.
  - Lower the validated `MatchArgs` flags in `src/cli.rs` into exactly one `MatchOutputMode` before calling `matching::execute_match`.
  - Add an integration test in `tests/cli.rs` that reproduces the current ambiguous behavior and passes only when `docgarden match --path-only --explain ...` fails at the CLI boundary.
- [x] **Step 5: Refactor `style_output` Boolean to Enum**
  - Create a new enum `ColorRendering { Plain, Ansi }` in `src/matching.rs`.
  - Replace the `style_output: bool` parameters in the rendering helper functions inside `src/matching.rs` with `ColorRendering`.
- [x] **Step 6: Refactor `CandidateReference::new` Boolean Flags to Enum**
  - Create a new enum `ReferenceSyntax { Standard, Relative, WorkspaceRoot }` in `src/lint/references.rs`.
  - Update `CandidateReference::new` signature to accept `ReferenceSyntax` instead of two booleans, and update callers.

## Validation
- Run targeted tests:
  - `cargo test config::tests`
  - `cargo test lint::tests`
  - `cargo test matching::tests`
  - `cargo test --test cli match_path_only_and_explain_conflict`
- Run the full workspace validation suite:
  - `cargo xtask validate`
- Verify documentation is clean:
  - `cargo run -- lint docs/exec-plans/active/0019-address-review-findings.md --color never`

## Discoveries
- `Config::for_testing(root)` added as a `#[cfg(test)] pub(crate)` constructor to replace the test-only struct literal in `src/lint/mod.rs`. Avoids exposing fields just for tests (CODESTYLE principle 6).
- Review findings 1–3 addressed: tests now use getters (`config_path()`, `config_was_explicit()`, `respect_gitignore()`, `known_extensions()`, `special_filenames()`, `repository_root()`, `include()`), `rule_applications`/`frontmatter_rules` tests replaced with behavioral checks through `effective_rule_policy_for_path`, and `skills_dir`/`plans_dir` assertions rewritten to use `skills_root()`/`plans_root()`.
- Reverted `{ line: _ }` on `FrontmatterParseResult::Malformed` back to `{ .. }`; the plan was rescoped to not change the wildcard match due to clippy `unneeded_field_pattern` conflict.

## Review

- [x] **Finding 1 (CODESTYLE violation — Principle 6, §"Keep production and test type shapes identical"):**
  Unit tests in `src/config.rs` directly access private fields that Step 4 made private: `config_path` (lines 719, 744, 776, 824), `rule_applications` (lines 720, 746, 1507–1508), `frontmatter_rules` (line 721), `skills_dir` (lines 722, 779), `plans_dir` (lines 723, 780), `include` (lines 747, 826), `known_extensions` (lines 781–783), `special_filenames` (lines 784–785), `respect_gitignore` (lines 778, 888), `config_was_explicit` (lines 777, 825), and `repository_root` (line 741). Because the tests live in `mod tests` inside `src/config.rs`, Rust allows this — but it defeats the encapsulation goal and violates CODESTYLE Principle 6 ("tests assert behavior, not implementation"). The getters added in Step 4 (`config_path()`, `config_was_explicit()`, `respect_gitignore()`, `known_extensions()`, `special_filenames()`) should be what tests call. Tests that check `rule_applications.len()` and `rule_applications[0].severity` directly are also implementation checks: the observable behavior is `effective_rule_policy_for_path()`, which is already asserted on lines 1510–1514 of the same test.

- [x] **Finding 2 (missing getters for `skills_dir` and `plans_dir`):**
  `Config` exposes `skills_root()` and `plans_root()` (which join the repository root to the relative dir path) but no getters for the underlying `skills_dir` / `plans_dir` fields. Tests at lines 722–723 and 779–780 assert on the raw relative paths. Either expose `skills_dir()` / `plans_dir()` getters so tests can use the public API, or rewrite those assertions to use `skills_root()` / `plans_root()` (which covers the same observable behavior, just with the root prepended). Without a getter the tests are bypassing the encapsulation boundary.

- [x] **Finding 3 (minor — `for_testing` doc comment contradicted by test code):**
  The `Config::for_testing` doc comment says it "Avoids exposing fields just for tests (CODESTYLE principle 6)", but the existing tests in the same module still bypass getters to read private fields. The comment is accurate about what `for_testing` itself does, but may mislead a future maintainer into thinking the encapsulation is complete when it is not. This is a documentation accuracy issue, not a code defect, but it should be fixed once Finding 1 is resolved.

- [x] **Evaluator (2026-05-21): Findings 1–3 confirmed addressed.** Diff inspection shows all `src/config.rs` tests use getters (`config_path()`, `config_was_explicit()`, `respect_gitignore()`, `known_extensions()`, `special_filenames()`, `repository_root()`, `include()`); `rule_applications`/`frontmatter_rules` implementation checks replaced with behavioral checks through `effective_rule_policy_for_path()` and `frontmatter_policy_for_path()`. `skills_dir`/`plans_dir` assertions rewritten to use `skills_root()`/`plans_root()` joined against `repository_root()`. `for_testing` doc comment now accurate — tests no longer bypass encapsulation.
- [x] **Evaluator: CODESTYLE.md compliance verified.** Principle 3 (invalid states unrepresentable): `MatchOutputMode` enum eliminates two-boolean flag conflict; `ReferenceSyntax` enum replaces two boolean flags; `Debug` impl fully destructures `Config` so new fields force a compile error. Principle 6 (tests assert behavior): all unit test assertions use public getters or behavioral policy queries. No `#[cfg(test)]` on production fields. `_ => {}` match arms replaced with explicit variant lists in `config.rs`, `references.rs`, and `frontmatter.rs`.
- [x] **Evaluator: TESTING.md compliance verified.** Integration test `match_path_only_and_explain_conflict` added in `tests/cli.rs` for the `--path-only`/`--explain` conflict. Targeted test runs (`cargo test config::tests`, `cargo test lint::tests`, `cargo test matching::tests`) listed in plan's Validation section pass. Full `cargo xtask validate` passes with 96.59% coverage.
- [x] **Evaluator: `FrontmatterParseResult::Malformed { .. }` reversion confirmed.** Pattern remains `{ .. }` in `src/documents.rs:27`; the `{ line: _ }` change was correctly rescoped out (clippy `unneeded_field_pattern`). Destructured `{ line }` in `src/lint/rules/frontmatter.rs:49` is a legitimate use that reads the line number to report the diagnostic — not a wildcard pattern.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [x] Goal achieved: Resolve all codebase smells and defensive programming issues identified in `docs/investigations/review_findings.md`.
- [x] All planned steps are complete.
- [x] All validation commands pass.
- [x] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Pass 1: Cold review completed.
- [ ] Pass 2: Context review completed:
    - [ ] Adheres to the principles of `docs/CODESTYLE.md`.
    - [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
