# AGENTS

## Repository Map
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/IDEAS.md` – Backlog of future `dglint` rule ideas and repository-knowledge checks that are intentionally not committed ExecPlan scope yet.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `src/` – Rust code for the `dglint` linter.

## Documentation Guidance
- When writing hypothetical repository paths or sample Markdown links that are examples rather than live references, prefer indented code blocks. For short inline hypothetical examples, plain inline code such as `` `example/path.md` `` is also acceptable.
- Keep live repository references in normal prose only when they are intended to resolve and be linted.
- After updating documentation, run `dglint` on the changed files before finishing so hypothetical examples and stale references are caught locally.

## Rust Workflow
- Format Rust code with `cargo fmt`.
- Run `cargo check` as the fast baseline compiler verification step.
- Lint Rust code with `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test` for the automated test suite.
- Run `cargo llvm-cov --summary-only` and keep coverage at or above 80% across the codebase before considering work complete.
