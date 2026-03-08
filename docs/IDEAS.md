# Linter Ideas

These are candidate rule families and checks for `dglint` beyond local file-reference style and path-resolution validation. They are grounded in the repository-knowledge model described in `docs/repository-knowledge/`.

## Repository-Knowledge Checks

- Structure checks: verify `AGENTS.md` stays a short map and points into `docs/` rather than duplicating deep guidance. This follows the "table of contents, not encyclopedia" model in `docs/repository-knowledge/repository-knowledge-system.md`.
- Cross-link coverage: require key docs to point to their adjacent sources of truth. Architecture docs should reference related design docs, plans, or generated docs where appropriate, and index docs should not leave orphaned documents.
- Index completeness: if a directory is intended to be catalogued, ensure every expected document is listed in its index and every indexed document exists.
- Freshness markers: check for stale generated-doc references, missing verification status, or docs that claim to be system-of-record material but show no visible maintenance signal.
- Repo-map consistency: if `AGENTS.md` says a knowledge area exists under `docs/`, verify the directory or document is actually present.
- Exception auditing: if the repository allows link exceptions or backtick exceptions, require them to be explicit and locally justified instead of ad hoc.
- Unresolved-link repair suggestions: when a repository-local reference does not resolve, search the workspace in two stages before offering help. First, search for exact basename matches elsewhere in the tree. Second, if that is ambiguous or empty, search with fuzzy filename matching. Treat these as ranked repair suggestions first, not unconditional autofix, because the tool is inferring intent rather than applying a deterministic rewrite. If promoted later, require a clear safety policy such as "only autofix when exactly one high-confidence candidate exists."
- Nonportable machine-path detection: add a separate scanner and rule family for host-specific absolute paths such as `C:\...`, `C:/...`, `/Users/alice/...`, or `/home/bob/...`. Keep this separate from repository-local path classification so `unresolved-local-path` remains focused on workspace references. Start warning-only if adopted, because machine-local path examples may be intentional in some docs.
- Rich terminal presentation: after the basic `--color auto|always|never` support lands, consider richer human-readable output such as clickable hyperlinks in supported terminals, grouped rule summaries, compact and verbose display modes, and a more intentional visual layout. Keep this separate from the initial implementation so terminal styling does not distort the core linting contract.
- Optional themed output: if open-source adopters want more ergonomic terminal output later, consider user-selectable themes or style presets for human-readable diagnostics. This should stay optional and must never affect JSON or other machine-readable output.

## Ownership And Freshness Metadata

- Minimum viable metadata: start with just `owner` and `last_reviewed`. That is enough for doc-gardening workflows because it answers who should maintain a document and whether someone has reviewed it recently.
- Standardized frontmatter: if metadata is stored in Markdown frontmatter, keep the required schema as small as possible at first rather than forcing a large universal schema.
- Ownership checks: require `owner` to be present and non-empty for documents that are expected to stay current.
- Freshness checks: require `last_reviewed` to be a valid date and flag docs whose review timestamp is too old according to repository policy.
- Token-efficiency exceptions: allow high-traffic, context-sensitive files such as `AGENTS.md` to skip frontmatter requirements when token efficiency matters more than metadata uniformity.
- Rule-level configurability: support both file-level ignore patterns and per-rule ignore patterns. This should behave more like `.gitignore` than hard-coded special cases.
- Global ignore patterns: allow repositories to exclude whole paths from all linting, such as generated docs or imported third-party material.
- Per-rule exceptions: allow repositories to skip only selected rules for matching files. For example, `AGENTS.md` might still be checked for structure and path references while skipping frontmatter rules.

## ExecPlan Checks

- ExecPlan hygiene: enforce the required `Progress`, `Decision Log`, `Surprises & Discoveries`, and `Outcomes & Retrospective` sections from `docs/PLANS.md`, and verify active and completed plans live in the correct directories.
- Progress completeness: flag plans whose `Progress` section is stale, fully unchecked, or inconsistent with the rest of the document.
- Revision-log enforcement: require a revision note at the bottom of the plan whenever the plan changes materially.

## Architecture-Doc Checks

- Architecture-doc rules: prefer naming important files, modules, and types without brittle deep links to source lines, following the guidance in `docs/repository-knowledge/architecture-md.md`.
- Architecture content checks: require sections that cover the bird's-eye view, code map, invariants, boundaries, and cross-cutting concerns for docs that claim to be architecture documents.
- Boundary-language checks: flag architecture docs that drift into low-level implementation detail instead of answering "where does X live?" and "what are the invariants?".

## Rust Tooling

- `cargo-deny` integration: add dependency-policy, license, and source checks once the dependency graph stabilizes and CI is ready to enforce them.
- `cargo-audit` integration: add vulnerability scanning for published crates and transitive dependencies as part of release or CI hygiene.
- `cargo-nextest` adoption: consider switching the test runner from `cargo test` to `cargo nextest` once the integration-test matrix grows enough that speed, retries, and richer CI reporting matter.

## Priority Candidates

If these ideas are implemented incrementally, the strongest next rule families for this repository are:

1. `knowledge-map-consistency`
2. `exec-plan-shape`
3. `architecture-doc-shape`
