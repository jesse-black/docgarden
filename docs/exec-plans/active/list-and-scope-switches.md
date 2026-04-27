---
description: "ExecPlan for adding `docgarden list` plus configured scope switches for `list` and `match`."
---

# Add `docgarden list` and scope switches

## Goal

- `docgarden list` (alias `ls`) prints compact metadata rows for discovered Markdown documents.
- `docgarden list` supports positional file/directory targets, shallow directory listing by default, and `-R, --recurse` for recursive directory targets.
- `docgarden list` supports `--skills`, `--plans`, `--active-plans`, and `--completed-plans` scopes.
- `docgarden match` supports `--skills` and `--plans` scope switches that restrict ranked matching to configured document sets.
- Both commands keep the common default row shape: `path | name | description`.
- `docgarden list` omits documents without frontmatter `description` during directory and scope listing, while still rendering explicit file targets.

## Scope

- In:
  - Add top-level config fields for `skills_dir` and `plans_dir`, parsed from `docgarden.toml`.
  - Provide built-in defaults: `skills_dir = ".agents/skills"` and `plans_dir = "docs/exec-plans"`.
  - Add `list`/`ls` CLI wiring, help text, output rendering, and integration tests.
  - Add shared discovery/document-metadata helpers used by both `list` and `match`.
  - Keep `list` output as a metadata catalog by filtering undescribed documents from directory and scope listings.
  - Add mutually exclusive scope switches to `list`; reject scope switches combined with positional targets.
  - Add mutually exclusive `--skills` and `--plans` scope switches to `match`.
  - Preserve `match` ranking, `--limit`, `--path-only`, `--explain`, `--color`, and `--no-gitignore` behavior after scope filtering.
- Out:
  - JSON output.
  - Full-text body search.
  - `--active-plans` or `--completed-plans` for `match`.
  - Optional curated index files.
  - Skill validation beyond reading skill frontmatter from `SKILL.md`.
  - Scope labels in default output unless implementation finds an existing field model that makes them trivial and well-tested.

## Relevant Areas

- `docs/design-docs/match-and-list.md` — source of truth for command shape, scope semantics, output, and traversal behavior.
- `src/cli.rs` — add `List(ListArgs)`, `ls` alias, list flags, scope switches, and dispatch; extend `MatchArgs` with scope switches.
- `src/config.rs` — parse `skills_dir` and `plans_dir`; expose configured-or-default scope roots in repository-relative form.
- `src/discover.rs` — add shallow directory discovery for `list` while preserving shared include/exclude and gitignore behavior.
- `src/matching.rs` — factor document metadata loading and candidate construction so `match` can run over the full repo or scoped files.
- `src/frontmatter.rs` — continue using the shared parser for `name` and `description`.
- `src/paths.rs` — reuse repository-relative rendering and add small helpers only if needed for normalized scope roots.
- `tests/cli.rs` — integration coverage for CLI behavior and output contracts.
- `tests/discovery-repo/` — extend fixture with default-path skills, default-path plans, nested docs, and config override coverage.
- `src/data/default-config.toml` — add built-in `skills_dir = ".agents/skills"` and `plans_dir = "docs/exec-plans"` defaults.

## Open Questions

- None yet

## Steps

- [x] Add failing integration tests for `list --help`, root help showing `list`, `ls` alias parity, default row shape, shallow directory listing, and `-R, --recurse`.
- [x] Add failing integration tests for `list --skills`, `list --plans`, `list --active-plans`, `list --completed-plans`, scope mutual exclusion, and scope-plus-target rejection.
- [x] Add failing integration tests for `match --skills` and `match --plans`, including preservation of `--limit`, `--path-only`, `--explain`, and no-scope output compatibility.
- [x] Extend `tests/discovery-repo/` fixture files with default-path skills and plans, plus targeted config override coverage where needed.
- [x] Add `skills_dir` and `plans_dir` parsing to `Config`; default them to `.agents/skills` and `docs/exec-plans`; validate configured paths are non-empty relative paths under the repository root, or document and test the chosen absolute-path behavior.
- [x] Introduce a scope model, likely `enum Scope { Skills, Plans, ActivePlans, CompletedPlans }`, with helpers that resolve configured scope roots and produce clear errors for missing configuration.
- [x] Refactor document metadata extraction from `src/matching.rs` into a reusable helper that returns repository-relative path, display name, optional description, and absolute path.
- [x] Extend discovery so `list` can discover explicit Markdown files, shallow directory Markdown children, or recursive directory contents using the existing include/exclude and gitignore rules.
- [x] Implement `docgarden list` rendering with stable sorted output, pipe escaping, fallback filename-stem names, `--config`, `--no-gitignore`, `--color`, and target defaulting to `.` when no scope is selected.
- [x] Wire `list` scope switches to configured roots: skills enumerate `SKILL.md` files under `skills_dir`; plans enumerate Markdown under `plans_dir`; active/completed enumerate Markdown under `{plans_dir}/active` and `{plans_dir}/completed`.
- [x] Update `match` to accept `--skills` and `--plans`; restrict the discovered candidate set before scoring while keeping full scoped-corpus IDF deterministic.
- [x] Ensure the CLI rejects mutually exclusive scope switches for each command and keeps `match` without positional filesystem targets.
- [x] Add focused unit tests for any new scope parsing, path normalization, shallow discovery, and metadata helper behavior.
- [x] Update help text so both commands document columns, scope switches, traversal behavior, and target restrictions.
- [x] Run formatting and targeted tests; iterate until the new failing tests pass.
- [x] Run Markdown lint for the new ExecPlan and final repository validation before handoff.
- [x] Add tests and implementation for omitting undescribed documents from directory and scope listings while preserving explicit file output.

## Validation

- `cargo fmt --check`
- `cargo test list_help_documents_output_columns_and_flags --test cli`
- `cargo test list_alias_ls_works_identically_to_list --test cli`
- `cargo test list_directory_targets_are_shallow_without_recurse --test cli`
- `cargo test list_recurse_descends_into_nested_directories --test cli`
- `cargo test list_scope_switches_select_configured_sets --test cli`
- `cargo test match_scope_switches_restrict_ranked_corpus --test cli`
- `cargo test --test cli match_`
- `cargo test --lib`
- `cargo run -- lint docs/exec-plans/active/list-and-scope-switches.md --color never`
- `cargo xtask validate`

## Discoveries

- `match` is already implemented in `src/matching.rs` and uses `discover_markdown_files_for_targets(&config, &[repository_root])`, shared frontmatter parsing, and BM25F scoring from `src/score.rs`.
- Current `Config` does not yet expose `skills_dir` or `plans_dir`; scope switches need either new required config fields or documented defaults.
- Current discovery always recurses into directory targets; `list` needs an explicit shallow mode while preserving the existing recursive behavior for `lint` and `match`.
- Existing `match` output already matches the current design doc: default rows omit scores, `--explain` prints score diagnostics, and `--path-only` prints repository-relative paths.
- Decision closed: provide built-in defaults of `.agents/skills` for `skills_dir` and `docs/exec-plans` for `plans_dir`; config values override those defaults.
- `list` uses the same metadata row shape as `match`, but does not apply query-term highlighting; `--color` is accepted for CLI consistency.
- Missing configured scope directories produce an empty scoped result set; existing non-directory paths at configured scope roots report an error.
- Existing default `match` tests are sensitive to fixture metadata, so new scoped fixtures use unique beacon terms to avoid changing unscoped ranking behavior.
- Decision closed: `list` keeps fixed `path | name | description` columns and omits undescribed documents from directory and scope catalogs; explicit file targets still render for metadata inspection.

## Review

- [ ] None yet
