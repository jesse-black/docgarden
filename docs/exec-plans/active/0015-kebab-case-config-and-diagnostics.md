---
description: "Active plan for normalizing all user-facing identifiers (TOML config keys, diagnostic rule names) in `docgarden` to kebab-case and codifying the convention in `docs/CODESTYLE.md`."
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
- [ ] Update `Rule::as_str` so `MaxTokens` returns `"max-tokens"` and `MaxLines` returns `"max-lines"`, matching the kebab spelling of every other rule.
- [ ] Simplify `Rule::FromStr` in `src/config.rs` so it accepts only the canonical kebab spelling; delete the snake-case fallback arms now that there is a single user-facing convention.
- [ ] Add `#[serde(rename = "skills-dir")]` (and equivalents) or apply `#[serde(rename_all = "kebab-case")]` to `FileConfig`, `RuleConfig`, and `FrontmatterFieldConfig` so the TOML keys become `skills-dir`, `plans-dir`, `extend-extensions`, `remove-extensions`, `extend-special-filenames`, `remove-special-filenames`, `respect-gitignore`, `max-tokens`, `max-lines`, `max-chars`. Keep Rust field names snake.
- [ ] Update `src/data/default-config.toml` to use kebab keys throughout.
- [ ] Update the `README.md` Quickstart config example to use kebab keys.
- [ ] Update TOML fixtures inside `src/config.rs` unit tests to use kebab keys; tests that previously asserted snake-spelling rejection (or the `unresolved_link_path` style spellings) should now assert that snake-case keys fail to parse.
- [ ] Update TOML fixtures in `tests/cli.rs` and any other integration tests to use kebab keys.
- [ ] Update narrative and code blocks in `docs/design-docs/configuration.md`, `docs/design-docs/skills.md`, `docs/design-docs/line-and-token-limits.md`, `docs/design-docs/match-and-list.md`, and `docs/PRODUCT.md` to reference kebab spellings. The `skills.md:48` claim that "the spelling follows the rest of the new docgarden configuration by using snake_case in TOML" must be reversed to state the kebab convention.
- [ ] Run `cargo fmt`.

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
- None yet

## Review
- [ ] None yet
