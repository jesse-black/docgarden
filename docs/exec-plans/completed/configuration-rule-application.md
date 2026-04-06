# Implement Configuration Rule Application

This completed ExecPlan lives at `docs/exec-plans/completed/configuration-rule-application.md`.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document according to `docs/PLANS.md`.

## Purpose / Big Picture

After this change, users can configure today’s existing `docgarden` lint behavior with the shared configuration shape described by `docs/design-docs/configuration.md`: `[[documents]]` defines named document families, and `[[rules]]` applies existing rules to those families or to path patterns. This makes the configuration model ready for future repository-knowledge checks without implementing those future checks now.

The observable behavior is limited to features that already exist in the repository: local reference style enforcement, unresolved local path diagnostics, ambiguous inline-code warnings, include and exclude scanning, extension and special-filename classification, gitignore respect, and rule-specific suppressions. There must be no context-budget, front matter, discovery command, imported-reference, or curated-index implementation in this plan.

## Progress

- [x] (2026-04-06 22:00Z) Read `docs/PLANS.md`, `docs/design-docs/configuration.md`, `ARCHITECTURE.md`, `src/config.rs`, `src/diagnostics.rs`, `src/lint/mod.rs`, `src/discover.rs`, `tests/cli.rs`, `tests/path_behavior.rs`, `tests/config.rs`, `README.md`, and the root `docgarden.toml`.
- [x] (2026-04-06 22:00Z) Created a decision-complete ExecPlan that scopes the design to existing features only and leaves future feature families out of implementation.
- [x] (2026-04-06 22:03Z) Implemented the first pass of configuration parsing and effective-config lowering in `src/config.rs`, including document family expansion, rule validation, disabled-rule lowering, style overrides, and scoped ambiguous-inline-code enablement.
- [x] (2026-04-06 22:04Z) Added integration tests in `tests/path_behavior.rs` for document-family `disable`, path-pattern `disable`, scoped `enable = ["ambiguous-inline-code"]`, and scoped `local-reference-style`; `cargo test --test path_behavior` now passes.
- [x] (2026-04-06 22:06Z) Updated `README.md` with the implemented `[[documents]]` / `[[rules]]` configuration example and migrated the repository root `docgarden.toml` to dogfood the new shape.
- [x] (2026-04-06 22:07Z) Ran targeted tests, targeted doc linting, and `cargo xtask validate`; all pass after a small traversal-state cleanup removed fresh clippy warnings.
- [x] (2026-04-06 22:07Z) Prepared this plan for evaluator review without moving it to `completed/`; the generator considered the implementation ready for evaluation.
- [x] (2026-04-06 22:08Z) Completed an independent evaluator review against `main`, reran the branch validation commands, and found no blocking issues.
- [x] (2026-04-06 22:23Z) Completed a second independent evaluator review of the staged closure branch, reran targeted integration tests, targeted doc linting, whole-repo dogfood linting, and `cargo xtask validate`; no blocking issues were found.

## Surprises & Discoveries

- Observation: `src/config.rs` currently parses only the older shape: top-level scan/classification settings, `[per-file-ignores]`, `local-reference-style`, and `report-ambiguous-inline-code`.
  Evidence: `FileConfig` has no `documents` or `rules` field, and `Config` exposes `per_file_ignores`, `local_reference_style`, and `report_ambiguous_inline_code` directly.
- Observation: Rule suppression currently happens through `src/diagnostics.rs::ignored_rules_for_path`, which accepts a map of path pattern to rule names and is called once per file by `src/lint/mod.rs::lint_file`.
  Evidence: `lint_file` computes `ignored_rules` from `config.per_file_ignores` before walking the Markdown AST.
- Observation: The existing design doc was renamed to `docs/design-docs/configuration.md` before this ExecPlan was created, and context-budget examples now point to that shared configuration shape.
  Evidence: `docs/design-docs/context-budget-limits.md` now says context-budget limits should use `[[rules]]` instead of a separate `[[limits]]` table.
