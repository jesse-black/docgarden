# Implement Minimal Rules-Only Configuration

This completed ExecPlan lives at `docs/exec-plans/completed/configuration-rule-application.md`.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document according to `docs/PLANS.md`.

## Purpose / Big Picture

After this change, users can configure existing `docgarden` lint behavior with one minimal rules-only shape. A `[[rules]]` entry targets repository-relative paths with `path`, disables or enables existing rule names, and may override `path_style` for that target. The plan intentionally removes the unshipped `[[documents]]`, `[per-file-ignores]`, `[[rules]].match`, `local-reference-style`, and top-level `report-ambiguous-inline-code` shapes so the branch does not carry compatibility code for an API that never shipped.

The observable behavior is limited to existing local reference style enforcement, unresolved local path diagnostics, ambiguous inline-code warnings, include and exclude scanning, extension and special-filename classification, gitignore respect, and rule-specific suppressions. Do not implement `skills_dir`, `scope = "skills"`, front matter validation, discovery commands, context budgets, imported-reference policy, curated indexes, or generated guidance in this plan.

## Progress

- [x] (2026-04-06 22:00Z) Earlier implementation added the larger `[[documents]]` plus `[[rules]].match` shape, then evaluator review and PR feedback found that the public shape was more complex than the current requirement needed.
- [x] (2026-04-06 23:10Z) Reopened this ExecPlan from `docs/exec-plans/completed/` to `docs/exec-plans/active/` and rewrote the completion bar around the simplified rules-only surface requested by the user.
- [x] (2026-04-06 23:18Z) Removed document-family parsing, branch-only compatibility keys, and overloaded `match` handling from `src/config.rs`; updated focused config tests and `cargo test config` passes.
- [x] (2026-04-06 23:30Z) Updated CLI/path behavior tests, fixtures, README, root `docgarden.toml`, and design docs so the current public examples use `path` and `path_style`.
- [x] (2026-04-06 23:36Z) Ran targeted integration tests, targeted doc linting, and `cargo xtask validate`; this plan is ready for evaluator review.

## Surprises & Discoveries

- Observation: The previous `[[documents]]` layer added naming, expansion, duplicate handling, and typo behavior without a current second consumer.
  Evidence: The simplified design in `docs/design-docs/configuration.md` now says `[[documents]]` should stay deferred until at least two concrete features need the same named group.
- Observation: The previous `match` field was overloaded between family names and path patterns.
  Evidence: PR feedback showed that `match = "refrences"` was accepted before an ad hoc `looks_like_document_family_name` guard was added. The new plan removes `match` entirely and uses `path` for path targets.
- Observation: The earlier branch compatibility story was unnecessary.
  Evidence: The user clarified the config shape has not shipped, so removed shapes should be rejected instead of kept as aliases.

## Decision Log

- Decision: Use a rules-only public configuration shape for this plan.
  Rationale: Current behavior only needs path-scoped rule application. Generic document groups should wait until a later feature actually needs them.
  Date/Author: 2026-04-06 / Planner
- Decision: Use `path` as the only `[[rules]]` target field.
  Rationale: The value accepts repository-relative literal paths and gitignore-style path patterns. `path` reads well for both `README.md` and `docs/**`; `match` was too ambiguous.
  Date/Author: 2026-04-06 / Planner
- Decision: Use `path_style` for both the repo-wide default and path-scoped overrides.
  Rationale: The design docs already describe this as path style, and the user selected snake_case as the new TOML convention for this unshipped config surface.
  Date/Author: 2026-04-06 / Planner
- Decision: Remove branch-only compatibility for `[[documents]]`, `[per-file-ignores]`, `[[rules]].match`, `local-reference-style`, and top-level `report-ambiguous-inline-code`.
  Rationale: These shapes have not shipped. Keeping aliases would preserve the complexity this plan is removing.
  Date/Author: 2026-04-06 / Planner
- Decision: Do not add `skills_dir` or `scope = "skills"` in this plan.
  Rationale: Skills configuration is a real future need, but this plan is only about existing lint behavior. Adding built-in scopes now would widen the implementation beyond the current acceptance criteria.
  Date/Author: 2026-04-06 / Planner

## Outcomes & Retrospective

The earlier closure result is superseded. This plan is active again because review and design discussion found that the previous implementation overfit future document-family needs.

