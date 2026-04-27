# AGENTS

## Step 0 (required before keyword-searching for documentation)
Run `cargo run -- match <query>` before using `rg`, `grep`, `find`, or agents to locate Markdown documentation, plans, repository guidance, or repository-local skills.
Use this when your first instinct is to search docs or guidance by keyword.
Run `cargo run -- ls --active-plans` to list active ExecPlans when continuing or checking current plan-driven work.
Do not repeat this step when the relevant file is already named by the user, listed in this file, or still in active context.
Do not use this step for code-first work, code symbol searches, test names, compiler errors, or known file paths; inspect and search code directly.

## Must Follow
- ALWAYS run `cargo run -- lint <changed-files> --color never` after modifying any `.md` file.
- ALWAYS treat `AGENTS.md` as a routing layer, not an encyclopedia. Keep repo-wide guidance here short and move long procedures or rationale into `docs/` or repository-local skills.
- ALWAYS treat `docs/` as the repository knowledge system of record for product, architecture, planning, testing, and operating-context guidance.
- ALWAYS use targeted test commands while iterating. Reserve `cargo xtask validate` for the final validation pass before handoff.
- ALWAYS address bug reports and review findings with TDD: reproduce the issue in a failing test first, then fix it and rerun the relevant tests until they pass.
- ALWAYS consult `docs/CODESTYLE.md` before adding handwritten validators, wrapper structs, parallel collections, or stringly-typed identifiers; it captures the recurring code smells this repo rejects at review.

## Repository Map
### Start Here for Architecture and Implementation
- `ARCHITECTURE.md` – Top-level code map, module boundaries, and architectural invariants for `docgarden`. Read this first when you need the current system boundaries, shared seams, or architectural intent.
- `src/` – Rust implementation for the `docgarden` CLI, matcher, and lint engine. Start here once the architecture docs have identified the area to change.
- `docs/design-docs/` – Deeper feature and policy rationale for specific subsystems. Use this when `ARCHITECTURE.md` is not enough and you need the design history behind a behavior or rule family.

### Start Here for Product and Repository Context
- `docs/` – Repository knowledge system of record, including product docs, plans, testing guidance, design docs, and references.
- `docs/PRODUCT.md` – Product overview for `docgarden`, focused on the agentic-engineering and repository-knowledge use case. Use this for product intent, target users, core workflows, and non-goals.
- `docs/TOOLS.md` – Environment and tooling guide for agents working in this repository. Start here when choosing local commands or checking runtime/tool availability.

### Start Here for Planning and Backlog
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `docs/exec-plans/` – Active and completed execution plans. Start here when continuing plan-driven work or checking how a change was scoped and implemented.

### Start Here for Testing and Validation
- `docs/TESTING.md` – Canonical testing workflow, TDD expectations, and validation guidance. Read this when adding features, fixing regressions, or deciding how to verify a change.
- `tests/` – Integration tests, fixture-backed regression coverage, and shared CLI harness code. Start here for bug repros, CLI behavior, repository-walking scenarios, and end-to-end validation.

### Start Here for Code Style and Quality
- `docs/CODESTYLE.md` – Rust coding conventions and prohibited code smells for the crate. Read this before introducing new structs, validators, parsers, or CLI flags, and when reviewing a refactor.