- Observation: At generator handoff, `git status --short` showed only this new ExecPlan as untracked; the earlier design-doc rename and context-budget edits were no longer uncommitted in this working tree.
  Evidence: `git status --short` printed only `?? docs/exec-plans/active/configuration-rule-application.md` at 2026-04-06 22:00Z.
- Observation: The first implementation could lower `[[rules]]` into the existing config model without adding a new lint rule engine.
  Evidence: `cargo test config` passed after adding `local_reference_style_for_path`, `report_ambiguous_inline_code_for_path`, and unit tests for document family expansion and future-rule rejection.
- Observation: `cargo test path_behavior` does not run the `tests/path_behavior.rs` integration target because Cargo treats the argument as a test-name filter.
  Evidence: The command reported `0 passed; 0 failed; 20 filtered out` for `tests/path_behavior.rs`. Running `cargo test --test path_behavior` executed all 20 tests and passed.
- Observation: The scoped style override behavior worked on the first integration run, but the exact-count assertion was too strict because the same rule name also appears in the fix-hint summary.
  Evidence: The failing output contained one diagnostic and one `Fixable rules in this run: prefer-links-for-local-paths` line. The test was updated to assert the scoped file and absence of `README.md` instead of counting rule-name occurrences.
- Observation: The first targeted doc lint failed because the active ExecPlan used hypothetical future paths and example paths as live backticked repo references.
  Evidence: `cargo run -- lint README.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/exec-plans/active/configuration-rule-application.md --color never` reported unresolved paths in this ExecPlan. Rephrasing those examples made the same command pass.
- Observation: The first full validation pass exited successfully after tests, but clippy emitted new `too_many_arguments` warnings in the lint traversal.
  Evidence: `cargo xtask validate` printed warnings for `walk_node`, `lint_inline_code_node`, and `lint_link_node`. The implementation was simplified with `FilePolicy` and `WalkState`, then `cargo xtask validate` passed with no clippy warnings.
- Observation: The repository root config migration works for whole-repo dogfooding, not only the targeted docs.
  Evidence: `cargo run -- lint . --color never` passed after migrating `docgarden.toml` from `[per-file-ignores]` to `[[documents]]` and `[[rules]]`.

## Decision Log

- Decision: Implement `[[documents]]` and `[[rules]]` for existing features only; do not implement context-budget, front matter, discovery commands, imported-reference policy, curated indexes, or generated guidance.
  Rationale: The user explicitly asked to implement the `docs/design-docs/configuration.md` design shape only for existing repo features. The current linter has only path/reference/style rules and related configuration, so future rule families would exceed scope.
  Date/Author: 2026-04-06 / Planner
- Decision: Keep the existing top-level keys and `[per-file-ignores]` backward compatible while adding the new shape.
  Rationale: Existing tests and users rely on `local-reference-style`, `report-ambiguous-inline-code`, `include`, `exclude`, extension settings, special-filename settings, `respect-gitignore`, and `[per-file-ignores]`. The new shape is an additive migration path rather than a breaking replacement.
  Date/Author: 2026-04-06 / Planner
- Decision: Treat a `[[rules]]` entry with `match = "<document family name>"` as a family-targeted entry when `<document family name>` matches a `[[documents]].name`; otherwise treat `match` as a gitignore-style path pattern.
  Rationale: The design asks whether rules should target family names, path patterns, or both. Supporting both gives immediate value while preserving the existing path-pattern behavior. Requiring family names to resolve avoids silently accepting typos in family-targeted config.
  Date/Author: 2026-04-06 / Planner
- Decision: Lower supported `[[rules]]` entries into the existing effective configuration fields instead of adding a separate rule engine.
  Rationale: The architecture says configuration affects classification and scope, while `src/lint/` consumes effective policy. Reusing `per_file_ignores`, `local_reference_style`, and `report_ambiguous_inline_code` keeps the implementation small and avoids a second rule execution path.
  Date/Author: 2026-04-06 / Planner

