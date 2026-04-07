# Implement Explicit Context Budget Limits

Save this completed ExecPlan at `docs/exec-plans/completed/context-budget-limits.md`. The active version lived in repository history while the work was in progress and while the recorded finding was reopened.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

`docgarden` should help repositories keep agent-facing Markdown cheap to load in context. After this change, a repository can add explicit path-targeted budget rules to `docgarden.toml`, run `docgarden lint`, and get deterministic diagnostics when a Markdown file exceeds a configured line or token limit. This first version is intentionally explicit: it does not add built-in defaults for agent entry-point files, does not implement `docgarden init`, does not add `skills_dir`, and does not introduce named scope targets.

The visible proof is a small repository with a config entry such as the example below. If `README.md` has more than 10 tokens, `docgarden lint README.md --color never` reports an error with rule `max_tokens`, the observed token count, and the configured limit.

    [[rules]]
    path = "README.md"
    max_tokens = 10

## Progress

- [x] (2026-04-07 00:53Z) Created the initial decision-complete ExecPlan from the agreed context-budget workflow and design decisions.
- [x] (2026-04-07 01:02Z) Confirmed the modularization prerequisite passed evaluator review before this reopening.
- [x] (2026-04-07 01:06Z) Added and passed focused config and CLI tests for explicit `max_tokens`, explicit `max_lines`, entry-level severity, disable behavior, duplicate path entries, stale rejected shapes, and non-fixability.
- [x] (2026-04-07 01:06Z) Implemented context-budget configuration lowering, `tiktoken-rs` token counting, line counting, diagnostics, and file-level rule execution.
- [x] (2026-04-07 01:08Z) Updated `ARCHITECTURE.md`, `docs/PRODUCT.md`, `docs/design-docs/configuration.md`, and `docs/design-docs/context-budget-limits.md` to reflect explicit v1 context-budget behavior.
- [x] (2026-04-07 01:10Z) Ran focused tests, full validation, and doc linting. This plan is ready for evaluator review and remains active.
- [x] (2026-04-07 03:23Z) Reopened this plan after evaluator review recorded a blocking finding: budget rule disables were also lowered into global per-file ignores, preventing a later matching budget entry from re-enabling the same budget kind.
- [x] (2026-04-07 03:23Z) Added a focused CLI regression test for `max_tokens`, then `disable = ["max_tokens"]`, then a later matching `max_tokens` entry restoring the token budget.
- [x] (2026-04-07 03:23Z) Updated configuration lowering so `max_tokens` and `max_lines` disables remain in the ordered context-budget rule list and are not copied into global per-file ignores.
- [x] (2026-04-07 03:23Z) Ran focused tests, doc lint, `cargo test`, and `cargo xtask validate`; all passed after the reopened fix.
- [x] (2026-04-07 03:23Z) Clean-room evaluator re-reviewed the reopened fix and recorded no remaining findings.
- [x] (2026-04-07 03:23Z) Orchestrator closeout moved this plan back to completed after confirming the evaluator result.

## Surprises & Discoveries

- Observation: The existing configuration parser currently rejects future context-budget fields.
  Evidence: `src/config.rs` has a negative test where `rule = "context-budget"` and `max-lines = 500` fail parsing. This plan changes the accepted surface to snake_case `max_tokens` and `max_lines`, while continuing to reject stale wrappers and kebab-case fields.

- Observation: Adding `tiktoken-rs` required network access to update `Cargo.lock`.
  Evidence: the first `cargo check` failed to download the crates.io index because the sandbox could not resolve `index.crates.io`. Rerunning `cargo check` with escalation downloaded `tiktoken-rs v0.7.0` and updated the lockfile.

## Decision Log

- Decision: Implement only explicit path-targeted context budgets in v1.
  Rationale: The user selected explicit-only scope. Built-in agent entry-point defaults and generated init defaults would widen the feature into policy and setup UX before the core lint behavior exists.
  Date/Author: 2026-04-07 / Planner

- Decision: Do not add `rule = "context-budget"` or named target scopes.
  Rationale: The presence of `max_tokens` or `max_lines` already selects the budget check. A separate rule wrapper and a `scope` field would add ceremony and conflict with the current path-only configuration direction.
  Date/Author: 2026-04-07 / Planner

