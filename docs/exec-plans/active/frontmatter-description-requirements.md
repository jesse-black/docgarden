---
description: "Execution plan for implementing path-targeted frontmatter validation in docgarden, covering required-field checks, field length limits, a purpose-built YAML subset parser, config lowering, and dogfood policy for the repository."
---

# Implement Frontmatter Description Requirements

## Goal
- Add support for path-targeted frontmatter configuration so repositories can enforce `description` requirements and field length limits, including the repository-wide Markdown policy described in `docs/design-docs/configuration.md` and `docs/design-docs/standardized-frontmatter.md`.

## Scope
- In: planning implementation for `rules.frontmatter.required`, `rules.frontmatter.fields.<name>.max_chars`, Markdown frontmatter parsing/validation, tests, docs, and the later dogfooding update to `docgarden.toml`.
- Out: implementing the feature in this turn, adding broader frontmatter schemas beyond the requested `description` rule shape, autofix, non-Markdown metadata formats, or changing unrelated lint behavior.

## Relevant Areas
- `docs/design-docs/configuration.md` — defines the intended nested `rules.frontmatter` configuration shape and the split between "required" and "validate when present".
- `docs/design-docs/standardized-frontmatter.md` — defines the repository-wide first-party `description` requirement with `README.md` and `AGENTS.md` exceptions.
- `ARCHITECTURE.md` — points to `src/config.rs` and `src/lint/mod.rs` as the current config and lint orchestration extension points; this plan should add a dedicated frontmatter rule module under `src/lint/rules/`.
- `src/config.rs` — current strict TOML parsing and rule lowering; will need new frontmatter config structs and effective per-path policy lookup.
- `src/lint/mod.rs` — current file lint orchestration; will need to invoke shared frontmatter parsing and route frontmatter findings alongside other file-level checks.
- `src/lint/rules/file.rs` — existing file-level checks such as `max_tokens` and `max_lines`; should remain separate from the frontmatter-specific implementation.
- `tests/cli.rs` — end-to-end coverage for config-driven rule behavior.
- `docgarden.toml` — dogfood target for the final requested repository policy during implementation, not during this planning turn.

## Open Questions
- Confirm v1 malformed-frontmatter behavior for a file that starts with `---` but does not contain a valid closing frontmatter block before content begins.

## Steps
- [x] Confirm the minimal v1 frontmatter behavior against the design docs: YAML frontmatter block at file start, `required` string list, and `fields.<name>.max_chars` integer limit.
- [x] Treat only a file-leading `--- ... ---` block as frontmatter; any later `---` should remain normal Markdown content such as a thematic break or body text delimiter.
- [x] Implement one shared frontmatter parser architecture rather than separate discovery and lint parsers, shipping the in-memory lint entry point now and preserving a clear path for a later prefix-only discovery entry point over the same core behavior.
- [x] Support the minimal YAML subset drafted in `docs/design-docs/standardized-frontmatter.md`: top-level mappings, nested mappings, single-line scalar strings, booleans, base-10 integers, ISO `YYYY-MM-DD` date-like strings, and `- ` sequences for list values.
- [x] Keep likely permanent exclusions out of scope for this plan: anchors, aliases, tags, multi-document YAML, and duplicate keys as valid input.
- [x] Keep temporary parser exclusions out of scope for this plan: block scalars and flow-style collections.
- [x] Add failing config tests in `src/config.rs` for the nested TOML shape, unknown-field rejection, invalid empty field names, and non-positive `max_chars`.
- [x] Add failing config tests proving duplicate field names within a single rule entry are rejected, while multiple matching rule entries still merge by last-match-wins semantics.
- [x] Add failing lint tests for these behaviors: `description` required on matching Markdown files, `AGENTS.md` excluded from the requirement, non-matching files unaffected, malformed leading frontmatter is reported distinctly from missing fields, and the chosen diagnostic ids are `frontmatter-field-missing` and `frontmatter-field-max-chars`.
- [x] Extend `src/config.rs` with frontmatter rule parsing and effective per-path lowering that preserves current last-matching-entry-wins behavior alongside rule-entry `exclude`.
- [x] Add a dedicated frontmatter rule module under `src/lint/rules/` for frontmatter parsing and validation, with one shared parser core, a shipped full-document entry point for linting, and architecture that can support a later buffered prefix-read entry point without semantic drift.
- [x] Wire `src/lint/mod.rs` to invoke that dedicated frontmatter rule module during linting, emitting deterministic diagnostics for malformed leading frontmatter, missing required fields, and overlong field values without adding autofix.
- [x] Update repository docs to describe the shipped frontmatter config shape and behavior, including any new rule ids and YAML-frontmatter parsing constraints.
- [x] During implementation, add the requested policy to `docgarden.toml` exactly as dogfood configuration:

      [[rules]]
      path = "**/*.md"

      [rules.frontmatter.fields.description]
      max_chars = 1024

      [[rules]]
      path = "**/*.md"
      exclude = ["AGENTS.md"]

      [rules.frontmatter]
      required = ["description"]

