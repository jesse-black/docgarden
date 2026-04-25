---
description: "Working design draft for `docgarden match` and `list`; read when planning metadata-based document discovery, skills discovery, or search-versus-routing behavior."
---

# Match and List

## Purpose

This document is a working design draft for the `docgarden match` and `list` commands.

The goal is to make repository knowledge discoverable without requiring handwritten index-style files as the default navigation mechanism. If repositories adopt standardized YAML front matter with fields such as `name` and `description`, `docgarden` should be able to derive useful catalog and matching views directly from that metadata.

The product should describe these commands in terms of explicit configuration, not in terms of this repository's current directory layout. The selected near-term conventions are `skills_dir` for repository-local skills and `plans_dir` for ExecPlans. Broader discovery roots or named groups can wait until a concrete feature needs them.

## Proposed Commands

The initial command family under consideration is:

- `docgarden list` with `ls` as an alias
- `docgarden match <QUERY>` with `m` as an alias

These commands should operate on Markdown documents that opt into recognizable front matter schemas and belong to configured scopes.

The product intent is discovery, not arbitrary text search.

For v1, the discovery surface should stay intentionally small and deterministic:

- `path` is always present
- frontmatter `name` is surfaced when present
- frontmatter `description` is surfaced when present
- scope switches such as `--skills`, `--plans`, `--active-plans`, and `--completed-plans` restrict discovery to configured document sets
- `match` uses numeric ranking internally, but the default output should emphasize result order rather than score display

Skill-specific validation remains outside this command family. `docgarden skills validate` is covered in `docs/design-docs/skills.md`.

## Why Frontmatter Instead Of Mandatory Index Files

Handwritten index files can be useful, but they are an additional maintenance burden.

If a repository already expects agents or humans to maintain document front matter, then the same metadata can power:

- list views
- query-time matching
- future scope-specific routing or linting

This fits the broader `docgarden` philosophy better than requiring every document collection to maintain both the documents themselves and a parallel set of catalog pages.

It also matches an agent-oriented workflow where prompts, skills, or harness instructions already tell agents to put the information needed for discovery into front matter.

## Why `match` Instead Of `search`

The tentative subcommand name is `match`, not `search`, because the intended behavior is narrower and more mechanical than full-text search.

The current direction is that `docgarden match <QUERY>` should match against front matter and closely related metadata fields, not against the entire Markdown body by default.

This naming distinction matters because repositories already have a strong full-text search tool: `rg`.

If an agent wants broad body-text retrieval, it can use `rg` directly. `docgarden match` should provide a higher-signal, lower-noise path that rewards repositories for maintaining good front matter.

## Agent Triggering Alignment

There is also a strong precedent for metadata-first matching in agent tooling.

For example, the OpenAI `skill-creator` guidance says:

    Every SKILL.md consists of:

        Frontmatter (YAML): Contains name and description fields. These are the only fields that Codex reads to determine when the skill gets used, thus it is very important to be clear and comprehensive in describing what the skill is, and when it should be used.
        Body (Markdown): Instructions and guidance for using the skill. Only loaded AFTER the skill triggers (if at all).

This is highly relevant to `docgarden`.

If repositories standardize front matter and teach agents to include the information needed to trigger use in fields such as `name`, `description`, tags, or scope metadata, then `docgarden match` can remain intentionally narrow and still be very useful.

That gives the repository a clean contract:

- front matter is the discovery layer
- document bodies are the deep content layer
- `docgarden match` operates on the discovery layer
- `rg` and similar tools remain the body-search fallback

## Discovery Traversal

`docgarden list` and `docgarden match` should reuse the existing lint file-traversal system rather than introducing a separate discovery walker.

For v1, that means discovery commands should:

- reuse the same discovery walker and include/exclude rules that lint uses
- discover only Markdown files selected by the existing traversal and include/exclude rules
- respect `.gitignore`, `.ignore`, and related git exclude behavior by default
- support the same `--no-gitignore` flag so callers can opt out of gitignore-based filtering when needed

For `docgarden list`, directory targets should support an explicit recursion flag:

- `-R, --recurse`

With `--recurse`, `list` should descend into nested directories under each directory target. Without it, directory targets should be limited to Markdown files directly within the named directory. This gives `list` a predictable "shallow by default, deep on request" shape while still reusing the same ignore and include/exclude rules as lint for the files it considers.
Recursive output should stay flat: one discovered document per output row, with no per-directory headings or nested tree rendering.

