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

## Preliminary Working Ideas

The rest of this document is still exploratory.

### Tokenizer Decision

Use `tiktoken-rs` as the tokenizer backend for token-budget checks.

Use `o200k_base` as the default encoding for counting tokens.

This is a pragmatic product decision:

- `tiktoken-rs` is an active and well-supported Rust library for OpenAI-style token counting
- OpenAI model tokenization is a good enough proxy for general LLM context cost in an agent-oriented repository workflow
- `o200k_base` is a stable encoding choice that aligns with newer OpenAI model families without tying the rule behavior to one model name

This should be documented as an approximation for agent context cost, not as a universal exact token count across every model vendor.

### Rule Model

`docgarden` should support context-budget checks that are:

- mechanical
- file-local
- configurable
- scoped by built-in scope or path pattern

The first measurable checks should be:

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

This implies that budget rules should be driven by built-in scope, path, or both.

For `AGENTS.md`, the current external reference points are still softer than the Agent Skills defaults for skill files, but they are directionally useful:

- [OpenAI's harness engineering post](https://openai.com/index/harness-engineering/) describes a short `AGENTS.md` as "roughly 100 lines"
- [Factory's AGENTS.md guidance](https://docs.factory.ai/cli/configuration/agents-md#7-·-best-practices) recommends aiming for `<= 150` lines

These should be treated as provisional reference points for future `AGENTS.md` defaults rather than as settled standards.

### Configuration Direction

Context-budget limits should use the shared configuration model in `docs/design-docs/configuration.md` rather than a separate `[[limits]]` table.

A small example:

    [[rules]]
    scope = "skills"
    rule = "context-budget"
    max-lines = 500
    max-tokens = 5000

    [[rules]]
    path = "AGENTS.md"
    rule = "context-budget"
    max-tokens = 1200

The important point is that limit fields are rule options scoped by the same targeting layer as other rule behavior. That lets budget checks reuse built-in scopes such as skills without requiring a feature-specific configuration table.

### Skill-Aware Configuration Questions

Open questions for the configuration model:

- Should there be a top-level skill-directory configuration that both skill commands and budget checks reuse?
- Should skill main-file detection be based on path patterns, explicit skill roots, or both?
- Should budget defaults be attached to built-in scopes such as `skill_main_file` rather than raw globs?
- How should repositories override defaults for one skill collection without affecting another?

### Severity And Enforcement

Another open question is whether budgets should default to warnings or errors.

A plausible direction is:

- high-traffic files such as `AGENTS.md` may justify error-level enforcement
- skill main-file defaults may start as warnings so repositories can adopt them gradually
- imported references may opt out or use warning-only budgets

This remains preliminary and should not be treated as settled policy yet.

## Open Questions

- Which built-in scopes should ship with defaults beyond skill main files?
- Should imported external references have no default budget checks because source fidelity matters more than compactness?
- Should `AGENTS.md` get a built-in default budget, and if so, should it be token-based, line-based, or both?
- How should budget rules interact with generated files or files intentionally exempted through configuration?
- How should skill-file limits balance usefulness and simplicity when repositories may contain both local/generated skills and imported skills from external sources?
