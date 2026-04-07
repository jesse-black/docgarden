# Context Budget Limits

## Purpose

This document is a working design draft for line-count and token-budget checks in `docgarden`.

The product goal is mechanical enforcement of context efficiency for agent-facing documents. Rather than judging whether a document is "well structured", `docgarden` should enforce measurable limits that help repositories conserve agent context and keep high-traffic files cheap to load.

## Accepted Default: Skill Main File

For skill files, the Agent Skills specification already gives a strong default policy. Treat that guidance as accepted input for skill main-file defaults in `docgarden`.

The specification guidance is:

- metadata is small and loaded for every skill at startup
- the full skill main-file body is loaded when the skill activates
- resources are loaded only as needed
- the main skill file should stay under 500 lines
- the main skill file body should stay under 5000 tokens

In `docgarden`, that suggests the following default checks for skill main files:

| Limit | Default | Status |
| --- | --- | --- |
| `max_lines` | `500` | Accepted default for skill main files |
| `max_tokens` | `5000` | Accepted default for skill main files |

These defaults should be treated as defaults, not hard-coded universal policy. Repositories may need to override them, but `docgarden` can ship them as a strong built-in starting point for skill files.

## Current Direction

The first implementation is explicit-config only. It supports path-targeted `max_lines` and `max_tokens` rules, counts the complete Markdown file, defaults configured budget diagnostics to errors, and lets entries use `severity = "warn"` for warning-only adoption.

### Tokenizer Decision

Use `tiktoken-rs` as the tokenizer backend for token-budget checks.

Use `o200k_base` as the default encoding for counting tokens.

This is a pragmatic product decision:

- `tiktoken-rs` is an active and well-supported Rust library for OpenAI-style token counting
- OpenAI model tokenization is a good enough proxy for general LLM context cost in an agent-oriented repository workflow
- `o200k_base` is a stable encoding choice that aligns with newer OpenAI model families without tying the rule behavior to one model name

This should be documented as an approximation for agent context cost, not as a universal exact token count across every model vendor.

### Rule Model

`docgarden` supports context-budget checks that are:

- mechanical
- file-local
- configurable
- targeted by repository-relative path pattern

The first measurable checks are:

- line count
- token count

Other possible measures such as file size or section count are lower priority and should not be assumed yet.

### Why Limits Instead Of Structure Rules

Line and token limits fit the product philosophy better than generic "structure checks".

They:

- conserve context directly
- are easy to explain
- avoid natural-language judgment
- are easy to validate in CI
- create pressure to move detailed material into deeper documents, references, or generated artifacts

This is especially valuable for high-traffic agent-facing entry points such as `AGENTS.md` and skill main files.

### Scope Policy

Different file types should be able to carry different budget defaults.

Examples:

- skill main files may use the Agent Skills defaults
- `AGENTS.md` may need a tighter token budget than other docs because it is frequently auto-loaded
- imported external references may need looser or disabled budget checks because fidelity to source material matters more than aggressive trimming
- execution plans may need a different budget model because self-containment is part of their purpose

This implies that budget rules should be driven by repository-relative paths and path patterns, not named target aliases.

For `AGENTS.md`, the current external reference points are still softer than the Agent Skills defaults for skill files, but they are directionally useful:

- [OpenAI's harness engineering post](https://openai.com/index/harness-engineering/) describes a short `AGENTS.md` as "roughly 100 lines"
- [Factory's AGENTS.md guidance](https://docs.factory.ai/cli/configuration/agents-md#7-·-best-practices) recommends aiming for `<= 150` lines

These should be treated as provisional reference points for future `AGENTS.md` defaults rather than as settled standards.

### Configuration Direction

Context-budget limits should use the shared configuration model in `docs/design-docs/configuration.md` rather than a separate `[[limits]]` table.

A small example:

    [[rules]]
    path = ".agents/skills/**/SKILL.md"
    max_lines = 500
    max_tokens = 5000

    [[rules]]
    path = "AGENTS.md"
    max_tokens = 1200

The important point is that limit fields are rule options targeted by the same path layer as other rule behavior. Setting `max_tokens` or `max_lines` is enough to enable the corresponding budget check for that path pattern. A separate `rule = "context-budget"` field would add ceremony without selecting any behavior that the limit fields do not already identify.

If built-in defaults later apply a token or line budget automatically, repositories can opt out through the existing disable list:

    [[rules]]
    path = "docs/references/**"
    disable = ["max_tokens", "max_lines"]
    reason = "Imported source-derived docs preserve source fidelity over compactness."

### Skill-Aware Configuration Questions

Open questions for the configuration model:

- Should context-budget implementation wait for `skills_dir`, or should the first version require explicit path patterns for skill main files?
- Should skill main-file detection use the configured `skills_dir` plus the Agent Skills main-file filename convention once `skills_dir` exists?
- How should repositories override defaults for one skill collection without affecting another?

### Severity And Enforcement

Explicit context-budget limits default to errors.

The configured entry's `severity` applies to every budget field in that `[[rules]]` entry. If a repository wants token limits to be errors and line limits to be warnings for the same path, it can use two entries with the same path:

    [[rules]]
    path = "AGENTS.md"
    max_tokens = 1200

    [[rules]]
    path = "AGENTS.md"
    max_lines = 150
    severity = "warn"

### Agent Entry-Point Defaults

Agent entry-point files such as `AGENTS.md`, Claude guidance files, and Gemini guidance files are good candidates for budget checks because they are often loaded early and repeatedly.

The safer initial direction is to let `docgarden init` generate explicit default entries rather than shipping broad built-in defaults for every possible agent entry point. An interactive TUI could ask which agents the repository wants to configure for, while non-interactive flows could use command-line switches. That keeps the resulting policy visible in `docgarden.toml` and avoids surprising repositories that carry large Claude or Gemini guidance files for reasons unrelated to Codex-style context loading.

Built-in defaults remain more defensible for skill main files because the Agent Skills specification already provides line and token limits. Even there, the implementation should make override and opt-out behavior explicit through path-targeted rules.

## Open Questions

- Should any built-in defaults ship beyond skill main files, or should agent entry-point defaults always be generated by `docgarden init`?
- Should imported external references have no default budget checks because source fidelity matters more than compactness?
- Should `AGENTS.md`, Claude guidance files, and Gemini guidance files get init-generated token budgets, line budgets, or both?
- How should budget rules interact with generated files or files intentionally exempted through configuration?
- How should skill-file limits balance usefulness and simplicity when repositories may contain both local/generated skills and imported skills from external sources?