## Outcomes & Retrospective

The branch is complete and has been independently evaluated against `main`. The code now parses `[[documents]]` and `[[rules]]` for existing features, keeps legacy config working, dogfoods the new shape in the repository root config, and passes targeted validation plus `cargo xtask validate`. The latest round simplified the traversal call shape by adding `FilePolicy` and `WalkState` rather than adding another fix on top of the earlier implementation, and the evaluator found no blocking issues.

A second evaluator pass reviewed the staged closure branch on 2026-04-06. The review checked the implementation diff, confirmed the completed ExecPlan location, and reran `cargo test config`, `cargo test --test path_behavior`, `cargo test --test cli`, targeted doc linting for the changed documentation and completed ExecPlan, `cargo run -- lint . --color never`, and `cargo xtask validate`. All commands exited successfully, and no blocking findings remained.

## Context and Orientation

`docgarden` is a Rust CLI that lints repository-local Markdown references. The binary is built from `src/main.rs`, which delegates to `src/lib.rs` and then to `src/cli.rs`. `src/cli.rs` resolves targets, infers the repository root, loads `docgarden.toml` through `src/config.rs`, discovers Markdown files through `src/discover.rs`, and calls `src/lint/mod.rs::lint_file` for each file.

The current configuration parser lives in `src/config.rs`. `FileConfig` is the deserialized shape of `docgarden.toml`; `Config` is the effective shape used by the rest of the program. Today the effective config includes `include`, `exclude`, `per_file_ignores`, `local_reference_style`, `known_extensions`, `special_filenames`, `report_ambiguous_inline_code`, and `respect_gitignore`.

The existing rule names are string values in diagnostics. They are:

- `unresolved-local-path`
- `prefer-links-for-local-paths`
- `prefer-backticks-for-local-paths`
- `ambiguous-inline-code`

The term “document family” in this plan means a named group of files declared in `docgarden.toml` with `[[documents]]`. A family has at least `name` and `match`. For this plan, `kind` may be parsed for forward-compatible shape, but it must not change behavior because no existing rule needs it.

The term “rule application” means a `[[rules]]` table in `docgarden.toml` that targets either a document family name or a gitignore-style path pattern and then modifies existing lint behavior for that target. The supported fields for this plan are:

- `match`: required string target. It is either a `[[documents]].name` or a gitignore-style path pattern relative to the repository root.
- `disable`: optional array of existing rule names. These suppress matching diagnostics in the same way `[per-file-ignores]` does today.
- `enable`: optional array of existing rule names. For this plan, it only has behavior for `ambiguous-inline-code`, because that is the only existing opt-in rule. Unknown or currently unsupported enabled rule names must fail config loading with a clear error.
- `local-reference-style`: optional `backticks` or `links`. This overrides the repo-wide style for matching files.
- `reason`: optional string accepted for human review. It is not used by the linter yet.

This plan intentionally does not implement the `rule = "context-budget"`, `max-lines`, `max-tokens`, `severity`, or `enabled = false` examples from `docs/design-docs/configuration.md`, because context-budget is not an existing feature. The parser should reject unsupported fields through Serde’s default unknown-field behavior or through explicit validation so unsupported future-looking config does not appear to work.

## Plan of Work

First, update `src/config.rs` with deserialization structs for `[[documents]]` and `[[rules]]`. Keep the existing `FileConfig` fields. Add `documents: Vec<DocumentConfig>` and `rules: Vec<RuleConfig>` to the parsed config shape. Use Serde `rename` and `alias` attributes consistently with the current code so kebab-case is the documented form and snake_case remains accepted where existing tests already rely on it.

Second, introduce an effective rule-application model that remains small. Add a new effective field to `Config` only if needed by `src/lint/mod.rs`; otherwise lower `disable` entries into `per_file_ignores`, keep a per-path or per-pattern style override list for `local-reference-style`, and keep an `ambiguous_inline_code` enable map for scoped opt-in. The simplest acceptable implementation is:

