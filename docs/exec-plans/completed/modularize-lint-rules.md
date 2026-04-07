# Modularize Lint Rule Execution

This completed ExecPlan lives at `docs/exec-plans/completed/modularize-lint-rules.md`.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

`docgarden` can already detect and fix repository-local path problems, and the recent pipeline refactor improved correctness by making fixes operate from the same finding that powers diagnostics. The next user-facing gain is stability as the rule set grows. After this change, contributors should be able to add or change a rule without editing one large lint module or re-learning the whole traversal flow. A user should still see the same `docgarden lint` and `docgarden fix` behavior, but the implementation should be split into small rule modules with clear interfaces so new deterministic checks can be added safely.

The visible proof is behavioral parity plus easier extension. A novice should be able to run the existing CLI tests, see that current local-path behavior still works, then add a small rule-focused test in the new module layout without touching unrelated traversal code.

## Progress

- [x] (2026-04-02 00:00Z) Authored the initial ExecPlan in `docs/exec-plans/active/modularize-lint-rules.md`.
- [x] (2026-04-07 00:53Z) Refreshed this plan with the context-budget dependency: modularization now must leave a clear file-level rule path for `docs/exec-plans/active/context-budget-limits.md`.
- [x] (2026-04-07 00:54Z) Confirmed the regression contract already exists and passes: ignored style rules do not fix README links, multibyte rewrites are stable, and link labels are linted as one link node.
- [x] (2026-04-07 00:56Z) Extracted local-path rule evaluation into `src/lint/rules/local_paths.rs` and added the file-level rule hook in `src/lint/rules/file.rs`; `cargo check` and `cargo test lint:: --lib -- --nocapture` passed.
- [x] (2026-04-07 00:57Z) Preserved byte-accurate fix application, per-file ignore behavior, and current CLI output while moving logic into rule modules; `cargo test --test cli -- --nocapture` and `cargo test --test path_behavior -- --nocapture` passed.
- [x] (2026-04-07 01:00Z) Updated `ARCHITECTURE.md` for the new rule-module boundary and file-level hook; `cargo test`, targeted doc linting, and `cargo xtask validate` passed. This plan is ready for evaluator review.
- [x] (2026-04-07 01:07Z) Independent evaluator review passed against `main`; the plan is being moved to `docs/exec-plans/completed/`.

## Surprises & Discoveries

- Observation: the recent refactor already created a natural seam by introducing a `Finding` that contains both diagnostic payload and optional edit.
  Evidence: `src/lint/mod.rs` now routes both reporting and fix collection through one helper, which removed the bug where ignored rules could still be rewritten during `fix`.

- Observation: the most fragile part of the pipeline is not rule classification itself but source-span editing.
  Evidence: the current integration suite includes multibyte rewrite coverage in `tests/cli.rs`, which exists because parser offsets had to be treated as byte offsets rather than character counts.

- Observation: the current architecture document still says `src/lint/reporting.rs` respects per-file ignores, but the refactor moved that decision into `src/lint/mod.rs`.
  Evidence: `src/lint/reporting.rs` now only constructs `Diagnostic` values, while `src/lint/mod.rs` performs the ignore check before pushing diagnostics or edits.

- Observation: context-budget rules are a near-term downstream consumer and they are file-level rather than AST-node-local.
  Evidence: `docs/exec-plans/active/context-budget-limits.md` requires `max_tokens` and `max_lines` diagnostics for complete Markdown files at line 1, column 1. The modularization must not leave future rules with only node-level extension points.

- Observation: the initial concrete-step command that listed two `cargo test` filters in one invocation is not accepted by Cargo.
  Evidence: `cargo test fix_respects_rule_disable_for_readme_style_rules fix_handles_multibyte_text_before_rewrites_without_corruption -- --nocapture` failed with `unexpected argument`. Running the filters separately passed.

## Decision Log

- Decision: preserve one AST traversal and move rule logic behind a shared internal interface instead of creating multiple independent traversals.
  Rationale: the recent pipeline refactor fixed real drift between detection and fixing. Reintroducing parallel traversals would recreate that risk and make overlapping edit coordination harder.
  Date/Author: 2026-04-02 / Codex

- Decision: keep fix application centralized in `src/lint/mod.rs` even after rule extraction.
  Rationale: edit ordering, overlap detection, and file writing are cross-rule concerns. Centralizing them preserves the byte-accurate rewrite logic and keeps rule modules focused on producing findings.
  Date/Author: 2026-04-02 / Codex

- Decision: modularize by rule family, not by Markdown node type alone.
  Rationale: users and future contributors think in terms of rules such as `unresolved-local-path` or `prefer-backticks-for-local-paths`, not in terms of `InlineCode` versus `Link` dispatch. A rule-family layout makes new checks easier to place and test.
  Date/Author: 2026-04-02 / Codex

