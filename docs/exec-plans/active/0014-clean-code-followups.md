---
description: "Active plan for addressing clean-code follow-ups across config rule typing, lint policy shape, reference classification, scoring internals, YAML parsing, and root inference."
---

# Clean Code Follow-ups

## Goal
- Address the clean-code follow-ups while preserving current CLI, config, lint, match-scoring, frontmatter, and root-inference behavior.

## Scope
- In: behavior-preserving refactors, type cleanup, focused regression tests, and small API simplifications for config rule typing, lint policy shape, reference classification, scoring internals, YAML parsing, and root inference.
- Out: the separate `src/config.rs` modularization TODO, new lint rules, output format changes, and broad architecture rewrites.

## Relevant Areas
- `docs/CODESTYLE.md` — required before adding validators, wrapper structs, parallel collections, or stringly typed identifiers.
- `src/config.rs` — rule config parsing, rule validation, effective rule policy reduction, and several TODO items.
- `src/lint/mod.rs` — `LintResult`, `FilePolicy`, `WalkState`, and `lint_file` call shape.
- `src/lint/reporting.rs` — `DiagnosticPayload` rule field type may change when introducing `Rule`.
- `src/lint/rules/file.rs` — file-budget diagnostics and rule identifiers.
- `src/lint/rules/frontmatter.rs` — frontmatter diagnostics and rule identifiers.
- `src/lint/rules/local_paths.rs` — local-path node dispatch, duplicate pattern matching, and rule identifiers.
- `src/lint/references.rs` — duplicated inline/link reference classification logic.
- `src/frontmatter.rs` — YAML parser skip/indent duplication and repeated unsupported-character predicates.
- `src/score.rs` — `CombinedFieldStats` test-only fields.
- `src/root.rs` — eager `current_dir()` fallback in `infer_repository_root`.
- `src/cli.rs` — caller of `lint_file` and `LintResult`.
- `tests/cli.rs` and `tests/path_behavior.rs` — integration coverage for config rule behavior and lint diagnostics.

## Open Questions
- None yet

## Steps
- [ ] Read `docs/CODESTYLE.md` before implementation and keep discoveries here if it changes any design choice.
- [ ] Pin current behavior for rule parsing and rule policy reduction with focused tests covering unknown rule rejection, supported `enable` rules, `disable` always-on rules, opt-in rule toggles, and budget rule disable/override behavior.
- [ ] Introduce a `Rule` enum for rule identifiers with `as_str`, `FromStr`, and serde deserialization from existing kebab-case and snake-case config spellings.
- [ ] Replace string rule storage and comparisons in `RuleConfig`, `RuleApplication`, `EffectiveRulePolicy.ignored_rules`, config validation, and `DiagnosticPayload` emit sites with `Rule`.
- [ ] Keep external diagnostic output unchanged by converting `Rule` to the existing string spelling at reporting boundaries.
- [ ] Collapse `lint::FilePolicy` into `config::EffectiveRulePolicy` or pass the required leaf fields directly, choosing the smaller change after `Rule` lands.
- [ ] Refactor `Config::effective_rule_policy_for_path` to initialize and mutate `EffectiveRulePolicy` directly instead of maintaining parallel locals.
- [ ] Flatten `RuleConfig.disable` and `RuleConfig.enable` from `Option<Vec<_>>` to serde-defaulted `Vec<_>`.
- [ ] Add or adjust focused tests for inline and link reference classification before refactoring `src/lint/references.rs`.
- [ ] Unify `classify_inline_reference` and `classify_link_reference` behind `ReferenceKind`, with a small `CandidateReference` constructor or helper for repeated field assembly.
- [ ] Extract private `Config::load` orchestration helpers such as config path resolution, parse/default loading, extension merging, and special-filename merging without changing error context.
- [ ] Remove `#[cfg(test)]` fields from `CombinedFieldStats`; either delete formula-only assertions or expose stable `pub(crate)` accessors for derived values without changing the struct shape under tests.
- [ ] Add a YAML parser skip/indent helper and use it in `parse_yaml_block`, `parse_block_value`, `parse_sequence`, and `parse_nested_mapping`.
- [ ] Replace repeated YAML unsupported-character and unsupported-prefix checks with single-pass `matches!` helpers.
- [ ] Change `lint_file` to return `Result<Vec<Diagnostic>>` directly and update `src/cli.rs` plus tests that mention `LintResult`.
- [ ] Change `emit_finding` to take `&mut WalkState` and `Finding`, leaving `emit_findings` as a simple iterator.
- [ ] Replace duplicate node variant re-checks in `src/lint/rules/local_paths.rs` with idiomatic pattern binding or typed helper functions.
- [ ] Replace eager `unwrap_or(std::env::current_dir()?)` in `infer_repository_root` with a lazy fallback path and keep existing root tests passing.
- [ ] Run `cargo fmt`.

## Validation
- `cargo test --lib config::tests`
- `cargo test --lib lint::tests`
- `cargo test --lib score::tests`
- `cargo test --lib frontmatter::tests`
- `cargo test --lib root::tests`
- `cargo test --test path_behavior`
- `cargo test --test cli`
- `cargo run -- lint docs/exec-plans/active/0014-clean-code-followups.md --color never`
- `cargo xtask validate`

## Discoveries
- None yet

## Review
- [ ] None yet
