---
description: "Exec plan for normalizing all user-facing identifiers (TOML config keys, diagnostic rule names) in `docgarden` to kebab-case and codifying the convention in `docs/CODESTYLE.md`."
---

# Kebab-case Config and Diagnostics

## Goal
- Every user-facing identifier in `docgarden` (TOML config keys, diagnostic rule names) is kebab-case. `docs/CODESTYLE.md` codifies this as the project standard so future config or rule additions do not reintroduce snake-case.

## Scope
- In: TOML schema rename of every snake-case key in `FileConfig`, `RuleConfig`, and `FrontmatterFieldConfig`; `Rule::as_str` kebab spellings for `MaxTokens`/`MaxLines`; `Rule::FromStr` simplified to accept only kebab-case (single canonical spelling); embedded `src/data/default-config.toml`; `README.md` config example; design-docs that show TOML; integration test fixtures; `docs/CODESTYLE.md` rule.
- Out: CLI flag names (already kebab), frontmatter document keys (only `description` is in use, single-word), Rust struct field names (stay snake; bridge with serde renames), `Cargo.toml` package metadata, hostname-style filenames or directory names.

## Relevant Areas
- `docs/CODESTYLE.md` — needs a new rule under "Rules in practice" mandating kebab-case for user-facing identifiers, with the snake-case-stays-on-Rust-fields exception noted.
- `src/config.rs` — `FileConfig`, `RuleConfig`, `FrontmatterFieldConfig` need `#[serde(rename = "...")]` (or `#[serde(rename_all = "kebab-case")]`) to bridge snake Rust field names to kebab TOML keys; `Rule::as_str` and `Rule::FromStr` need updating.
- `src/data/default-config.toml` — embedded default config that ships with the crate.
- `docgarden.toml` — repository dogfood config; must keep parsing after the schema rename.
- `README.md` — Quickstart config example block.
- `docs/design-docs/configuration.md`, `skills.md`, `line-and-token-limits.md`, `match-and-list.md` — show snake-case keys in narrative and code blocks; need updating with a one-line note that this rename happened.
- `docs/PRODUCT.md` — mentions `max_lines`/`max_tokens` in prose.
- `tests/cli.rs` — TOML fixtures.
- `src/config.rs` test module — TOML fixtures inside unit tests.
- `docs/exec-plans/active/0014-clean-code-followups.md` — its `Rule::as_str` kebab step is marked `[x]` and superseded by this plan; the actual code change happens here.

## Open Questions
- None yet

## Steps
- [x] Add a "Naming user-facing identifiers" subsection to `docs/CODESTYLE.md` under "Rules in practice" stating: kebab-case for diagnostic rule names, TOML config keys, and other user-typed identifiers; Rust struct fields stay snake-case and bridge via `#[serde(rename = "...")]` or `#[serde(rename_all = "kebab-case")]`. Cite principles 1 (Trust the toolchain — `serde` rename support) and 2 (One fact, one place — single canonical spelling).
- [x] Confirm `Rule::as_str` already returns `"max-tokens"` / `"max-lines"` for `MaxTokens` / `MaxLines` (the rename appears to have shipped already as part of the `[x]` step in plan 0014); if anything is still snake, fix it.
- [x] Simplify `Rule::FromStr` in `src/config.rs` so it accepts only the canonical kebab spelling; delete the snake-case fallback arms now that there is a single user-facing convention.
- [x] Add `#[serde(rename = "skills-dir")]` (and equivalents) or apply `#[serde(rename_all = "kebab-case")]` to `FileConfig`, `RuleConfig`, and `FrontmatterFieldConfig` so the TOML keys become `skills-dir`, `plans-dir`, `extend-extensions`, `remove-extensions`, `extend-special-filenames`, `remove-special-filenames`, `respect-gitignore`, `max-tokens`, `max-lines`, `max-chars`. Keep Rust field names snake.
- [x] Update `src/data/default-config.toml` to use kebab keys throughout.
- [x] Update the `README.md` Quickstart config example to use kebab keys.
- [x] Update TOML fixtures inside `src/config.rs` unit tests to use kebab keys. Delete or invert `rule_names_accept_existing_kebab_and_snake_spellings` so it asserts snake-case rule names now fail to parse (single canonical spelling).
- [x] Replace user-visible snake literals in error and diagnostic messages with their kebab spellings so the spoken-by-the-tool spelling matches the new TOML keys: `budget_limit("max-tokens", …)` and `budget_limit("max-lines", …)` in `src/config.rs`, the `"exceeds configured max-tokens = {}"` and `"exceeds configured max-lines = {}"` diagnostic strings in `src/lint/rules/file.rs`, and any matching test assertions on those error strings.
- [x] Sweep with `rg "max_tokens|max_lines|skills_dir|plans_dir|extend_extensions|remove_extensions|extend_special_filenames|remove_special_filenames|respect_gitignore|max_chars|unresolved_link_path|unresolved_backtick_path|prefer_links_for_local_paths|frontmatter_field_missing|frontmatter_malformed|frontmatter_field_max_chars" src/ tests/` and verify every remaining hit is either a Rust struct field name (snake stays) or an intentional negative-test fixture; no residual user-facing snake strings should survive in production code paths.
- [x] Update TOML fixtures in `tests/cli.rs` and any other integration tests to use kebab keys.
- [x] Update narrative and code blocks in `docs/design-docs/configuration.md`, `docs/design-docs/skills.md`, `docs/design-docs/line-and-token-limits.md`, `docs/design-docs/match-and-list.md`, and `docs/PRODUCT.md` to reference kebab spellings. The `skills.md:48` claim that "the spelling follows the rest of the new docgarden configuration by using snake_case in TOML" must be reversed to state the kebab convention.
- [x] Run `cargo fmt`.

## Validation
- `cargo test --lib config::tests`
- `cargo test --lib lint::tests`
- `cargo test --test cli`
- `cargo test --test path_behavior`
- `cargo run -- lint README.md docs/design-docs --color never`
- `cargo run -- lint docs/exec-plans/active/0015-kebab-case-config-and-diagnostics.md --color never`
- `cargo xtask validate`
- Manual: `cargo run -- lint .` against the dglint repo with the migrated `src/data/default-config.toml` to confirm the embedded default still parses end-to-end.

## Discoveries
- The repository root `docgarden.toml` also used the renamed budget and frontmatter keys; it must move with the embedded default so validation commands can parse the dogfood config.

## Review
- [ ] None yet