- Decision: include a simple file-level rule hook in the modularized lint pipeline.
  Rationale: the next planned rule family, context-budget limits, evaluates the complete Markdown source instead of one inline-code or link node. Adding the hook during modularization avoids immediately reopening the traversal design or deepening `src/lint/mod.rs`.
  Date/Author: 2026-04-07 / Planner

## Outcomes & Retrospective

The completed work keeps today’s local-path behavior unchanged while making room for future deterministic rule families. The local-path rule family now lives in `src/lint/rules/local_paths.rs`, and `src/lint/rules/file.rs` provides an empty file-level rule hook for the context-budget follow-up plan. `src/lint/mod.rs` still owns parsing, traversal, ignore handling, fix collection, and source rewriting. Focused unit tests, integration tests, targeted doc linting, and `cargo xtask validate` passed before closeout.

2026-04-07 evaluator outcome: Passed clean-room review against `main` with no blocking findings. Evidence reviewed: `docs/PLANS.md`; the completed plan text; `git diff` and source reads for `src/lint/mod.rs`, `src/lint/rules/local_paths.rs`, `src/lint/rules/file.rs`, and `ARCHITECTURE.md`; `cargo test fix_respects_rule_disable_for_readme_style_rules -- --nocapture`; `cargo test fix_handles_multibyte_text_before_rewrites_without_corruption -- --nocapture`; `cargo test ignored_style_rule_in_readme_still_lints_backticked_link_as_one_link -- --nocapture`; `cargo test --test cli -- --nocapture`; `cargo test --test path_behavior -- --nocapture`; `cargo test`; `cargo run -- lint docs/exec-plans/active/modularize-lint-rules.md docs/exec-plans/active/context-budget-limits.md ARCHITECTURE.md --color never`; and `cargo xtask validate`. The new file-level hook is present, local-path logic is isolated, and the architecture doc now explains the rule-module boundary.

## Context and Orientation

`docgarden` is a Rust command-line linter. It reads Markdown files, parses each file into an abstract syntax tree, walks that tree, decides whether repository-local references are valid, emits diagnostics, and optionally applies safe source-span rewrites. The main lint orchestration currently lives in `src/lint/mod.rs`. Path classification, resolution, and replacement rendering live in `src/lint/references.rs`. Diagnostic structs and ignore-pattern helpers live in `src/diagnostics.rs`. The small adapter in `src/lint/reporting.rs` turns source positions into final `Diagnostic` values.

The recent refactor introduced an important concept: a single rule evaluation produces a `Finding`, which contains both the diagnostic payload and an optional `Edit`. `src/lint/mod.rs` then decides whether to emit that finding, whether per-file ignore rules suppress it, and whether fix mode should enqueue the edit. This is the behavior that must be preserved.

Today `src/lint/mod.rs` still contains three responsibilities that will become harder to manage as the product grows. First, it performs AST traversal. Second, it contains the rule logic for inline-code and Markdown-link path checks. Third, it owns edit application and file rewriting. The modularization work in this plan should separate the second responsibility from the first and third without changing user-visible behavior.

In this plan, “rule module” means a Rust module responsible for evaluating one cohesive family of deterministic checks and returning findings through a shared interface. A “finding” means one possible lint result for one source location: rule id, message, severity, fixability, and an optional source rewrite. A “pipeline” means the fixed order of operations from parse, to traverse, to evaluate rule modules, to collect findings, to apply edits.

The key files that a novice will touch are `src/lint/mod.rs`, `src/lint/references.rs`, `src/lint/reporting.rs`, `src/diagnostics.rs`, `tests/cli.rs`, `tests/path_behavior.rs`, and `ARCHITECTURE.md`. New helper modules should stay under `src/lint/` so the rule engine remains discoverable from one directory.

## Plan of Work

Start by locking in the current contract with tests. Add or refine tests in `tests/cli.rs` and `tests/path_behavior.rs` so the modularization cannot accidentally split diagnostics from fixes again. At minimum, preserve coverage for three behaviors: a per-file ignored style rule must not produce an edit during `fix`, multibyte text before an edit must not corrupt the rewrite, and same-label link handling must still treat a Markdown link as one lint unit instead of descending into the label as a separate inline-code rule. If any of these behaviors are not directly asserted at the right layer, add focused tests before moving code.

Once the tests define the contract, introduce a shared internal rule surface under `src/lint/`. One acceptable shape is:

    src/lint/rules/mod.rs
    src/lint/rules/local_paths.rs