- Decision: Keep public TOML keys snake_case.
  Rationale: Recent configuration work standardized on snake_case keys such as `path_style`. Budget fields must follow the same convention and reject kebab-case variants.
  Date/Author: 2026-04-07 / Planner

- Decision: Use `max_tokens` and `max_lines` as diagnostic rule identifiers.
  Rationale: Matching rule ids to config fields makes `disable = ["max_tokens", "max_lines"]` obvious and lets users suppress token and line checks independently.
  Date/Author: 2026-04-07 / Planner

- Decision: Make severity entry-level and default explicit budget diagnostics to errors.
  Rationale: Explicit budget config should be CI-enforceable by default. If one `[[rules]]` entry contains both `max_tokens` and `max_lines`, its `severity` applies to both; users who want different severities for the same path can write two entries with the same `path`.
  Date/Author: 2026-04-07 / Planner

- Decision: Count the entire Markdown file in v1.
  Rationale: Counting the exact file text is simple, deterministic, and does not require front matter parsing policy. Skill-body-only counting can be reconsidered later when `skills_dir` and skill validation exist.
  Date/Author: 2026-04-07 / Planner

- Decision: Implement context budgets after lint-rule modularization.
  Rationale: `docs/exec-plans/completed/modularize-lint-rules.md` established the rule-module structure so context budgets can use a file-level rule hook rather than deepen the lint orchestration module.
  Date/Author: 2026-04-07 / Planner

## Outcomes & Retrospective

2026-04-07 implementation outcome: The modularization prerequisite is complete. The first context-budget implementation is in place with config parsing, effective budget lookup, token and line diagnostics, CLI tests, and documentation updates. Focused tests, `cargo test`, doc linting, and `cargo xtask validate` passed. This plan is ready for evaluator review.

2026-04-07 evaluator outcome: Passed clean-room evaluation against `main` at `3cd15b7b34972b6e5dfab0de4901c96e164d0a63`. Evidence reviewed included the plan itself, `ARCHITECTURE.md`, `src/config.rs`, `src/lint/mod.rs`, `src/lint/rules/file.rs`, `tests/cli.rs`, `tests/path_behavior.rs`, `cargo test`, `cargo run -- lint docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/PRODUCT.md ARCHITECTURE.md` plus the then-active copy of this plan, and `cargo xtask validate`. No blocking findings remained, and the solution stayed intentionally simple: explicit path-targeted budget rules, entry-level severity, file-level diagnostics, and no autofix.

2026-04-07 reopened-generator outcome: A later evaluator pass found that the first closeout missed an ordering bug in budget disables. The reopened implementation now keeps `max_tokens` and `max_lines` disables out of global per-file ignores so the ordered `context_budgets_for_path` logic can apply last matching entry wins per budget kind. Focused validation passed with `cargo test config` and `cargo test --test cli context_budget -- --nocapture`. Broader validation also passed with active-plan doc lint, `cargo test`, and `cargo xtask validate`.

2026-04-07 reopened evaluator and closeout outcome: A clean-room evaluator re-reviewed the reopened fix against `3cd15b7b34972b6e5dfab0de4901c96e164d0a63` and found no remaining blocking findings. Evidence reviewed included `src/config.rs`, `src/lint/mod.rs`, `src/lint/rules/file.rs`, `tests/cli.rs`, `cargo test config`, and `cargo test --test cli context_budget -- --nocapture`. The orchestrator accepted the finding as resolved and moved this plan back to completed.

## Context and Orientation

`docgarden` is a Rust CLI that lints repository Markdown. The command flow starts in `src/cli.rs`, loads `docgarden.toml` through `src/config.rs`, discovers Markdown files through `src/discover.rs`, and lints each file through `src/lint/mod.rs`. Diagnostics use the shared `Diagnostic` type in `src/diagnostics.rs`, where `severity` can be `error` or `warning`; the CLI already exits nonzero for errors and not for warnings.

