---
description: "Active ExecPlan for splitting local-path lint into syntax-specific link and backtick rules, removing ambiguous-inline and prefer-backticks behavior, and updating config and tests accordingly."
---

# Split Local Path Rules By Syntax

## Goal
- Replace the current mixed local-path rule set with a syntax-aware model where `unresolved-link-path` is enforced by default, `unresolved-backtick-path` is available only as an opt-in rule with configurable warning or error severity, `prefer-links-for-local-paths` is available only through explicit `[[rules]].enable`, and `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, and `path_style` are removed from product and config surfaces.

## Scope
- In: rule taxonomy changes, lint behavior changes in `src/lint/rules/local_paths.rs`, config-surface updates in `src/config.rs`, diagnostics-summary updates, tests and fixtures that mention removed or renamed rules, and design-doc / ExecPlan updates needed to reflect the new semantics.
- Out: cross-link rule implementation, wiki-link support, frontmatter changes, discovery-command work, or broader relationship-graph linting.

## Relevant Areas
- `src/lint/rules/local_paths.rs` — current inline-code and Markdown-link logic for unresolved-path and style findings.
- `src/config.rs` — accepted rule names, rule enable/disable behavior, path-style overrides, and current ambiguous-inline opt-in handling.
- `src/diagnostics.rs` — fixable-rule summaries and rule-name expectations in diagnostics output.
- `tests/path_behavior.rs` — current behavior coverage for unresolved paths, ambiguous inline code, and style-rule suppression.
- `tests/cli.rs` — CLI-visible rule-name and fix-summary assertions that currently mention removed rules.
- `docs/design-docs/configuration.md` — planned public config shape and examples for per-path rule control.
- `docs/design-docs/path-style-policy.md` — product-facing statement of what links versus backticks mean under the new posture.
- `docs/design-docs/backtick-path-classification.md` — current design document centered on ambiguous inline code and backtick-path heuristics that will need rewrite or retirement.

## Open Questions
- None yet

## Steps
- [ ] Update the design docs to describe the new semantic split: local Markdown links are validated by default as explicit repository references; backtick-path resolution is optional and configured per scope; `prefer-links-for-local-paths` and `unresolved-backtick-path` are enabled explicitly through `[[rules]].enable`; and `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, and `path_style` are removed.
- [ ] Revise `src/config.rs` so the accepted rule-name set becomes `unresolved-link-path`, `unresolved-backtick-path`, and `prefer-links-for-local-paths`; remove public support for `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, and `path_style`; and use ordinary `enable` plus entry-level `severity` when `unresolved-backtick-path` is turned on for a path.
- [ ] Refactor `src/lint/rules/local_paths.rs` so link nodes emit `unresolved-link-path` by default, inline backticks emit `unresolved-backtick-path` only when the rule is enabled for the file, and link-style rewrites emit `prefer-links-for-local-paths` only when that rule is explicitly enabled.
- [ ] Update diagnostics and fix-summary plumbing so only surviving fixable rules appear in summaries and removed rules disappear cleanly from machine-readable and human-readable output.
- [ ] Rewrite tests and fixtures in `tests/path_behavior.rs` and `tests/cli.rs` to cover the new defaults: broken local links fail by default, broken backtick paths do not report unless `unresolved-backtick-path` is enabled, enabled backtick-path checks honor configured severity, and `prefer-links-for-local-paths` appears only when explicitly enabled.
- [ ] Update design docs, completed-plan references, and any repo config examples that still mention `unresolved-local-path`, `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, or `path_style` so the repository guidance matches the shipped behavior. Discussion of these should be removed, do not reference that they were removed.

## Validation
- `cargo test config`
- `cargo test --test path_behavior`
- `cargo test --test cli`
- `cargo run -- lint . --color never`
- Confirm that a fixture with a broken local Markdown link reports `unresolved-link-path` without extra config.
- Confirm that the same broken target in backticks is silent by default and only reports once `unresolved-backtick-path` is enabled for the matching path.
- Confirm that enabling `prefer-links-for-local-paths` through `[[rules]].enable` produces fixable findings without any `prefer-backticks-for-local-paths` output.

## Discoveries
- `docs/exec-plans/active/` was empty when this plan was created, so this file is the active ExecPlan for the requested rule-taxonomy change.
- The current implementation emits `unresolved-local-path` for both inline backticks and links from `src/lint/rules/local_paths.rs`, and it emits `ambiguous-inline-code` plus `prefer-backticks-for-local-paths`; those names are also hard-coded in `src/config.rs`, `tests/path_behavior.rs`, `tests/cli.rs`, `src/diagnostics.rs`, and several design / completed-plan docs.
- `docs/design-docs/configuration.md` still describes disabling `unresolved-local-path`, enabling `ambiguous-inline-code`, and disabling both style rules together, so the design-doc update is part of the requested scope rather than a follow-up.
- Plan decision update: remove `path_style` entirely, require explicit `[[rules]].enable = ["prefer-links-for-local-paths"]` for link-style rewrites, and use existing `enable` plus entry-level `severity` to opt into `unresolved-backtick-path`.

## Review
- [ ] None yet