- [x] Run targeted validation and then `cargo xtask validate` before handing off for implementation review.

## Validation
- `cargo test config`
- `cargo test --test cli`
- `cargo run -- lint docs/design-docs/configuration.md docs/design-docs/standardized-frontmatter.md docs/exec-plans/active/frontmatter-description-requirements.md --color never`
- `cargo xtask validate`

## Discoveries
- `docs/design-docs/configuration.md` already specifies the exact split requested here: one rule entry that always enforces `description.max_chars`, plus a second rule entry that requires `description` for `**/*.md` while excluding `AGENTS.md`.
- `docs/design-docs/standardized-frontmatter.md` narrows the first-party policy intent: repository docs should require `description`, with `README.md` and any `AGENTS.md` treated as the clearest exceptions; this request covers only the `AGENTS.md` exception for now.
- Current code has no frontmatter-specific config or parser implementation yet, so this work is additive and should fit naturally into `src/config.rs`, `src/lint/mod.rs`, and a new dedicated rule module under `src/lint/rules/`.
- Decision: Use `frontmatter-field-missing` for required-field violations and `frontmatter-field-max-chars` for present values that exceed configured limits.
- Decision: Reject duplicate declarations within a single rule entry, but keep cross-entry merging as last-match-wins for multiple matching rule entries.
- Decision: Use one shared purpose-built frontmatter parser for both linting and future discovery commands rather than separate parser implementations, but phase delivery so linting ships first and discovery-oriented prefix reads can be added later on the same parser core.
- Decision: Keep anchors, aliases, tags, multi-document YAML, and duplicate keys permanently out of scope unless a later product requirement clearly justifies reopening that choice.
- Decision: Keep block scalars and flow-style collections out of scope for this implementation even though they may be reconsidered later if repository metadata needs them.

## Review
- [x] Finding: `RuleConfig.exclude` is now accepted for every `[[rules]]` entry in `src/config.rs`, and `docs/design-docs/configuration.md` describes it as a generic rule-entry narrow, but `lower_rules` only applies `exclude` when lowering `frontmatter` rules. Entries such as `max_tokens`, `max_lines`, `path_style`, or future rule-specific options will silently ignore `exclude`, so the shared rule-entry contract is now inconsistent with runtime behavior.
- [x] Finding: The shared rule-lowering shape in `src/config.rs` is not DRY enough yet. `RuleConfig.exclude` lives on the generic rule-entry model, but only the frontmatter lowering path consumes it, which is a strong sign the branch wants a shared "match, narrow with exclude, then apply rule-specific behavior" abstraction rather than repeating targeting logic per rule family.
- [x] Finding: `src/lint/rules/frontmatter.rs` is readable overall, but key validation and duplicate-key rejection are duplicated between `parse_yaml_block` and `parse_nested_mapping`. That duplication is manageable now, though it increases the risk of semantic drift as the supported YAML subset expands.
- [x] Finding: Frontmatter diagnostic construction is repetitive in `src/lint/rules/frontmatter.rs`; the same missing-field payload is assembled in both the `FrontmatterParseResult::None` and `FrontmatterParseResult::Valid` branches. A small helper would make later message or severity changes safer and keep rule evaluation more focused on control flow.
- [x] Finding: The refactor leaves the old `ignored_rules_for_path` helper in `src/diagnostics.rs` even though `src/lint/mod.rs` now routes through `Config::ignored_rules_for_path`. `cargo test config` and `cargo test --test cli` both emit a dead-code warning for that stale helper, and keeping it around preserves an outdated ignore-resolution implementation that does not reflect the new shared rule-entry matching path.
