---
description: "Embed a default lint config so `docgarden lint` applies sensible rules when no `docgarden.toml` is present in the target repository."
---

# Embed default lint config

## Goal

When `docgarden lint` runs in a tree with no `docgarden.toml` (and no `--config`), apply an embedded default `FileConfig` that mirrors this repo's own `docgarden.toml` minus its repo-specific `exclude = ["tests/**"]`. Any user-provided config file (even empty) fully replaces the default.

## Scope

- In:
  - `src/data/default-config.toml` — new embedded asset; content = `docgarden.toml` minus `exclude`
  - `src/config.rs` — replace the no-config `FileConfig::default()` fallback with `include_str!` + `toml::from_str` of the embedded file
  - Two tests whose assertions assumed "no rules when no config": `load_ignores_nested_config_when_root_config_is_absent` and `config_debug_reports_stable_summary_fields`
  - New unit test `load_applies_embedded_default_when_no_config_found`
  - Integration test `git_root_is_used_when_no_docgarden_toml_is_found` updated so its fixture satisfies the embedded default's `description` rule
  - New integration test `lint_applies_embedded_default_when_no_config_found`
- Out:
  - Merge/layering semantics (user config always fully overrides, no base-then-extend)
  - `--no-default-config` CLI flag (empty `docgarden.toml` is the escape hatch)
  - Changes to the repo's own `docgarden.toml`
  - Changes to `src/defaults.rs` constants (extensions, special filenames, scan patterns)

## Relevant Areas

- `src/config.rs:226-236` — no-config branch of `Config::load`; the only call site changed
- `src/data/` — existing embedded-asset directory; `stopwords_en.txt` loaded via `include_str!` in `src/score.rs:165` is the pattern to follow
- `docgarden.toml` — source content for the embedded default (kept unchanged; `exclude` is omitted from the asset)
- `tests/cli.rs:860` — `git_root_is_used_when_no_docgarden_toml_is_found`; fixture `docs/guide.md` will need `description` frontmatter
- `src/config.rs:529` — `load_ignores_nested_config_when_root_config_is_absent`; asserts `rule_applications.is_empty()`, now wrong
- `src/config.rs:623` — `config_debug_reports_stable_summary_fields`; asserts `rule_application_count: 0` and `frontmatter_rule_count: 0`, now wrong

## Open Questions

- None.

## Steps

- [x] Add `src/data/default-config.toml` with `docgarden.toml` content minus `exclude = ["tests/**"]`
- [x] In `src/config.rs`, replace the `else` branch at lines 231-235 with `toml::from_str::<FileConfig>(include_str!("data/default-config.toml")).context("failed to parse embedded default config")?`
- [x] Add unit test `load_applies_embedded_default_when_no_config_found` in `src/config.rs`
- [x] Update `load_ignores_nested_config_when_root_config_is_absent`: replace `config.rule_applications.is_empty()` assertion with non-empty check (embedded default applied)
- [x] Update `config_debug_reports_stable_summary_fields`: remove the `rule_application_count: 0` and `frontmatter_rule_count: 0` assertions; replace with non-zero presence checks
- [x] Update `git_root_is_used_when_no_docgarden_toml_is_found` in `tests/cli.rs`: add `description: guide` frontmatter to `docs/guide.md` so the embedded default's required-`description` rule passes
- [x] Add integration test `lint_applies_embedded_default_when_no_config_found` in `tests/cli.rs` (rule name: `frontmatter-field-missing`)

## Validation

- `cargo test config::tests::load_applies_embedded_default_when_no_config_found`
- `cargo test config::tests::load_ignores_nested_config_when_root_config_is_absent`
- `cargo test config::tests::config_debug_reports_stable_summary_fields`
- `cargo test --test cli git_root_is_used_when_no_docgarden_toml_is_found`
- `cargo test --test cli lint_applies_embedded_default_when_no_config_found`
- `cargo run -- lint --color never` from repo root — must still pass
- `cargo xtask validate`

## Discoveries

- Actual rule name for missing required frontmatter field is `frontmatter-field-missing`, not `required-frontmatter-field`.
- `serde(default = "default_respect_gitignore")` on `FileConfig.respect_gitignore` provides `true` automatically when the embedded TOML omits that field — no manual override needed in `Config::load`.
- All 156 tests (77 unit, 55 CLI, 24 path_behavior) pass after the change.

## Review

- [ ] None yet.