The shared interface should be simple enough for a novice to follow. A practical design is for traversal code in `src/lint/mod.rs` to build a lightweight node-rule context containing the `Config`, current file policy, current repository-relative file path, and current AST node, then ask the local-path rule module for zero or more `Finding` values. Keep the existing `Finding` and `Edit` concepts, even if their exact type names move.

Also include a lightweight file-rule context containing the `Config`, current file policy, repository-relative path, and complete source text. It does not need to have a real rule implementation in this plan, but the pipeline should call a file-rule entry point once per file before or after the AST traversal. The initial file-rule module may return an empty vector. This is the extension point that `docs/exec-plans/active/context-budget-limits.md` will use for `max_tokens` and `max_lines` without adding another large branch to `src/lint/mod.rs`.

Perform the extraction in small steps. First, move the current link and inline-code path logic into a dedicated local-path rule module while keeping `src/lint/mod.rs` responsible for traversal and `emit_finding`. Do not change behavior at the same time as the move. After that compiles and tests pass, decide whether to split further into smaller helpers inside the rule module. For example, one helper can handle unresolved paths and another style-policy rewrites, but they should still live under the same rule family because they share resolution and rendering helpers from `src/lint/references.rs`.

After the first extraction, clean up module boundaries. `src/lint/reporting.rs` should remain a pure adapter from source position to `Diagnostic`, or its responsibilities should be renamed clearly if that proves awkward. Avoid a half-state where ignore handling is described as “reporting” even though it actually belongs to finding emission. If the move reveals a better boundary, update names and documentation in the same change so the code and architecture text match.

Finally, update `ARCHITECTURE.md` to explain the new rule-module boundary precisely. The architecture document must describe where traversal lives, where node-level rule families live, where file-level rule families plug in, and where fix application stays centralized. If the implementation introduces a new internal interface or trait, define it in plain language in the architecture document and in this plan’s `Interfaces and Dependencies` section.

## Concrete Steps

Work from the repository root of this checkout.

1. Add or tighten tests before moving code.

    cargo test fix_respects_rule_disable_for_readme_style_rules -- --nocapture
    cargo test fix_handles_multibyte_text_before_rewrites_without_corruption -- --nocapture
    cargo test ignored_style_rule_in_readme_still_lints_backticked_link_as_one_link -- --nocapture

Expected result before code movement: these tests pass on the current branch and define the regression boundaries the refactor must preserve. If a missing contract is discovered, add a new failing test first and only then continue.

2. Introduce the internal rule-module structure and move the current local-path logic.

    cargo fmt
    cargo test lint:: --lib -- --nocapture

Expected result: the crate builds, unit tests still pass, and the new module structure compiles without changing CLI behavior.

3. Run focused integration tests while iterating on the extraction.

    cargo test --test cli -- --nocapture
    cargo test --test path_behavior -- --nocapture

Expected result: the refactor preserves existing behavior, including fix rewrites, ignored-rule behavior, and link-as-unit traversal.

4. Run the repository validation stack required for Rust work.

    cargo xtask validate

Expected result: formatting, clippy, coverage, dependency checks, and deny checks all succeed.

5. Lint the documentation changed by this plan and the architecture update.

    cargo run -- lint docs/exec-plans/active/modularize-lint-rules.md ARCHITECTURE.md --color never

Expected result: `docgarden` reports no unresolved local paths or style-policy violations in the updated documents.

## Validation and Acceptance

Acceptance is primarily about unchanged behavior plus improved internal structure.

First, the regression-defining tests for the current pipeline must still pass. A per-file ignored style rule must not create an edit during `fix`. A file containing multibyte text before a rewrite must still be rewritten without corruption. A Markdown link with inline-code label text must still be linted as one link node, not as both a link and a nested inline-code path.

Second, the end-to-end CLI behavior must remain unchanged for existing user workflows. Run `cargo test --test cli` and expect all tests to pass. In particular, `docgarden fix` must still rewrite link-style and backtick-style paths, preserve unrelated formatting, and allow a second `docgarden lint` pass to succeed.

Third, the module boundary must become observable in the source tree. A novice should be able to open `src/lint/mod.rs` and see traversal plus finding emission orchestration, then open a dedicated rule module under `src/lint/` and find the current local-path rule logic there. If the extraction is successful, adding another rule family should no longer require pasting large blocks into `src/lint/mod.rs`.

Fourth, the repository documentation must describe the new structure accurately. `ARCHITECTURE.md` should say where rule families live, where traversal lives, where file-level rules plug in, and where edits are applied.

Fifth, the modularization must support the next context-budget plan without another structural refactor. A file-level rule entry point should exist and be invoked once per linted file even if it initially returns no findings. The evaluator should be able to inspect the code and see where a future context-budget rule can receive the complete source text and return normal findings.

