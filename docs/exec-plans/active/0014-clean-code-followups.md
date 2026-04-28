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
- [x] Read `docs/CODESTYLE.md` before implementation and keep discoveries here if it changes any design choice.
- [x] Pin current behavior for rule parsing and rule policy reduction with focused tests covering unknown rule rejection, supported `enable` rules, `disable` always-on rules, opt-in rule toggles, and budget rule disable/override behavior.
- [x] Introduce a `Rule` enum for rule identifiers with `as_str`, `FromStr`, and serde deserialization from existing kebab-case and snake-case config spellings.
- [x] Replace string rule storage and comparisons in `RuleConfig`, `RuleApplication`, `EffectiveRulePolicy.ignored_rules`, config validation, and `DiagnosticPayload` emit sites with `Rule`.
- [x] Keep external diagnostic output unchanged by converting `Rule` to the existing string spelling at reporting boundaries.
- [x] Collapse `lint::FilePolicy` into `config::EffectiveRulePolicy` or pass the required leaf fields directly, choosing the smaller change after `Rule` lands.
- [x] Refactor `Config::effective_rule_policy_for_path` to initialize and mutate `EffectiveRulePolicy` directly instead of maintaining parallel locals.
- [x] Flatten `RuleConfig.disable` and `RuleConfig.enable` from `Option<Vec<_>>` to serde-defaulted `Vec<_>`.
- [x] Add or adjust focused tests for inline and link reference classification before refactoring `src/lint/references.rs`.
- [x] Unify `classify_inline_reference` and `classify_link_reference` behind `ReferenceKind`, with a small `CandidateReference` constructor or helper for repeated field assembly.
- [x] Extract private `Config::load` orchestration helpers such as config path resolution, parse/default loading, extension merging, and special-filename merging without changing error context.
- [x] Remove `#[cfg(test)]` fields from `CombinedFieldStats`; either delete formula-only assertions or expose stable `pub(crate)` accessors for derived values without changing the struct shape under tests.
- [x] Add a YAML parser skip/indent helper and use it in `parse_yaml_block`, `parse_block_value`, `parse_sequence`, and `parse_nested_mapping`.
- [x] Replace repeated YAML unsupported-character and unsupported-prefix checks with single-pass `matches!` helpers.
- [x] Change `lint_file` to return `Result<Vec<Diagnostic>>` directly and update `src/cli.rs` plus tests that mention `LintResult`.
- [x] Change `emit_finding` to take `&mut WalkState` and `Finding`, leaving `emit_findings` as a simple iterator.
- [x] Replace duplicate node variant re-checks in `src/lint/rules/local_paths.rs` with idiomatic pattern binding or typed helper functions.
- [x] Replace eager `unwrap_or(std::env::current_dir()?)` in `infer_repository_root` with a lazy fallback path and keep existing root tests passing.
- [x] Run `cargo fmt`.
- [x] Normalize `Rule::as_str` to kebab-case for `MaxTokens`/`MaxLines` so diagnostic output and `disable`/`enable` spellings are consistent across rules. `RuleConfig` TOML field keys (`max_tokens`, `max_lines`) stay snake-case; only the rule identifier changes. `FromStr` already accepts both spellings, so existing user config keeps working — only emitted diagnostics and test fixtures move.

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
- `Rule` serde deserialization now rejects unknown rule names during TOML parsing, so the unknown-rule test pins parse rejection rather than the old post-parse validator wording.

## Review
- [x] Delete `CombinedFieldStats::pseudo_df` and `pseudo_doc_count` accessors in `src/score.rs` and rewrite their three callers to assert behavior. The accessors are pure internal-state introspection: `pseudo_df("the") == 0` in `stopword_filter_is_symmetric_for_index_and_query` is redundant with the existing `score(...) > 0.0` assertion in the same test; `bm25_stats_follow_combined_field_shape` is formula-only and should be deleted; `can_build_stats_from_empty_corpus` should drop the `pseudo_doc_count` and `idf` re-derivation assertions and keep the behavioral `avgdl() == 1.0` and a `score(...)` call against the empty corpus. This satisfies the spirit of CODESTYLE principle 6 (tests assert behavior, not implementation), not just the letter of the `#[cfg(test)]` rule.
- [x] Drop the `Result` wrapper from `Config::effective_rule_policy_for_path` and `Config::frontmatter_policy_for_path` in `src/config.rs`. Both bodies are infallible after the in-place mutation refactor; returning the policy directly removes the `?` at the two call sites and `.unwrap()` across ~25 test sites.
- [x] Delete `validate_rule_list` in `src/config.rs` and its two call sites in `lower_rules`. The function only checks `rules.is_empty()`, but every call site is already guarded by `if !rules.is_empty()`, so the check is unreachable.
- [x] Move `is_supported_enabled_rule` in `src/config.rs` onto `Rule` as `Rule::supported_in_enable` so rule semantics live with the enum.
- [x] Move the in-function `use crate::lint::references::label_text;` in `src/lint/rules/local_paths.rs` to the top-of-file `use` block alongside the other `references` imports.
- [ ] Delete the dead `if !enabled_rules.is_empty()` guard around the `for enabled_rule in &enabled_rules` loop in `lower_rules` (`src/config.rs`). A `for` loop over an empty `Vec` is already a no-op, so the guard is unreachable for the same reason that motivated deleting `validate_rule_list`.
- [ ] Split `lower_rules` in `src/config.rs` (~80 lines) into `lower_rule_application` and `lower_frontmatter_rule` helpers. The loop body lowers two unrelated outputs (`RuleApplication` and `FrontmatterRule`) sharing only locals; CODESTYLE asks for sub-step extraction at ~50 lines for sequential bodies.
- [ ] Delete `once_lock_initializes_once_and_reuses_cached_value` in `src/lint/rules/file.rs`. It exercises `std::sync::OnceLock` itself, not anything in this crate; CODESTYLE principle 6 plus "Trust the toolchain" rejects tests that re-verify the standard library.
- [ ] Reshape `tokenizer_is_cached_across_calls` in `src/lint/rules/file.rs` to drop the pointer-identity assertion. Pointer identity is implementation; keep only the behavioral `count_tokens("hello world").unwrap() > 0` half (or delete the test entirely if the smoke check is already covered elsewhere).
- [ ] Drop the second `assert!(contains_disallowed_backtick_syntax(value) || value.is_empty() || value.starts_with("https://"))` line inside `inline_reference_rejects_disallowed_backtick_syntax` in `src/lint/mod.rs`. It re-asserts the implementation detail that explains why the previous behavioral assertion holds; the behavioral assertion already covers the contract.
