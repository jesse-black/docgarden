# Frontmatter-Driven Discovery Commands

## Purpose

This document is a working design draft for metadata-driven discovery commands in `docgarden`.

The goal is to make repository knowledge discoverable without requiring handwritten index-style files as the default navigation mechanism. If repositories adopt standardized YAML front matter with fields such as `title` and `description`, `docgarden` should be able to derive useful catalog and matching views directly from that metadata.

The product should describe these commands in terms of configured document families and configured search roots, not in terms of this repository's current directory layout. Repo-local paths such as `docs/references/` or `.agents/skills` are examples of configuration, not universal product conventions. The configuration model itself is split into `docs/design-docs/configuration.md`.

## Proposed Commands

The initial command family under consideration is:

- `docgarden list`
- `docgarden tree`
- `docgarden match <QUERY>`
- `docgarden skills list`
- `docgarden skills match <QUERY>`

These commands should operate on Markdown documents that opt into recognizable front matter schemas and belong to configured discovery scopes.

The product intent is discovery, not arbitrary text search.

## Why Frontmatter Instead Of Mandatory Index Files

Handwritten index files can be useful, but they are an additional maintenance burden.

If a repository already expects agents or humans to maintain document front matter, then the same metadata can power:

- list views
- tree views
- query-time matching
- future family-specific routing or linting

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

If repositories standardize front matter and teach agents to include the information needed to trigger use in fields such as `title`, `description`, tags, or document-family metadata, then `docgarden match` can remain intentionally narrow and still be very useful.

That gives the repository a clean contract:

- front matter is the discovery layer
- document bodies are the deep content layer
- `docgarden match` operates on the discovery layer
- `rg` and similar tools remain the body-search fallback

## Preliminary Command Semantics

### `docgarden list`

`docgarden list` should emit a flat list of matching Markdown documents with a compact metadata summary.

By default, it should operate over configured discovery families or configured knowledge roots rather than over every Markdown file in the repository.

A reasonable first output shape is:

- path
- title
- description
- optional document-family label if known

The command should be useful both for humans scanning the output and for agents consuming it in a deterministic workflow.

### `docgarden tree`

`docgarden tree` should present the same metadata grouped hierarchically, likely by directory and possibly later by configured document family.

This command is primarily a navigation aid.

It should help answer questions like:

- what knowledge exists under a configured design-docs family
- which configured roots contain richly described documents versus sparse ones
- which parts of the repository knowledge map have metadata coverage

### `docgarden match <QUERY>`

`docgarden match <QUERY>` should rank candidate documents based on metadata fields rather than full body text.

The first pass should likely consider:

- title
- description
- path
- document-family metadata
- tags or similar fields if a schema defines them

The output should explain why a document matched, for example by surfacing the matching fields or snippets from metadata values.

### `docgarden skills list`

`docgarden skills list` should behave like `docgarden list`, but scoped to the configured agent skills root, which in this repository would usually be `.agents/skills`.

This command should surface the discovery metadata for skills without requiring the caller to know the exact skill paths in advance.

A reasonable first output shape is:

- skill path
- skill `name`
- skill `description`
- optional compatibility or other skill-schema metadata if configured for display

Because skill front matter is already the trigger surface for agent use, this command is a natural fit for `docgarden`.

### `docgarden skills match <QUERY>`

`docgarden skills match <QUERY>` should behave like `docgarden match <QUERY>`, but restricted to the configured skills directory and skill front matter schema.

The initial matching fields should likely be:

- `name`
- `description`
- path
- optional compatibility metadata

This command should help answer questions like:

- which existing skill should handle this task
- whether the repository already has a skill for a workflow
- which skill names or descriptions overlap with a given topic

Because the skill body is only relevant after a skill has been selected, this command should stay focused on the metadata that determines triggering rather than on full-text body search.

## Optional Index Files

This proposal does not reject index files entirely.

There are valid cases where a repository may want curated index-style files:

- an existing repository or imported workflow already uses them
- the index provides a narrative entrypoint rather than a mechanical catalog
- a team wants a human-authored overview for a document family
- someone starts from a workflow like the LLM wiki gist, where an index page is part of the pattern

The current direction is:

- frontmatter-driven discovery should be the default
- curated index files may be supported as optional configuration
- index files should be treated as overlays, not mandatory parallel inventories

That means future rules might validate configured index files when they exist, but repositories should not need them just to get discovery.

## Interaction With Front Matter Policy

This proposal depends on standardized front matter being meaningful enough to support discovery.

That raises a product requirement: front matter cannot be treated as perfunctory metadata. Repositories that want strong discovery need prompts, skills, or local guidance that tell agents to write titles and descriptions that are specific enough to trigger later use.

This also argues for document-family-aware front matter schemas rather than one universal metadata blob. Different families may need different discovery fields, but each family should still provide a compact, high-signal trigger surface.

## Interaction With Context Budgets

Metadata-driven discovery complements context-budget limits.

If `docgarden list`, `tree`, and `match` can expose the right files from front matter alone, agents can avoid loading full document bodies too early. That supports the broader progressive-disclosure model:

- small routing surface first
- deeper documents only when needed

This is especially attractive for high-traffic document families such as skills, design docs, plans, and references.

## Open Questions

- Should `match` stay strictly limited to front matter, or should it eventually support an opt-in second tier that consults headings only?
- Which document families should be included in discovery by default, and which should require explicit configuration?
- What minimum metadata is required before a file is eligible for `list`, `tree`, or `match` output?
- Should `match` ranking be purely field-presence and keyword based, or should it include deterministic weighting such as giving `title` matches more weight than `description` matches?
- Should repositories be able to define canonical discovery roots or document families so `tree` and `list` do not need to scan every Markdown file?
- If curated index files are configured, should `docgarden` validate only their links, or also compare them against frontmatter-derived inventories for drift?
