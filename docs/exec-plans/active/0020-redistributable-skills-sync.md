---
description: "Plan for adding redistributable source skills under `skills/`, syncing generated `.agents/skills` copies with repository-local command forms, and keeping match routing unambiguous."
---

# Redistributable Skills Sync

## Goal
- `skills/` is the redistributable source-of-truth skills directory, starting with `skills/description-frontmatter-authoring/`.
- Synced `.agents/skills/description-frontmatter-authoring/` remains usable inside this repository with `cargo run -- <subcommand> ...` command forms.
- `docgarden match` routes agents to the live `.agents/skills` copy, while lint and sync validation still cover source and generated skill content.

## Scope
- In: source skill copy, generated local skill sync, xtask sync command, match-only exclusion config, tests, and repository config updates.
- Out: publishing/packaging automation beyond checked-in `skills/`, migrating every existing `.agents/skills` skill, and changing external skill installation behavior.

## Relevant Areas
- `skills/` — new redistributable source root.
- `.agents/skills/description-frontmatter-authoring/` — generated local copy with repository-local commands.
- `xtask/src/main.rs` — add `sync-skills` and `sync-skills --check`.
- `xtask/Cargo.toml` — add dependencies needed for Markdown-aware command rewriting tests.
- `src/lint/mod.rs` — existing reverse byte-edit application pattern to reuse for safe Markdown rewrites.
- `docgarden.toml` and `src/data/default-config.toml` — configure skills directory and match-only exclusions without weakening lint coverage.
- `src/config.rs`, `src/discover.rs`, `src/matching.rs` — load and apply match-only exclusion patterns.
- `docs/design-docs/configuration.md` — source of truth for the `[match].exclude` public config shape.
- `tests/cli.rs` and xtask unit tests — cover match-only exclusion, lint coverage, and sync behavior.

## Open Questions
- None yet.

## Steps
- [ ] Add `skills/description-frontmatter-authoring/` by copying the current skill and changing command examples from `cargo run -- <subcommand> ...` to `docgarden <subcommand> ...`.
- [ ] Keep `.agents/skills/description-frontmatter-authoring/` as generated output whose content matches `skills/description-frontmatter-authoring/` after command translation.
- [ ] Add `cargo xtask sync-skills` to copy every skill directory present under `skills/` into `.agents/skills/`, preserving unrelated repo-local `.agents/skills/*` directories.
- [ ] Add `cargo xtask sync-skills --check` to compare expected generated output with the working tree and fail with stale paths when generated files differ or are missing.
- [ ] Implement Markdown command translation using the same strategy as `docgarden fix`: collect byte-offset edits from parsed Markdown nodes, reject overlapping edits, sort edits in reverse order, then apply replacements.
- [ ] Translate shell command forms only, not product references: inline or fenced command text beginning with `docgarden <subcommand> ...` becomes `cargo run -- <subcommand> ...`; bare conceptual references such as ``docgarden match`` remain unchanged.
- [ ] Add xtask tests for inline command translation, fenced shell command translation, conceptual ``docgarden match`` preservation, stale generated-file detection, and preservation of unrelated `.agents/skills` directories.
- [ ] Add command-specific match exclusions with this public TOML shape: `[match]` plus `exclude = ["skills/**"]`.
- [ ] Apply `[match].exclude` only to the unscoped `docgarden match <query>` corpus after normal Markdown discovery; do not apply it to `docgarden lint`, `docgarden fix`, `docgarden list`, `docgarden match --skills`, or `docgarden match --plans`.
- [ ] Keep `.agents/skills` as configured `skills-dir`, exclude redistributable source `skills/**` from broad `docgarden match`, and keep both `skills/` and `.agents/skills/` visible to `lint`.
- [ ] Add config/unit coverage for `[match].exclude` defaults, parsing, and unknown-key rejection.
- [ ] Add integration coverage showing `docgarden match <query>` excludes redistributable source `skills/**` while still surfacing live `.agents/skills/**`, `docgarden match --skills <query>` uses configured live `.agents/skills`, `docgarden match --plans <query>` is not narrowed by `[match].exclude`, and `docgarden lint . --color never` still checks both source and generated skill files.
- [ ] Update root `docgarden.toml` to keep `skills-dir = ".agents/skills"` and set `[match].exclude = ["skills/**"]`.
- [ ] Keep `src/data/default-config.toml` default `skills-dir = ".agents/skills"` for ordinary repositories because `docgarden match --skills` should route to live skills usable by the agent.

## Validation
- `cargo test -p xtask sync_skills`
- `cargo test config::tests::match_exclude_parses_and_defaults`
- `cargo test --test cli match_only_exclusion_filters_source_skills`
- `cargo test --test cli match_skills_scope_uses_live_agent_skills_dir`
- `cargo test --test cli match_scopes_ignore_match_only_exclusion`
- `cargo test --test cli lint_still_scans_source_and_generated_skills`
- `cargo run -- lint skills/description-frontmatter-authoring/SKILL.md .agents/skills/description-frontmatter-authoring/SKILL.md docgarden.toml docs/design-docs/configuration.md --color never`
- `cargo run -- lint docs/exec-plans/active/0020-redistributable-skills-sync.md docs/design-docs/configuration.md --color never`
- `cargo xtask sync-skills --check`
- `cargo xtask validate`

## Discoveries
- `src/lint/mod.rs::apply_edits` already implements the safe rewrite pattern needed by the sync translator: reverse byte-offset replacements with overlap detection.
- Current top-level `exclude` in `docgarden.toml` is global discovery configuration used by both lint/fix and match; using it for either skill copy would weaken lint coverage or hide the source-of-truth path.
- `src/scopes.rs` discovers `--skills` from `Config::skills_root()` and filters to `SKILL.md`; keeping `skills-dir = ".agents/skills"` makes scoped matching/listing point at the live generated skills the agent can use.
- Existing `.agents/skills` contains repo-local skills that are not part of the first redistributable sync set, so sync must preserve unrelated destination directories.
- `docs/design-docs/configuration.md` now specifies `[match].exclude` as command-specific selection for broad unscoped matching; it does not apply to lint, fix, list, or explicit match scopes.
- Match-only exclusion should hide redistributable source `skills/**`, not live `.agents/skills/**`, because `docgarden match` is a routing tool for the skills available to the current agent.

## Review
- [ ] None yet.

## Definition of Done

### Planner
- [x] Plan is consistent, up to date, decision-complete, and ready to hand off.

### Generator
- [ ] Goal achieved: `skills/` is source of truth, `.agents/skills` generated copies stay synchronized with translated commands, and match routes to live generated skills while lint covers both trees.
- [ ] All planned steps are complete.
- [ ] All validation commands pass.
- [ ] Handed off to an independent reviewer (MUST use the `evaluator-execplan` skill via a subagent or separate agent, not the generator agent).

### Evaluator
- [ ] Pass 1: Cold review completed.
- [ ] Pass 2: Context review completed:
    - [ ] Adheres to the principles of `docs/CODESTYLE.md`.
    - [ ] Adheres to the principles of `docs/TESTING.md`.
- [ ] All review findings have been addressed.
