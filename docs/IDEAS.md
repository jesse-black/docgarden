# Linter Ideas

These are candidate rule families and checks for `docgarden` beyond local file-reference style and path-resolution validation. They are grounded in this repository's agentic-engineering model: `AGENTS.md` is a short routing map, `docs/` is the repository knowledge system of record, and deeper context should be discoverable through cross-linked repository documents.

## Repository-Knowledge Checks

- Structure checks: verify `AGENTS.md` stays a short map and points into `docs/` rather than duplicating deep guidance. This should enforce the "table of contents, not encyclopedia" model described in `AGENTS.md`.
- Cross-link coverage: require key docs to point to their adjacent sources of truth. Architecture docs should reference related design docs, plans, or generated docs where appropriate, and index docs should not leave orphaned documents.
- Repo-map consistency: if `AGENTS.md` says a knowledge area exists under `docs/`, verify the directory or document is actually present.
- Unresolved-link repair suggestions: when a repository-local reference does not resolve, search the workspace in two stages before offering help. First, search for exact basename matches elsewhere in the tree. Second, if that is ambiguous or empty, use deterministic approximate string matching for filename candidates. Treat these as ranked repair suggestions first, not unconditional autofix. Keep this strictly algorithmic rather than model-based, and require a clear safety policy before promoting any subset to autofix.
- Nonportable machine-path detection: add a separate scanner and rule family for host-specific absolute paths such as `C:\...`, `C:/...`, `/Users/alice/...`, or `/home/bob/...`. Keep this separate from repository-local path classification so `unresolved-local-path` remains focused on workspace references. Start warning-only if adopted, because machine-local path examples may be intentional in some docs.

## Front Matter And Config

- Front matter policy: continue the schema and required-field design work in `docs/design-docs/standardized-yaml-front-matter.md`, including which document families require front matter, which fields are linted for each family, and which files intentionally require none.
- Global ignore patterns: allow repositories to exclude whole paths from all linting, such as generated docs or imported third-party material.
- Per-rule exceptions: allow repositories to skip only selected rules for matching files. For example, `AGENTS.md` might still be checked for structure and path references while skipping frontmatter rules.
- Ignore-config readability: consider replacing context-poor `[per-file-ignores]` string-to-array mappings with self-describing array-of-tables entries such as a path field, ignored-rules field, and optional reason. This would make config diffs easier to understand when a reviewer sees only the changed entry without the section header.
    ```
    [[ignore]]
    path = "docs/references/**"
    ignore_rules = ["unresolved-local-path"]
    reason = "Imported external reference docs contain hypothetical paths from source material."
    ```

## ExecPlan Checks

- ExecPlan hygiene: enforce the required `Progress`, `Decision Log`, `Surprises & Discoveries`, and `Outcomes & Retrospective` sections from `docs/PLANS.md`, and verify active and completed plans live in the correct directories.
- Progress completeness: flag plans whose `Progress` section is stale, fully unchecked, or inconsistent with the rest of the document.
- Revision-log enforcement: require a revision note at the bottom of the plan whenever the plan changes materially.