- merge legacy `[per-file-ignores]` and `[[rules]].disable` into the same effective ignored-rules lookup;
- preserve top-level `local-reference-style` as the repository default;
- add a helper on `Config` or a small function in `src/lint/mod.rs` that computes the effective local reference style for a file by applying the most specific matching `[[rules]].local-reference-style` entry after the top-level default;
- compute whether `ambiguous-inline-code` is enabled for a file from the top-level `report-ambiguous-inline-code` boolean or from a matching `[[rules]].enable = ["ambiguous-inline-code"]` entry.

If multiple matching rule entries set different local reference styles for one file, use last matching entry wins. This is easy to explain, matches common configuration layering, and avoids inventing specificity sorting before the product needs it.

Third, implement family expansion. Build a map from `[[documents]].name` to `[[documents]].match`. When lowering a `[[rules]]` entry, if `match` equals a document family name, use that family’s `match` pattern. If the `match` value does not equal a family name, use it directly as a path pattern. Reject duplicate document family names. Reject missing or empty `match` values through typed parsing or validation. Reject empty `disable` entries and unknown rule names so typos fail fast.

Fourth, wire the effective values into linting. `src/lint/mod.rs::lint_file` already receives `Config` and the relative path before walking the AST. Use the relative path to compute ignored rules, effective style, and ambiguous-inline-code enablement before or during the AST walk. Keep diagnostics, fix behavior, and JSON output unchanged except where new configuration suppresses or enables an existing rule.

Fifth, add tests. Unit tests in `src/config.rs` should prove family expansion, duplicate-family rejection, unknown rule rejection, legacy compatibility, scoped ambiguous-inline-code enablement, and style override parsing. Integration tests in `tests/path_behavior.rs` or `tests/cli.rs` should prove real CLI behavior:

- a `[[documents]]` family plus `[[rules]] disable = ["unresolved-local-path"]` suppresses a broken link only inside that family;
- a path-pattern `[[rules]] disable = ["prefer-backticks-for-local-paths"]` suppresses style rewrites just like `[per-file-ignores]`;
- a `[[rules]] enable = ["ambiguous-inline-code"]` reports ambiguous inline code only for matching files;
- a repo-wide `local-reference-style = "backticks"` with a matching `[[rules]].local-reference-style = "links"` causes only matching files to prefer links while other files still prefer backticks;
- legacy `[per-file-ignores]` and `report-ambiguous-inline-code = true` tests continue to pass.

Sixth, update documentation. Update `README.md` with a compact configuration example that uses the new shape for existing features. Keep it clear that future rule-specific options shown in design docs are not implemented yet. Update the repository root `docgarden.toml` to use `[[documents]]` and `[[rules]]` where it makes the repository’s own ignores clearer, unless doing so would obscure compatibility testing.

## Concrete Steps

From the repository root, inspect the current working tree before editing:

    git status --short

Then edit:

- `src/config.rs` for parsed and effective configuration shape, validation, family expansion, and tests;
- `src/lint/mod.rs` only as needed to consume per-file effective style and ambiguous-inline-code enablement;
- `tests/path_behavior.rs` or `tests/cli.rs` for CLI behavior coverage;
- `README.md` for user-facing configuration examples;
- `docgarden.toml` only if the new shape can express the repository’s current policy more clearly;
- this ExecPlan after each meaningful stopping point.

Run targeted tests during development:

    cargo test config
    cargo test --test path_behavior
    cargo test --test cli

After documentation edits, run targeted doc linting for changed docs:

    cargo run -- lint README.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/exec-plans/completed/configuration-rule-application.md --color never

Before marking ready for evaluation, run:

    cargo xtask validate

Expected success means the test commands exit with status 0. For `cargo xtask validate`, expect formatting, clippy, coverage, cargo-machete, and cargo-deny to pass.

## Validation and Acceptance

Acceptance requires all of these independently verifiable behaviors:

