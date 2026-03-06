# AGENTS

## Repository Map
- `docs/` – Repository knowledge system of record, including design docs, references, generated docs, product specs, and execution plans.
- `docs/IDEAS.md` – Backlog of future `dglint` rule ideas and repository-knowledge checks that are intentionally not committed ExecPlan scope yet.
- `docs/PLANS.md` – Execution plan authoring and maintenance rules. Use this when creating, updating, or completing ExecPlans in `docs/exec-plans/`.
- `src/` – Rust code for the `dglint` linter.

## Documentation Guidance
- When writing hypothetical repository paths or sample Markdown links that are examples rather than live references, put them in indented code blocks so `dglint` can ignore them during dogfooding.
- Keep live repository references in normal prose only when they are intended to resolve and be linted.
