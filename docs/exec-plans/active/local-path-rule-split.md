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
- [x] Update the design docs to describe the new semantic split: local Markdown links are validated by default as explicit repository references; backtick-path resolution is optional and configured per scope; `prefer-links-for-local-paths` and `unresolved-backtick-path` are enabled explicitly through `[[rules]].enable`.
- [x] Revise `src/config.rs` so the accepted rule-name set becomes `unresolved-link-path`, `unresolved-backtick-path`, and `prefer-links-for-local-paths`; removed `LocalReferenceStyle`, `path_style`, `AmbiguousCodeEntry`, `LocalReferenceStyleOverride`; added `UnresolvedBacktickPathRule` and `PreferLinksRule`; use ordinary `enable` plus entry-level `severity` when `unresolved-backtick-path` is turned on for a path.
- [x] Refactor `src/lint/rules/local_paths.rs` so link nodes emit `unresolved-link-path` by default, inline backticks emit `unresolved-backtick-path` only when the rule is enabled for the file, and link-style rewrites emit `prefer-links-for-local-paths` only when that rule is explicitly enabled.
- [x] Updated `src/diagnostics.rs`, `src/lint/mod.rs`, and `src/lint/references.rs` so only surviving fixable rules appear in summaries and removed helpers disappear cleanly.
- [x] Rewrite tests and fixtures in `tests/path_behavior.rs`, `tests/cli.rs`, and `tests/config.rs` to cover the new defaults: broken local links fail by default, broken backtick paths do not report unless `unresolved-backtick-path` is enabled, enabled backtick-path checks honor configured severity, and `prefer-links-for-local-paths` appears only when explicitly enabled. Updated `tests/test-repos/backticks/` and `tests/test-repos/links/` fixtures.
- [x] Update design docs (`configuration.md`, `path-style-policy.md`, `backtick-path-classification.md`) and root `docgarden.toml` so the repository guidance matches the shipped behavior.

## Validation
- `cargo test config`
- `cargo test --test path_behavior`
- `cargo test --test cli`
- `cargo run -- lint . --color never`
- Confirm that a fixture with a broken local Markdown link reports `unresolved-link-path` without extra config.
- Confirm that the same broken target in backticks is silent by default and only reports once `unresolved-backtick-path` is enabled for the matching path.
- Confirm that enabling `prefer-links-for-local-paths` through `[[rules]].enable` produces fixable findings without any `prefer-backticks-for-local-paths` output.

## Discoveries
- The previous implementation emitted `unresolved-local-path` for both inline backticks and links from `src/lint/rules/local_paths.rs`, and emitted `ambiguous-inline-code` plus `prefer-backticks-for-local-paths`; those names were also hard-coded in `src/config.rs`, `tests/path_behavior.rs`, `tests/cli.rs`, `src/diagnostics.rs`, and several design docs.
- `docs/design-docs/configuration.md` described disabling `unresolved-local-path` and enabling `ambiguous-inline-code`; updated in scope.
- Plan decision: remove `path_style` entirely, require explicit `[[rules]].enable = ["prefer-links-for-local-paths"]` for link-style rewrites, and use existing `enable` plus entry-level `severity` to opt into `unresolved-backtick-path`.
- `Severity` enum needed `Copy` derived to allow `FilePolicy` to remain `Copy`; added.
- `label_equivalent` and `looks_path_adjacent` functions in `references.rs` were only used for removed rules; removed.
- `tests/test-repos/backticks/` fixture repurposed from `prefer-backticks-for-local-paths` to `prefer-links-for-local-paths`.

## Review
- [ ] Blocking finding (2026-04-16): later matching `enable` entries cannot re-enable `unresolved-backtick-path` or `prefer-links-for-local-paths` after an earlier matching `disable`. `src/config.rs` still lowers every non-budget `disable` into the global `per_file_ignores` set in `lower_rules`, and `src/lint/mod.rs` suppresses any finding whose rule id appears in that merged ignore set before considering rule-order semantics. Repro 1: a temp `docgarden.toml` with `README.md` entries `enable = ["unresolved-backtick-path"]`, then `disable = ["unresolved-backtick-path"]`, then later `enable = ["unresolved-backtick-path"]` plus a broken backtick path exits 0 with no diagnostic. Repro 2: the same pattern with `prefer-links-for-local-paths` plus an existing backtick path also exits 0 with no style finding. This repeats the same precedence bug previously fixed for context-budget disables and leaves path-scoped opt-ins unable to recover from broader disables.
- [ ] Blocking finding (2026-04-17): rule enable/disable evaluation is no longer consolidated. `src/config.rs` resolves matching `disable` entries for most rules through `per_file_ignores`, but resolves enabled local-path behavior through separate family-specific paths (`unresolved_backtick_path_rules`, `prefer_links_rules`, and the separate budget/frontmatter reducers). That split means rule precedence is not enforced uniformly and makes it easy for new rule families to drift into different semantics. The implementation should be consolidated so enable/disable handling for rules flows through one shared evaluation path, even if the final internal representation is decided later.