`docgarden match` does not take positional filesystem targets. It scores against the full repository-root discovery set determined by config and `--no-gitignore`, which keeps the corpus-local IDF table deterministic for a given repo state.

This keeps repository traversal behavior consistent across lint and discovery commands, reduces implementation duplication, and avoids subtle cases where `lint` and `list` or `match` disagree about which documents exist.

## Resolved V1 Output Contract

The v1 contract should optimize for two audiences:

- humans scanning CLI output
- agents doing first-pass document routing

The common result shape should be:

- `path`: repository-relative path to the document
- `name`: frontmatter `name` when present
- `description`: frontmatter `description` when present

For `match`, ranking still uses a numeric relevance score internally, but the default text output should optimize for routing rather than score inspection.

The score should be treated as an ordering aid, not as a durable semantic contract across scoring-version changes. Callers should trust sorted order first and only use score for debugging or explain-mode inspection within the same tool version.

### Text Output

Default text output should stay compact and token-conscious.

`docgarden list` should emit one result per line in a stable field order:

```text
docs/design-docs/example.md | Example Guide | Short description for discovery output.
```

`docgarden match <QUERY>` should emit:

```text
docs/design-docs/example.md | Example Guide | Short description for discovery output.
```

This keeps the path first for quick scanning and makes it easy for agents to strip trailing fields when they only need candidates.

`docgarden match --explain <QUERY>` should emit a header row followed by:

```text
score | relative | coverage | path | name | description
8.42 | 100% of top | 3/3 terms | docs/design-docs/example.md | Example Guide | Short description for discovery output.
```

This keeps the default mode compact while still providing a deterministic diagnostic view for humans or agents that need to inspect why one result outranked another.

There should be no JSON output surface for these commands in v1. Unlike lint diagnostics, discovery output is primarily an agent-facing routing tool, and compact text is the main product value.

The subcommand help text should explain the output columns and field order instead of adding a header row to every default result set. That is a good fit for progressive discovery: the first layer stays compact during normal use, while `--help` provides the format explanation when a human or agent needs to orient itself. `--explain` is the exception: it should print a header row because its purpose is deliberate inspection rather than compact routing.

### Result Limits

`match` should support limiting the number of results returned.

The v1 interface should be:

- `--limit N`
- `-n N`

This is the main token-conservation control for ranked discovery. Agents usually do not want every weak candidate; they want the best few options to inspect next.

The limit should apply after ranking, so callers get the top N highest-scoring matches.

### Path-Only Mode

An explicit path-only mode is worth adding because it matches how agents often work.

After an initial discovery step, agents frequently want to:

- pass candidate paths directly into a follow-up file read
- conserve tokens before opening the most promising documents
- avoid carrying repeated descriptions once routing is complete

The simplest v1 interface is:

- `--path-only` for `match`
- `-p` as the short form

That should print one repository-relative path per line.

`--path-only` is preferable to making path-only the default because first-pass discovery is usually better when the agent can see `name` and `description` before choosing which document bodies to load.

`-p` is a reasonable short flag here because it reads naturally as "path" and does not compete with `-n` for result count.

## Relationship To Scoring

`match` is a ranked discovery command, but the scoring design no longer lives in this document.

The contract here is intentionally narrow:

- `match` ranks over discovery metadata rather than Markdown body text
- ordering is the primary contract for default output
- raw numeric score is an explain-mode aid, not the main user-facing interface
- changes to scoring, normalization, tie-breaking, or explain-specific score semantics belong in `docs/design-docs/scoring.md`

### `docgarden list`

`docgarden list` should emit a flat list of matching Markdown documents with a compact metadata summary.

`list` should remain the canonical subcommand name in help text and documentation, with `ls` provided as a convenience alias.

By default, it should operate over the same Markdown traversal system used by lint, scoped by invocation targets and configuration rather than by a separate filesystem walk.

For directory targets, `list` should support:

- `-R, --recurse`

That flag should recurse into nested subdirectories under each target. Without it, directory targets should be shallow.
Even with `--recurse`, the output should remain a flat result list rather than grouped directory sections.

A reasonable first output shape is:

- path
- name frontmatter if present
- description frontmatter if present
- optional scope label if known

The command should be useful both for humans scanning the output and for agents consuming it in a deterministic workflow.

#### Scope switches

