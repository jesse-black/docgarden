# AGENTS

## Must Follow
- ALWAYS default to `cargo run -- match <query>` from the repository root when you need to route to the right repository document or agent skill for a task or topic. Use `rg` or other plain-text search only when you need exact text matches or broad body-text retrieval rather than metadata-based routing.
- ALWAYS run `cargo run -- lint <changed-files> --color never` from the repository root after updating documentation so required `description` frontmatter stays present for `match` routing and configured line/token budgets are enforced.
- ALWAYS treat `AGENTS.md` as a routing layer, not an encyclopedia. Keep repo-wide guidance here short and move long procedures or rationale into `docs/` or repository-local skills.
- ALWAYS treat `docs/` as the repository knowledge system of record for product, architecture, planning, testing, and operating-context guidance.
- ALWAYS use targeted test commands while iterating. Reserve `cargo xtask validate` for the final validation pass before handoff.
- ALWAYS address bug reports and review findings with TDD: reproduce the issue in a failing test first, then fix it and rerun the relevant tests until they pass.

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