Today `src/config.rs` accepts top-level keys such as `include`, `exclude`, `respect_gitignore`, and `path_style`. It also accepts path-targeted `[[rules]]` entries with `path`, `disable`, `enable`, `path_style`, and `reason`. The parser denies unknown fields. This work adds only three optional fields to `[[rules]]`: `max_tokens`, `max_lines`, and `severity`.

In this plan, a token means one piece returned by the `o200k_base` tokenizer from the `tiktoken-rs` crate. This is an approximation for agent context cost, not an exact count for every model vendor. A line means one logical text line in the complete Markdown file. The line-count implementation should count the exact file content, including front matter if present.

The plan depends on the lint modularization plan at `docs/exec-plans/completed/modularize-lint-rules.md`. That plan left a rule-module structure where file-level rules can evaluate a whole source file without adding a large new branch directly into `src/lint/mod.rs`.

## Plan of Work

First, complete the lint modularization work. Do not implement context-budget rules directly in the current monolithic lint module unless the modularization plan is explicitly changed by a planner. Once modularization is complete, add budget support as a file-level rule family in the resulting rule structure.

Update configuration parsing in `src/config.rs`. `RuleConfig` should accept `max_tokens: Option<usize>`, `max_lines: Option<usize>`, and `severity: Option<Severity>`, or a config-specific severity enum that lowers cleanly to `crate::diagnostics::Severity`. Reject zero limits with a clear parse/lowering error because a zero-line or zero-token Markdown budget is not useful for repository documents. Extend the internal effective configuration with a list of budget rule entries that preserves config order so later matching entries override earlier matching entries for the same budget kind.

The effective behavior is last matching entry wins per budget kind. For a file that matches two entries for `max_tokens`, use the later entry's limit and severity. For a file that matches one token entry and one line entry, apply both. For a file that matches a disabling entry such as the example below after an earlier budget entry, suppress that budget kind for that file.

    [[rules]]
    path = "docs/**"
    max_tokens = 1000

    [[rules]]
    path = "docs/references/**"
    disable = ["max_tokens"]
    reason = "References preserve source fidelity."

Add token counting with `tiktoken-rs` using `o200k_base`. Keep the first implementation simple: construct the tokenizer in the budget rule path and count the entire source string. If this becomes noisy or expensive during implementation, cache the tokenizer in a small helper, but do not add a broad global cache or asynchronous initialization.

Add file-level diagnostics. A `max_tokens` diagnostic should be emitted at line 1, column 1 when the counted token total is greater than the configured limit. A `max_lines` diagnostic should be emitted at line 1, column 1 when the counted line total is greater than the configured limit. The diagnostics are not fixable. Messages must include both the observed count and configured limit, for example:

    File has 17 tokens, which exceeds configured max_tokens = 10.
    File has 8 lines, which exceeds configured max_lines = 5.

Update docs after implementation. `docs/design-docs/context-budget-limits.md` and `docs/design-docs/configuration.md` should reflect the implemented explicit-only shape. If `ARCHITECTURE.md` has been updated by modularization, add the budget rule family in the new rule-module description without turning the architecture doc into an implementation log.

## Concrete Steps

Work from the repository root of this checkout.

1. Confirm modularization is completed or under evaluator review before starting this plan.

    git status --short
    ls docs/exec-plans/active

Expected result: this context-budget plan remains active, and the modularization plan has either moved to completed or has evaluator approval to proceed. If modularization still has blocking findings, stop and address that plan first.

2. Add failing tests for configuration.

    cargo test config

Expected pre-implementation result: tests for `max_tokens`, `max_lines`, `severity`, and stale rejected shapes fail because the parser does not yet accept or lower the new fields.

3. Add failing CLI tests for behavior.

    cargo test --test cli context_budget -- --nocapture

Expected pre-implementation result: tests fail because no budget diagnostics exist yet.

4. Implement config lowering and budget diagnostics, then run focused tests.

    cargo test config
    cargo test --test cli context_budget -- --nocapture

Expected result after implementation: focused config and CLI tests pass.

5. Run broader validation.

    cargo test
    cargo xtask validate

Expected result: the Rust test suite and repository validation pass. If `cargo xtask validate` reports only an expected coverage warning about an active untracked ExecPlan, record the warning in this plan and explain why it is non-blocking.