2026-04-06 outcome: The simplified rules-only surface is implemented and ready for evaluator review. Public config now uses `path_style` and `[[rules]].path`; the branch-only `[[documents]]`, `[per-file-ignores]`, `[[rules]].match`, `local-reference-style`, and top-level `report-ambiguous-inline-code` shapes are rejected by parsing tests. `cargo xtask validate` passed, with only a non-failing coverage-gate warning that the reopened ExecPlan is an untracked new file until the move is staged.

2026-04-07 evaluator outcome: Passed clean-room evaluation and moved back to completed. Evidence reviewed: the current working-tree diff against `main`; parser code showing `FileConfig` and `RuleConfig` deny unknown fields and expose only the rules-only shape; lint lowering that maps `[[rules]].disable`, `[[rules]].enable = ["ambiguous-inline-code"]`, and `[[rules]].path_style` to existing per-path policy; `rg` checks for removed public shapes; targeted acceptance tests in `src/config.rs` and `tests/path_behavior.rs`; `cargo test config`; `cargo test --test path_behavior`; `cargo test --test cli`; targeted `cargo run -- lint ... --color never`; and `cargo xtask validate`. No blocking findings remained. The only validation warning was covgate noting that the active plan file was untracked during evaluation; this is expected for the in-progress move and does not affect implementation evidence.

## Context and Orientation

`docgarden` is a Rust CLI that lints repository-local Markdown references. `src/cli.rs` resolves targets, finds the repository root, loads `docgarden.toml` through `src/config.rs`, discovers Markdown files through `src/discover.rs`, and calls `src/lint/mod.rs::lint_file` for each file.

The current configuration parser lives in `src/config.rs`. `FileConfig` is the TOML shape read from `docgarden.toml`; `Config` is the effective configuration used by the rest of the program. Existing linting already knows how to suppress rule names for a file path, choose the effective path style for a file, and decide whether to report ambiguous inline code for a file.

The existing rule names are:

- `unresolved-local-path`
- `prefer-links-for-local-paths`
- `prefer-backticks-for-local-paths`
- `ambiguous-inline-code`

The final public TOML shape for this plan is:

    path_style = "backticks"

    [[rules]]
    path = "docs/references/**"
    disable = ["unresolved-local-path"]
    reason = "Imported references may preserve source-derived paths."

    [[rules]]
    path = "docs/**"
    enable = ["ambiguous-inline-code"]

    [[rules]]
    path = "README.md"
    path_style = "links"

The field `path` is repository-relative and uses the same gitignore-style path pattern semantics as include and exclude matching. The field `path_style` accepts `backticks` or `links`. The field `reason` is accepted for human review and is not used by linting.

## Plan of Work

First, keep `src/config.rs` strict. `FileConfig` should deny unknown fields, parse only snake_case public keys, and expose no public parsed `[per-file-ignores]`, `documents`, `match`, `local-reference-style`, or top-level `report-ambiguous-inline-code` fields. `RuleConfig` should require `path`, optionally accept `disable`, `enable`, `path_style`, and `reason`, and reject unknown future fields through Serde.

Second, keep the effective implementation small. Lower `[[rules]].disable` into the internal ignored-rules map consumed by `src/lint/mod.rs`. Lower `[[rules]].enable = ["ambiguous-inline-code"]` into path patterns checked by `Config::report_ambiguous_inline_code_for_path`. Lower `[[rules]].path_style` into the existing path-style override list used by `Config::local_reference_style_for_path`. There should be no document-family map and no heuristic that guesses whether a string looks like a family name.

Third, update tests and fixtures. Replace public `local-reference-style` examples with `path_style`. Replace top-level ambiguous-inline-code opt-in with `[[rules]] path = "**" enable = ["ambiguous-inline-code"]`. Replace `[per-file-ignores]` tests with `[[rules]] path = ... disable = [...]`. Replace document-family tests with direct path-scope tests. Add or keep negative tests proving `[[documents]]`, `[per-file-ignores]`, `[[rules]].match`, `local-reference-style`, and top-level `report-ambiguous-inline-code` fail parsing.

Fourth, update docs and dogfood config. README and the root `docgarden.toml` should use only `path` and `path_style`. Design docs may mention `[[documents]]` only briefly as a deferred idea, not as implementation scope.

## Concrete Steps

From the repository root, inspect the working tree before editing:

    git status --short

Then edit:

