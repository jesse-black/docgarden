# AGENTS

## Repository Map
- `ARCHITECTURE.md` – High-level code map, module boundaries, and architectural invariants for `dglint`. *Use for system structure, rule-engine boundaries, and stable implementation invariants.*
- `docs/` – Repository knowledge system of record, including product documentation, execution plans, tool guidance, and supporting references.
- `docs/PRODUCT.md` – Product overview for `dglint`, focused on the agentic-engineering and repository-knowledge use case. *Use for product intent, target users, core workflows, and non-goals.*
- `docs/TOOLS.md` – Brief guide to tooling available to agents in this environment. *Use for environment capabilities and local tool discovery.*
- `docs/IDEAS.md` – Backlog of future `dglint` rule ideas and repository-knowledge checks that are intentionally not committed ExecPlan scope yet. *Use for plausible next rule families that are not yet scheduled work.*
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. *Use when creating, updating, or completing ExecPlans in `docs/exec-plans/`.*
- `src/` – Rust code for the `dglint` linter. *Use for implementation details once the relevant repository-knowledge doc has identified the area to change.*

## Documentation Guidance
- Treat `AGENTS.md` as a map, not an encyclopedia. Keep it short, stable, and routing-oriented, and move deeper guidance into `docs/` so agents can load context progressively.
- Treat `docs/` as the repository knowledge system of record. Important product, architecture, plan, and operating-context knowledge should live in versioned repository documents rather than external notes or ad hoc prompts.
- In this repository, live repository-local path mentions in prose should normally use backticked repo-relative paths such as `docs/PLANS.md`. Keep Markdown links for external destinations or for local references whose label adds meaning beyond repeating the path.
- When writing hypothetical repository paths or sample Markdown links that are examples rather than live references, prefer indented code blocks. For short inline hypothetical examples, plain inline code such as `` `example/path.md` `` is also acceptable.
- Keep live repository references in normal prose only when they are intended to resolve and be linted.
- After updating documentation, run `dglint` on the changed files before finishing so hypothetical examples and stale references are caught locally.

## Rust Workflow
- Format Rust code with `cargo fmt`.
- Run `cargo check` as the fast baseline compiler verification step.
- Lint Rust code with `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test` for the automated test suite.
- Run `cargo llvm-cov --summary-only` and keep coverage at or above 80% across the codebase before considering work complete.
