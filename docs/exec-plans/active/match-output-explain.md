---
description: "Revise `docgarden match` output to hide raw scores by default, add matched-term highlighting, and introduce an `--explain` mode with relative ranking context and explain-only colors."
---

# Revise `match` Output And Explain Mode

## Goal
- `docgarden match` defaults to `path | name | description` with matched-query-term highlighting and no score column, while `docgarden match --explain` adds ranking diagnostics (`raw score`, `% of top`, and `matched_terms/query_terms`) plus explain-only colors derived from hybrid relative-plus-coverage rules.

## Scope
- In: default output contract changes for `match`, a new `--explain` flag, matched-term highlighting for normal text output, explain-mode color behavior, help text and design-doc updates, and integration/unit tests for the new rendering rules
- Out: changes to ranking order or BM25F math, JSON output, new output modes beyond `--explain`, `--path-only` contract changes, and repository-wide highlighting helpers outside `match`

## Relevant Areas
- `src/cli.rs` — add `--explain`, rewrite `match --help`, and document that raw score/color semantics move behind explain mode
- `src/matching.rs` — restructure row rendering, compute explain-only relative metrics, render matched-term highlighting, and gate color output to explain mode
- `src/score.rs` — expose any additional scoring metadata needed for explain mode, especially the strongest matched field signal if the current `first_field_hit` is not sufficient
- `tests/cli.rs` — replace score-first parsing helpers, add assertions for default output, explain output, highlighting, and explain-only color behavior
- `docs/design-docs/frontmatter-driven-discovery-commands.md` — update the shipped output contract and explain-mode guidance
- `docs/design-docs/scoring.md` — document that raw BM25F score is primarily an explain/debug signal and that color bands are relative in explain mode

## Open Questions
- `--explain` should print a header row naming each output column before the results.
- Hybrid explain-color thresholds should be tuned after implementation and dogfooding, once the new `% of top` and coverage outputs can be observed on real queries.

## Steps
- [ ] Add `--explain` to `MatchArgs` in `src/cli.rs`, document the default output as `path | name | description`, and describe explain-mode fields plus explain-only color behavior in `match --help`
- [ ] Refactor `src/matching.rs` row rendering so default mode emits three columns with no score, while `--explain` emits the expanded diagnostics alongside the existing document metadata
- [ ] Implement matched-term highlighting for visible text fields in default and explain modes, using the same normalized query-term set as scoring and skipping stopwords/path-only output
- [ ] Define explain-mode derived metrics: `raw score`, `% of top`, and `matched_terms/query_terms`; keep sorting on raw score unchanged
- [ ] Change any remaining field-priority presentation or tie-break behavior that still uses `name > path > description` so it instead uses `name > description > path`
- [ ] Replace the fixed score-band renderer with explain-only color selection based on hybrid relative-plus-coverage rules, and ensure default mode stays uncolored even when `--color always` is requested
- [ ] Rewrite `tests/cli.rs` helpers and assertions for the new default column layout, `--explain` layout, matched-term highlighting, explain-only colors, and unchanged `--path-only` behavior
- [ ] Update `docs/design-docs/frontmatter-driven-discovery-commands.md` and `docs/design-docs/scoring.md` to reflect the new output contract and the role of explain-mode diagnostics

## Validation
- `cargo test --test cli match`
- `cargo run -- match review`
- `cargo run -- match --explain review`
- `cargo run -- match --explain review against the active plan`
- `cargo run -- match --path-only scoring`
- `cargo run -- match --help`
- `cargo run -- lint docs/exec-plans/active/match-output-explain.md`

## Discoveries
- `tests/cli.rs` currently parses `match` output as four positional columns with the raw score first, so the default-output contract change will require replacing the shared parser and the existing color assertions rather than updating only help text.
- `src/matching.rs` currently couples score rendering and color-band selection directly to default row rendering, so `--explain` is the natural place to move both score display and colorization.
- The strongest-matched-field column is intentionally omitted from `--explain`; if field-level introspection becomes necessary later, it should likely be a fuller debug breakdown rather than a single ambiguous label.
- Where field priority is still surfaced for `match`, the intended order is `name > description > path`, not the current `name > path > description`.

## Review
- [ ] None yet