6. Dogfood lint changed docs.

    cargo run -- lint docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/exec-plans/completed/context-budget-limits.md --color never

Expected result: the command reports no documentation-path or style-policy errors.

## Validation and Acceptance

Acceptance requires all of the following behavior.

First, config parsing accepts `max_tokens`, `max_lines`, and `severity` inside `[[rules]]` entries with `path`. A config entry with `max_tokens = 10` produces an effective token budget for matching files. A config entry with `max_lines = 5` produces an effective line budget for matching files. If `severity` is omitted, diagnostics are errors. If `severity = "warn"`, diagnostics are warnings.

Second, config parsing rejects stale or unsupported shapes. The parser must reject `scope`, `rule = "context-budget"`, `max-tokens`, `max-lines`, and `enabled = false` in a `[[rules]]` entry.

Third, check-only linting reports over-budget files. A Markdown file with more tokens than its configured token limit reports a non-fixable `max_tokens` diagnostic at line 1, column 1 with the observed and configured counts in the message. A Markdown file with more lines than its configured line limit reports a non-fixable `max_lines` diagnostic at line 1, column 1 with the observed and configured counts in the message.

Fourth, severity affects exit status through the existing CLI semantics. A default `max_tokens` or `max_lines` error makes `docgarden lint` exit nonzero. A warning-only budget diagnostic prints as `warning` and does not make the command fail.

Fifth, disabling works per rule id. `disable = ["max_tokens"]` suppresses token budget diagnostics for the matching path without suppressing `max_lines`. `disable = ["max_lines"]` suppresses line diagnostics without suppressing `max_tokens`. `disable = ["max_tokens", "max_lines"]` suppresses both.

Sixth, duplicate path entries can split severity. If one `[[rules]]` entry for `README.md` sets `max_tokens` with default error severity and a later or separate entry for the same path sets `max_lines` with `severity = "warn"`, a file that exceeds both budgets reports one error and one warning. If two matching entries configure `max_tokens`, the later matching entry wins.

Seventh, no autofix is offered for budget diagnostics. Human-readable output must not mark these diagnostics as fixable, and `docgarden fix` must not rewrite files solely because they exceed budget limits.

## Idempotence and Recovery

All implementation steps are safe to retry. Config tests and CLI tests create temporary repositories and should not mutate tracked files. Adding `tiktoken-rs` will update `Cargo.toml` and `Cargo.lock`; if dependency resolution fails because the sandbox cannot reach the registry, rerun the necessary cargo command with escalation rather than editing the lockfile by hand.

If the budget implementation starts to require a broader targeting model, stop and hand the plan back to `$planner-execplan`. Do not add `scope`, `[[documents]]`, `rule = "context-budget"`, built-in agent entry-point defaults, `skills_dir`, or `docgarden init` behavior while executing this plan.

## Artifacts and Notes

Evaluator review note (2026-04-07): Re-reviewed the current `feat/contextBudget` PR branch against `main` at merge base `3cd15b7b34972b6e5dfab0de4901c96e164d0a63`. One blocking finding remains: budget rule disables are lowered into both the ordered context-budget rule list and the global per-file ignore list, so a later matching budget entry cannot re-enable that budget kind even though the plan requires last matching entry wins per budget kind. Evidence: `src/config.rs` adds every non-empty `disable` list to `per_file_ignores` before filtering budget rules into `context_budget_rules`, and `src/lint/mod.rs` suppresses any finding whose rule id is in the merged ignore set. A manual repro with `max_tokens = 1`, then `disable = ["max_tokens"]`, then later `max_tokens = 1` for `README.md` exited 0 with no diagnostic for a five-word file. Evidence reviewed: this completed plan; `git diff --name-status main...HEAD`; `src/config.rs`; `src/lint/mod.rs`; `src/lint/rules/file.rs`; `src/lint/rules/local_paths.rs`; `tests/cli.rs`; `ARCHITECTURE.md`; `docs/PRODUCT.md`; `docs/design-docs/configuration.md`; `docs/design-docs/context-budget-limits.md`; `cargo test`; the then-current doc-lint command for both completed plan copies; `cargo xtask validate`; and the manual repro using `/workspaces/dglint/target/debug/docgarden lint README.md --color never` from a temporary repository. The review also observed pre-existing uncommitted worktree edits in `.agents/skills/`, `AGENTS.md`, and `docs/EXECPLAN_PERSONAS.md`; those were outside the `main...HEAD` implementation diff for this context-budget plan.

