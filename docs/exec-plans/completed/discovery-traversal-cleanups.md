---
description: "ExecPlan for tightening discovery to `.md` files only and removing avoidable matcher rebuild overhead in `src/discover.rs`; read when implementing traversal behavior for discovery commands built on the lint walker."
---

# Discovery Traversal Cleanups

## Goal
- Make repository discovery strictly `.md`-only and remove avoidable traversal overhead before `list` and `match` reuse the lint walker.

## Scope
- In: `src/discover.rs` cleanup for matcher reuse, strict `.md` filtering for walked entries and explicit file targets, and tests/docs needed to lock in the behavior.
- Out: implementing `list` or `match`, adding indexing or caching, changing frontmatter scoring, and broadening discovery beyond `.md`.

## Relevant Areas
- `src/discover.rs` — current traversal logic, explicit-target handling, and per-root matcher creation.
- `src/defaults.rs` — default include patterns that currently imply broader-than-`.md` discovery.
- `src/cli.rs` — existing target model and `--no-gitignore` behavior that discovery commands should reuse.
- `tests/cli.rs` — existing integration coverage for gitignore and explicit-target behavior; likely place for traversal acceptance tests.
- `docs/design-docs/frontmatter-driven-discovery-commands.md` — discovery design contract that now says `list` and `match` should reuse lint traversal and discover only Markdown docs.

## Open Questions
- None yet

## Steps
- [x] Decide and document the user-visible behavior for explicit non-`.md` file targets in discovery.
- [x] Refactor `src/discover.rs` so include/exclude matchers are constructed once per discovery run instead of once per directory target.
- [x] Add a shared `.md` path check and apply it to both walked entries and explicit file targets before they enter the discovered file set.
- [x] Update any default scan-pattern assumptions in `src/defaults.rs` and related tests so the effective discovery contract is `.md`-only.
- [x] Add targeted tests covering: `.gitignore` default behavior, `--no-gitignore`, explicit `.md` file targets, explicit non-`.md` targets, and directory traversal that contains non-`.md` files under `docs/`.
- [x] Update `docs/design-docs/frontmatter-driven-discovery-commands.md` only if implementation decisions change the current design wording.

## Validation
- `cargo test --test cli`
- `cargo test config`
- `cargo run -- lint docs/design-docs/frontmatter-driven-discovery-commands.md docs/exec-plans/active/discovery-traversal-cleanups.md --color never`
- Manual check: confirm the discovery path used for future `list` and `match` would return only `.md` files for the same targets and gitignore settings that `lint` sees.

## Discoveries
- Design doc required no changes; implementation aligns with the existing `.md`-only and reuse-lint-traversal contract.
- Replaced `frontmatter_non_md_files_unaffected_by_frontmatter_rules` test (which passed `.txt` as explicit target) with `explicit_non_md_target_fails_with_error` in `tests/cli.rs` and `discovered_set_includes_md_and_excludes_non_md` in `src/discover.rs`.
- Default scan patterns were simplified to `["*.md"]`; the narrower list is behaviorally equivalent because gitignore-style basename matching already covers Markdown files at any depth.


- `src/discover.rs` currently rebuilds `PatternMatcher` instances inside `discover_markdown_files_under`, so multiple directory targets repeat matcher compilation.
- `src/discover.rs` currently accepts explicit file targets without checking for a `.md` extension.
- `src/defaults.rs` currently uses `["docs/**", "README.md", "AGENTS.md", "*.md"]` as default scan patterns, which is broader than a strict `.md`-only discovery contract.
- Explicit non-`.md` file targets should fail with an error rather than being silently ignored.
- There should be no special-case discovery support for files without a `.md` extension; `.md` is the only Markdown indicator.

## Review
- [x] Test gap (2026-04-17): Added `discovered_set_includes_md_and_excludes_non_md` unit test in `src/discover.rs` that directly asserts the file set includes `docs/guide.md` and excludes `docs/notes.txt`.
- [x] Cleanup opportunity (2026-04-17): Extracted `is_markdown_path(&Path) -> bool` in `src/discover.rs`; both the walker loop and the explicit-target branch call it.
- [x] Cleanup opportunity (2026-04-17): Extracted `repository_relative_path` into `src/paths.rs`; both `src/discover.rs` and `src/lint/mod.rs` now import it from there, and the duplicate private helpers are removed.