## Idempotence and Recovery

This work should be safe to repeat. Creating or moving Rust modules under `src/lint/` is additive and can be retried after `cargo fmt`. The recommended approach is small moves followed by tests, not one large rename. If a halfway extraction breaks compilation, restore the last compiling state by manually moving only the partially extracted logic back into `src/lint/mod.rs` and rerun the focused tests before proceeding. Do not weaken tests to make an awkward module split pass.

The final code path must remain idempotent for users. Running `docgarden fix` twice on the same file after the refactor must produce the same stable file contents as it does today.

## Artifacts and Notes

Expected high-level source-tree shape after the first successful milestone:

    src/lint/mod.rs
    src/lint/references.rs
    src/lint/reporting.rs
    src/lint/rules/file.rs
    src/lint/rules/mod.rs
    src/lint/rules/local_paths.rs

Expected ownership after modularization:

    src/lint/mod.rs: parse source, walk AST, invoke rule modules, collect findings, apply edits
    src/lint/rules/file.rs: evaluate file-level rule families; initially an empty hook for context-budget follow-up work
    src/lint/rules/local_paths.rs: evaluate unresolved-path, prefer-links, prefer-backticks, and ambiguous-inline-code findings
    src/lint/references.rs: classify and resolve repository-local path candidates, render replacement text
    src/lint/reporting.rs: convert finding payload plus source position into final Diagnostic values

An acceptable focused proof after the refactor is a short read of `src/lint/mod.rs` that shows no large blocks of path-rule-specific branching beyond dispatch into the rule module.

Validation evidence collected during implementation:

    cargo test fix_respects_rule_disable_for_readme_style_rules -- --nocapture
    cargo test fix_handles_multibyte_text_before_rewrites_without_corruption -- --nocapture
    cargo test ignored_style_rule_in_readme_still_lints_backticked_link_as_one_link -- --nocapture
    cargo test lint:: --lib -- --nocapture
    cargo test --test cli -- --nocapture
    cargo test --test path_behavior -- --nocapture
    cargo test
    cargo run -- lint docs/exec-plans/active/modularize-lint-rules.md docs/exec-plans/active/context-budget-limits.md ARCHITECTURE.md --color never
    cargo xtask validate

The first `cargo xtask validate` run failed at covgate because the new files were not visible to diff coverage and because an intermediate loop helper left uncovered changed regions in `src/lint/mod.rs`. Adding intent-to-add entries with `git add -N` and factoring finding emission through `emit_findings` resolved the coverage failure. The final validation passed with diff region coverage of 94.71 percent.

## Interfaces and Dependencies

Keep the implementation inside the existing crate and continue using the `markdown` crate already used by `src/lint/mod.rs`. Do not add a second Markdown parser. Reuse `crate::config::Config`, `crate::diagnostics::Diagnostic`, and the helpers in `src/lint/references.rs`.

By the end of this work, the lint layer should expose one explicit internal interface for rule evaluation. One acceptable shape is:

    pub(crate) struct NodeRuleContext<'a> {
        pub(crate) config: &'a Config,
        pub(crate) policy: FilePolicy,
        pub(crate) file: &'a str,
    }

    pub(crate) fn evaluate_local_path_rules(
        context: &NodeRuleContext<'_>,
        node: &Node,
    ) -> Result<Vec<Finding<'_>>>

For file-level rules, expose a similarly small context:

    pub(crate) struct FileRuleContext<'a> {
        pub(crate) config: &'a Config,
        pub(crate) policy: FilePolicy,
        pub(crate) file: &'a str,
        pub(crate) source: &'a str,
    }

    pub(crate) fn evaluate_file_rules(context: &FileRuleContext<'_>) -> Result<Vec<Finding<'_>>>

Another acceptable shape is a small trait:

    pub(crate) trait Rule {
        fn evaluate(&self, context: &NodeRuleContext<'_>) -> Result<Vec<Finding<'_>>>;
    }

The exact interface can change, but the final design must satisfy five constraints. It must keep one traversal, it must return findings that can feed both diagnostics and fixes, it must allow rule modules to stay focused on rule evaluation rather than file writing, it must provide a file-level rule hook for context budgets, and it must be simple enough that a novice contributor can add a new deterministic rule family by copying the local-path module pattern.

Revision note: Created this plan to turn the recent detection-to-fix pipeline refactor into a durable module boundary before more rule families accumulate inside `src/lint/mod.rs`.

Revision note: 2026-04-07 / Planner refreshed this plan before implementation to account for the new context-budget ExecPlan. The key addition is a file-level rule hook so `max_tokens` and `max_lines` can be added after modularization without another traversal redesign.