1. A config like this suppresses an unresolved local path only in files matched by the `references` family:

        [[documents]]
        name = "references"
        match = "docs/references/**"

        [[rules]]
        match = "references"
        disable = ["unresolved-local-path"]
        reason = "Imported references may contain source-derived paths."

    A file under the references family with a broken local link must lint successfully for that rule, while a file such as `README.md` with the same broken local link must still fail with `unresolved-local-path`.

2. A config like this suppresses the style rule for `README.md` without suppressing unresolved local path diagnostics:

        local-reference-style = "backticks"

        [[rules]]
        match = "README.md"
        disable = ["prefer-backticks-for-local-paths"]
        reason = "README stays human-facing."

    A valid Markdown link in `README.md` that would normally be rewritten to backticks must not produce `prefer-backticks-for-local-paths`, but a broken local link in that same file must still produce `unresolved-local-path`.

3. A config like this enables ambiguous inline-code warnings only under `docs/`:

        [[rules]]
        match = "docs/**"
        enable = ["ambiguous-inline-code"]

    Inline code such as `` `crates/base_db` `` must report `ambiguous-inline-code` in a matching docs file but not in `README.md`.

4. A config like this applies a local reference style override only under `docs/`:

        local-reference-style = "backticks"

        [[rules]]
        match = "docs/**"
        local-reference-style = "links"

    A backticked existing path in a matching docs file must report `prefer-links-for-local-paths`, while the same backticked existing path in `README.md` must not report that rule under the repo-wide backticks default.

5. Existing legacy configuration still works. `[per-file-ignores]`, `report-ambiguous-inline-code = true`, `local-reference-style`, include and exclude patterns, extension settings, special filename settings, and `respect-gitignore` keep their current behavior.

6. Unsupported future config does not silently succeed. For example, `[[rules]] rule = "context-budget"` with `max-lines` must fail configuration parsing or validation because context-budget is out of scope for this implementation.

7. No future feature is implemented as part of this plan. The diff must not add token counting, front matter validation, discovery commands, imported-reference policy, curated-index validation, or generated guidance.

8. Validation commands pass:

        cargo test config
        cargo test --test path_behavior
        cargo test --test cli
        cargo run -- lint README.md docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/design-docs/frontmatter-driven-discovery-commands.md docs/exec-plans/completed/configuration-rule-application.md --color never
        cargo xtask validate

## Idempotence and Recovery

The implementation is additive and safe to retry. Re-running tests and lint commands should not mutate files. `docgarden fix` is not required for this plan. If an edit causes config parsing to fail broadly, restore the last known-good behavior by keeping legacy fields in `FileConfig`, then reintroduce `documents` and `rules` one validation case at a time.

Do not use destructive git commands to recover. If generated coverage or target artifacts change, ignore them unless they are tracked by git. Preserve unrelated user changes in the working tree.

## Artifacts and Notes

The plan was created after the design doc rename and context-budget example cleanup. At generator handoff, those edits were already part of the current tree state rather than uncommitted changes. The only uncommitted file shown by `git status --short` was this ExecPlan.

## Interfaces and Dependencies

No new crate dependency is expected. Use existing dependencies: `serde` for TOML deserialization, `toml` for config parsing, `anyhow` for validation errors, and `ignore::gitignore::GitignoreBuilder` through the existing pattern-matching approach.

At the end of implementation, `src/config.rs` must expose enough effective configuration for `src/lint/mod.rs` to answer these per-file questions:

- Which rule names are disabled for this relative path?
- What is the effective `LocalReferenceStyle` for this relative path?
- Is `ambiguous-inline-code` enabled for this relative path?

The names and visibility of helper structs are implementation details, but tests must cover the behavior above through `Config::load` and CLI runs.

Revision note: 2026-04-06 / Planner created this plan from the user request to implement `docs/design-docs/configuration.md` for existing features only, then proceed through generator implementation and evaluator review without prematurely moving the plan to completed. A second 2026-04-06 evaluator pass recorded the staged branch review evidence and corrected the integration-test command spellings so future readers run the intended Cargo targets directly.