`docgarden list` should support scope switches for high-value repository knowledge sets:

- `--skills`: list configured skills under `skills_dir`
- `--plans`: list all ExecPlans under `plans_dir`
- `--active-plans`: list ExecPlans under `{plans_dir}/active/`
- `--completed-plans`: list ExecPlans under `{plans_dir}/completed/`

These switches answer inventory questions rather than ranking questions. Active plans are usually one file or a small handful, so `docgarden list --active-plans` is a better affordance than teaching the scorer repo-specific active-plan boosts.

Scope switches should be mutually exclusive with positional directory targets unless a later design gives target-plus-scope filtering a clear use case.

The output shape should remain the common discovery row:

- path
- name frontmatter if present
- description frontmatter if present
- optional scope label if known

For skills, the path is the skill's `SKILL.md` path, `name` is the skill frontmatter `name`, and `description` is the skill frontmatter `description`.

For ExecPlans, the path is the plan Markdown path under `plans_dir`. The `name` field can use frontmatter `name` when present and otherwise fall back to the filename stem, matching normal discovery behavior.

### `docgarden match <QUERY>`

`docgarden match <QUERY>` should rank candidate documents based on metadata fields rather than full body text.

`match` should remain the canonical subcommand name in help text and documentation, with `m` provided as a convenience alias.

It should use the same Markdown discovery set that `lint` would see for the same repository root, config, and gitignore settings.

In shipped v1, `match` does not accept positional filesystem targets. The command always scores against the full repository-root discovery set so ranking stays deterministic for a given repo state.

The implementation should consider:

- name
- description
- path

The default output should stay compact and show the common result fields in ranked order without a raw score column.

`--explain` is the diagnostic output mode for ranking inspection. The scoring model, normalization rules, and explain-only score semantics belong in `docs/design-docs/scoring.md`.

#### Scope switches

`docgarden match` may support scope switches when ranked routing within a scope is useful:

- `--skills`: rank only configured skills
- `--plans`: rank only ExecPlans

Do not add `--active-plans` to `match` unless dogfooding shows a real need. Active-plan discovery is mostly deterministic inventory, so `docgarden list --active-plans` should be the first tool agents use when they need the current plan.

`docgarden match --skills <QUERY>` should behave like normal metadata matching, but restricted to the configured skills directory and skill front matter schema.

The initial matching fields should likely be:

- `name`
- `description`
- path
- optional compatibility metadata

This command should help answer questions like:

- which existing skill should handle this task
- whether the repository already has a skill for a workflow
- which skill names or descriptions overlap with a given topic

Because the skill body is only relevant after a skill has been selected, skill-scoped matching should stay focused on the metadata that determines triggering rather than on full-text body search.

`docgarden match --plans <QUERY>` should use the same matching fields as normal metadata routing, restricted to ExecPlans under `plans_dir`. It is useful when a repository has enough plans that exact inventory is too broad, but it should not replace `list --active-plans` for current-plan workflows.

## Optional Index Files

This proposal does not reject index files entirely.

There are valid cases where a repository may want curated index-style files:

- an existing repository or imported workflow already uses them
- the index provides a narrative entrypoint rather than a mechanical catalog
- a team wants a human-authored overview for a configured scope
- someone starts from a workflow like the LLM wiki gist, where an index page is part of the pattern

The current direction is:

- frontmatter-driven discovery should be the default
- curated index files may be supported as optional configuration
- index files should be treated as overlays, not mandatory parallel inventories

That means future rules might validate configured index files when they exist, but repositories should not need them just to get discovery.

## Interaction With Front Matter Policy

This proposal depends on standardized front matter being meaningful enough to support discovery.

That raises a product requirement: front matter cannot be treated as perfunctory metadata. Repositories that want strong discovery need prompts, skills, or local guidance that tell agents to write names and descriptions that are specific enough to trigger later use.

This also argues for scope-aware front matter schemas rather than one universal metadata blob. Different scopes may need different discovery fields, but each scope should still provide a compact, high-signal trigger surface.

## Interaction With Context Budgets

Metadata-driven discovery complements context-budget limits.

If `docgarden list` and `match` can expose the right files from front matter alone, agents can avoid loading full document bodies too early. That supports the broader progressive-disclosure model:

- small routing surface first
- deeper documents only when needed

This is especially attractive for high-traffic scopes such as skills, design docs, plans, and references.