- `src/config.rs` for the strict rules-only parsed shape, lowering, and unit tests;
- `src/lint/mod.rs` only as needed to keep test helpers in sync with the effective config;
- `tests/path_behavior.rs`, `tests/cli.rs`, and test fixture configs for CLI behavior coverage;
- `README.md`, `docgarden.toml`, and design docs for examples and dogfooding;
- this ExecPlan after each meaningful stopping point.

Run targeted tests during development:

    cargo test config
    cargo test --test path_behavior
    cargo test --test cli

After documentation edits, run targeted doc linting for changed docs:

    cargo run -- lint README.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/design-docs/skill-root-and-templated-agent-guidance.md docs/design-docs/standardized-yaml-front-matter.md docs/exec-plans/active/configuration-rule-application.md --color never

Before marking ready for evaluation, run:

    cargo xtask validate

## Validation and Acceptance

Acceptance requires all of these independently verifiable behaviors:

1. A `[[rules]] path = "docs/references/**" disable = ["unresolved-local-path"]` entry suppresses an unresolved local path only under `docs/references/**`; the same broken link in `README.md` still reports `unresolved-local-path`.
2. A `[[rules]] path = "README.md" disable = ["prefer-backticks-for-local-paths"]` entry suppresses the style rule without suppressing `unresolved-local-path` in that file.
3. A `[[rules]] path = "docs/**" enable = ["ambiguous-inline-code"]` entry reports ambiguous inline code in matching docs files but not in `README.md`.
4. A repo-wide `path_style = "backticks"` plus `[[rules]] path = "docs/**" path_style = "links"` reports `prefer-links-for-local-paths` only in matching files.
5. The parser rejects `[[rules]] match = "docs/**"`.
6. The parser rejects `[[documents]]`.
7. The parser rejects `[per-file-ignores]`.
8. The parser rejects `report-ambiguous-inline-code = true`.
9. The parser rejects `local-reference-style = "backticks"`.
10. Unsupported future rule fields such as `rule = "context-budget"` or `max-lines` fail parsing.
11. Existing include, exclude, extension, special filename, `respect_gitignore`, and explicit config path behavior keep working with snake_case config keys.
12. Validation commands pass:

        cargo test config
        cargo test --test path_behavior
        cargo test --test cli
        cargo run -- lint README.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/design-docs/skill-root-and-templated-agent-guidance.md docs/design-docs/standardized-yaml-front-matter.md docs/exec-plans/active/configuration-rule-application.md --color never
        cargo xtask validate

## Idempotence and Recovery

The implementation is safe to retry. Re-running tests and lint commands should not mutate tracked files. If config parsing becomes too strict and breaks an intended accepted key, restore only that specific key and add a test proving it is part of the desired snake_case surface. Do not restore branch-only compatibility for removed shapes unless the user changes the requirement.

Do not use destructive git commands to recover. If generated coverage or target artifacts change, ignore them unless they are tracked by git. Preserve unrelated user changes in the working tree.

## Artifacts and Notes

`cargo test config` passed after the first parser cleanup. It ran 8 config-filtered tests across unit and integration targets, including the new removed-shape parsing tests.

After the full cleanup, these commands passed:

    cargo test config
    cargo test --test path_behavior
    cargo test --test cli
    cargo run -- lint README.md docs/design-docs/backtick-path-classification.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/design-docs/path-style-policy.md docs/design-docs/skill-root-and-templated-agent-guidance.md docs/design-docs/standardized-yaml-front-matter.md docs/exec-plans/active/configuration-rule-application.md --color never
    cargo xtask validate

## Interfaces and Dependencies

No new crate dependency is expected. Use existing dependencies: `serde` for TOML deserialization, `toml` for config parsing, `anyhow` for validation errors, and `ignore::gitignore::GitignoreBuilder` through the existing pattern-matching approach.

At the end of implementation, `src/config.rs` must expose enough effective configuration for `src/lint/mod.rs` to answer these per-file questions:

- Which rule names are disabled for this relative path?
- What is the effective `LocalReferenceStyle` for this relative path?
- Is `ambiguous-inline-code` enabled for this relative path?

The names and visibility of helper structs are implementation details, but tests must cover the behavior above through `Config::load` and CLI runs.

Revision note: 2026-04-06 / Planner reopened this plan after review showed that `[[documents]]` and overloaded `match` were unnecessary for the current requirement. The new plan uses a minimal rules-only shape with snake_case TOML keys, `path_style`, and `[[rules]].path`.

Revision note: 2026-04-07 / Evaluator closed this plan after independent branch review and validation found the rules-only configuration acceptance criteria satisfied.
