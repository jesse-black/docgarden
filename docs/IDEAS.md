# Linter Ideas

These are candidate rule families and checks for `docgarden` beyond local file-reference style and path-resolution validation. They are grounded in this repository's agentic-engineering model: `AGENTS.md` is a short routing map, `docs/` is the repository knowledge system of record, and deeper context should be discoverable through cross-linked repository documents.

## Repository-Knowledge Checks

- Structure checks: verify `AGENTS.md` stays a short map and points into `docs/` rather than duplicating deep guidance. This should enforce the "table of contents, not encyclopedia" model described in `AGENTS.md`.
- Cross-link coverage: require key docs to point to their adjacent sources of truth. Architecture docs should reference related design docs, plans, or generated docs where appropriate, and index docs should not leave orphaned documents.
- Repo-map consistency: if `AGENTS.md` says a knowledge area exists under `docs/`, verify the directory or document is actually present.
- Unresolved-link repair suggestions: when a repository-local reference does not resolve, search the workspace in two stages before offering help. First, search for exact basename matches elsewhere in the tree. Second, if that is ambiguous or empty, use deterministic approximate string matching for filename candidates. Treat these as ranked repair suggestions first, not unconditional autofix. Keep this strictly algorithmic rather than model-based, and require a clear safety policy before promoting any subset to autofix.
- Nonportable machine-path detection: add a separate scanner and rule family for host-specific absolute paths such as `C:\...`, `C:/...`, `/Users/alice/...`, or `/home/bob/...`. Keep this separate from repository-local path classification so `unresolved-local-path` remains focused on workspace references. Start warning-only if adopted, because machine-local path examples may be intentional in some docs.

## Wiki Health Checks

- Routing-doc budget enforcement: extend context-budget and structure checks so high-traffic entrypoint docs such as `AGENTS.md`, configured narrative indexes, and skill main files stay compact enough for progressive disclosure instead of slowly turning into encyclopedias.
- Orphan-page detection: flag important documents within configured repository-knowledge families that are not reachable from the expected routing surface, such as a configured family index or another canonical entrypoint for that family. Treat `AGENTS.md` separately because agent harnesses may inject it automatically even when it is not cross-linked from repository prose.
- Knowledge-graph hygiene: detect weakly integrated documents such as pages with no inbound links, topic clusters with no overview page, or configured document-family members that are only reachable through raw filesystem traversal instead of the documented knowledge map.
- Chronological log conventions: explore optional checks for append-only repository logs or activity journals whose entries follow a parseable heading pattern so agents can inspect recent changes deterministically.
- Imported-reference policy: define document-family-aware rules for configured raw or reference directories so imported external material can require provenance front matter while selectively relaxing checks that are inappropriate for source-derived content. In this repository, `docs/references/` is one example of such a configured family.
- Query-artifact retention: explore whether durable outputs from agent work, such as comparison docs or synthesized analyses, should be linted as first-class repository knowledge instead of remaining only in chat history.
- Contradiction and staleness surfacing: investigate deterministic heuristics that can flag potentially superseded statements, duplicated facts across pages, or nearby documents that should be reviewed together after a major update.

## Front Matter And Config

- Front matter policy: continue the schema and required-field design work in `docs/design-docs/standardized-yaml-front-matter.md`, including which document families require front matter, which fields are linted for each family, and which files intentionally require none.
- Global ignore patterns: allow repositories to exclude whole paths from all linting, such as generated docs or imported third-party material.
- Per-rule exceptions: allow repositories to skip only selected rules for matching files. For example, `AGENTS.md` might still be checked for structure and path references while skipping frontmatter rules.

## ExecPlan Checks

- ExecPlan hygiene: enforce the required `Progress`, `Decision Log`, `Surprises & Discoveries`, and `Outcomes & Retrospective` sections from `docs/PLANS.md`, and verify active and completed plans live in the correct directories.
- Progress completeness: flag plans whose `Progress` section is stale, fully unchecked, or inconsistent with the rest of the document.
- Revision-log enforcement: require a revision note at the bottom of the plan whenever the plan changes materially.