Expected config examples:

    [[rules]]
    path = "README.md"
    max_tokens = 10

    [[rules]]
    path = "README.md"
    max_lines = 5
    severity = "warn"

    [[rules]]
    path = "docs/references/**"
    disable = ["max_tokens", "max_lines"]
    reason = "References preserve source fidelity."

Expected human-readable diagnostic examples:

    README.md:1:1  error  max_tokens
    File has 17 tokens, which exceeds configured max_tokens = 10.

    README.md:1:1  warning  max_lines
    File has 8 lines, which exceeds configured max_lines = 5.

Validation evidence collected during implementation:

    cargo check
    cargo test config
    cargo test --test cli context_budget -- --nocapture
    cargo test
    cargo run -- lint docs/design-docs/configuration.md docs/design-docs/context-budget-limits.md docs/PRODUCT.md ARCHITECTURE.md docs/exec-plans/completed/context-budget-limits.md docs/exec-plans/completed/modularize-lint-rules.md --color never
    cargo xtask validate

The first unprivileged `cargo check` after adding `tiktoken-rs` failed because the sandbox could not resolve `index.crates.io`; rerunning `cargo check` with approved network access downloaded `tiktoken-rs v0.7.0` and its transitive dependencies. The final `cargo xtask validate` passed with diff region coverage of 96.02 percent.

Reopened validation evidence collected on 2026-04-07:

    cargo test config
    cargo test --test cli context_budget -- --nocapture
    cargo run -- lint docs/exec-plans/completed/context-budget-limits.md docs/exec-plans/completed/modularize-lint-rules.md --color never
    cargo test
    cargo xtask validate

These focused tests passed after adding `context_budget_later_matching_limit_can_reenable_after_disable` in `tests/cli.rs` and changing `src/config.rs` so budget disables are not added to `per_file_ignores`.

Revision note: 2026-04-07 / Planner reopened this plan from completed status because evaluator review recorded a real blocking finding. The acceptance criteria already required last matching entry wins per budget kind; this revision restores the active path and records the generator fix needed to satisfy that criterion.

Evaluator review note (2026-04-07): Re-reviewed the current `feat/contextBudget` branch against `3cd15b7b34972b6e5dfab0de4901c96e164d0a63`. No blocking findings remained after checking `src/config.rs`, `src/lint/mod.rs`, `src/lint/rules/file.rs`, `tests/cli.rs`, and the targeted runs `cargo test config` and `cargo test --test cli context_budget -- --nocapture`. The budget disable path stays out of `per_file_ignores`, and the later `max_tokens` re-enable regression passes.

## Interfaces and Dependencies

Add the `tiktoken-rs` crate to the root `Cargo.toml` dependencies and use its `o200k_base` tokenizer for token counts. Do not add another Markdown parser.

Expose an internal effective budget representation from `src/config.rs`. One acceptable shape is a vector of entries containing `pattern`, optional `max_tokens`, optional `max_lines`, optional disabled rule ids, and severity. The exact type names may change, but `src/lint/` must be able to ask for the effective token and line budgets for one repository-relative file path after applying last-match-wins and disable behavior.

The lint layer should contain a file-level budget rule entry point. One acceptable shape after modularization is:

    pub(crate) fn evaluate_context_budget_rules(
        context: &FileRuleContext<'_>,
    ) -> Result<Vec<Finding>>

The context must provide the repository-relative file path, complete source text, effective config, and enough diagnostic construction support to emit line 1, column 1 findings. The rule must return findings through the same pipeline used by other rules so ignore handling, JSON output, and CLI exit behavior stay consistent.

Revision note: 2026-04-07 / Planner created this plan from the agreed context-budget workflow. It records the explicit-only v1 scope, path-only config, snake_case fields, entry-level severity, full-file counting, and modularization dependency before implementation starts.
