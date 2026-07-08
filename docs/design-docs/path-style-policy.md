---
description: "Working design draft for repository-local path presentation styles in Markdown prose, including backticks, Markdown links, and possible future wiki links; read when deciding path style policy, lint tradeoffs, or representation-focused rule boundaries."
---

# Path Style Policy

## Purpose

This document is a working design draft for how `docgarden` should think about repository-local path presentation styles in Markdown prose.

The immediate question is not only how to lint path validity, but which path *representation* a repository wants to standardize:

- backticked repo-relative paths such as `docs/PLANS.md`
- Markdown links such as `[docs/PLANS.md](docs/PLANS.md)`
- wiki-style links such as hypothetical `[[Path Style Policy]]`

This document focuses on the tradeoffs between those styles and on the product boundary between current support and possible future support.

## Scope

This document is about repository-local path style in agent-facing Markdown prose.

It is not a complete design for all Markdown linking behavior, and it does not redefine the path-classification details already covered in `docs/design-docs/backtick-path-classification.md`.

It also does not treat imported external reference captures as normal style-managed docs. Raw source captures should not be rewritten merely to satisfy local path-style preferences.

## Why This Matters

In an agent-oriented repository, path style is not just cosmetic.

It affects:

- token cost in high-traffic docs
- what agents generate instinctively in prose
- how clearly a path reads as a path versus a reader-oriented hyperlink
- whether a representation aligns with standard Markdown tooling
- how much autofix can be done safely and mechanically

This is why `docgarden` already treats local reference style as a configurable policy surface instead of a one-size-fits-all formatting preference.

## Backticks

Backticks are the repository's current default style for plain local path mentions in prose.

Strengths:

- compact and token-efficient when the visible text would otherwise just repeat the destination
- naturally emitted by agents when naming files in prose
- visually reads as a path-like code token instead of presentation-oriented hyperlink text
- easy to preserve in environments where rendered link UX does not matter
- potentially compatible with editor extensions that make backtick paths clickable without changing the underlying repository text

Weaknesses:

- overloaded with many non-path meanings, so classification requires extra heuristics
- not clickable in plain Markdown renderers unless tools add special handling
- can be ambiguous for humans when the same syntax is also used for commands, identifiers, and config keys

An Obsidian extension could make backtick paths clickable for human navigation, but that is not the same product surface as native wikilinks. Obsidian's graph, backlinks, and metadata workflows are built around Obsidian's own internal link model. A backtick-path extension would need to maintain its own parallel navigation model unless Obsidian exposes a way to add nonstandard links into the native metadata cache. Relevant references:

- https://github.com/obsidianmd/obsidian-api
- https://forum.obsidian.md/t/api-method-to-add-link-and-have-it-parsed-into-metadatacache/72046

Backticks are especially attractive for agent-first repositories where many references function more like symbolic path mentions than like human-targeted navigation affordances.

## Markdown Links

Markdown links make the repository-local navigation intent explicit.

Strengths:

- standard Markdown representation with broad editor and renderer support
- clearer distinction between link targets and ordinary inline code
- more obviously navigational for humans reading rendered docs
- easier to validate as links because the destination is explicit and unambiguous

Weaknesses:

- more verbose and more expensive in tokens when the label merely repeats the destination
- agents often do not choose this form by default for plain file mentions, necessitating strict guidance in AGENTS.md or Skills that can still be ignored
- a path-repeating label can feel redundant when no human-friendly label is needed

Markdown links are strongest when the label adds meaning beyond repeating the path or when the repository optimizes for rendered-document navigation over raw-text compactness.

## Wikilinks

Wiki-style links are the native representation for Obsidian-style Markdown wikis and LLM-maintained wiki workflows like [llm-wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f).

Strengths:

- compact compared with path-repeating Markdown links
- highly aligned with wiki-oriented graph navigation and backlink workflows
- familiar to users building Obsidian-native or Zettelkasten-like knowledge systems
- enables a human user to open the repository in Obsidian and navigate it as a wiki-like knowledge base
- aligns with the workflow described in [llm-wiki](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f), where Obsidian is the human-facing IDE and the LLM maintains an interlinked Markdown wiki

Weaknesses:

- not part of standard Markdown
- constrained by parser support: the `markdown-rs` library used by `docgarden` has a closed-as-not-planned issue for core wikilink support ([wooorm/markdown-rs#62](https://github.com/wooorm/markdown-rs/issues/62)), so native support requires workarounds or future extension hooks rather than a straightforward core-parser option
- less portable across editors, renderers, and repository-hosting environments
- usually requires repository-specific parsing and resolution semantics
- typically encodes page identity rather than literal repository path identity
- requires a resolution layer that maps page identity, basename, title, or alias to an actual file path rather than directly naming the repository path in the link text

Wikilinks are therefore not a third spelling of repository paths. They represent a different model: page identity in a Markdown wiki.

### Path To Enabling Wikilinks

Supporting wikilinks in `docgarden` requires more than a parser tweak.

#### Parser And AST Support

There is an open issue about custom plugins or extensions in `markdown-rs` ([wooorm/markdown-rs#32](https://github.com/wooorm/markdown-rs/issues/32)). That is the most plausible upstream path for proper parser integration, but `docgarden` should not model wikilinks as a near-term parser-flag change.

That said, `markdown-rs` is an active open source project with explicit contribution guidance in the repository, so if wikilink support becomes important enough for `docgarden`, one possible path would be to contribute the extension-hook capability discussed in [wooorm/markdown-rs#32](https://github.com/wooorm/markdown-rs/issues/32) and then build a wikilinks plugin or equivalent integration on top of it.

#### Local Resolution Cache Or Index

Even with parser support, `docgarden` cannot treat wikilinks as ordinary repository-path strings.

It would need a local cache or index that can resolve wiki identity to actual files. At minimum, that means tracking some combination of:

- canonical page title
- basename or stem
- configured aliases
- file path
- built-in scope or future named group
- headings or section anchors if deep wikilinks are in scope

That index must be built deterministically from repository contents and configuration, not from an editor-private database. In practice, `docgarden` would scan configured scopes, parse front matter and selected headings, and build its own mapping layer for link resolution, rename checks, and ambiguity detection.

#### Agent Ergonomics

Wikilinks also create an agent-usage problem.

Agents can read and emit raw Markdown, but they cannot mechanically follow a wikilink such as `[[Path Style Policy]]` unless they also have a tool or local index that resolves that identity to a concrete file path. Backticks and Markdown links are directly actionable from the filesystem alone; wikilinks require wiki-aware navigation support.

The LLM wiki reference points at [`qmd`](https://github.com/tobi/qmd) as one example of that support layer. `qmd` is a local Markdown search and retrieval tool with CLI and MCP surfaces for agent use. It is useful inspiration because it treats navigation as an agent tool problem, not only as a Markdown syntax problem.

Two plausible implementation directions are:

- a gitignored local cache plus a resolver command such as a future `docgarden` subcommand that maps wikilink identity to file path
- a persisted on-filesystem index or database file that agents can read or query directly

Both approaches are technically workable, but both give up one of the nicest properties of backticks and Markdown links: those forms are self-describing and directly actionable from the repository filesystem without extra repository-specific tooling.

So the main cost of wikilinks is not only parser work or cache maintenance. It is also reduced default agent legibility. A repository that adopts wikilinks needs to teach agents through `AGENTS.md` or equivalent repo guidance:

- that wikilinks are part of the repository convention
- how to resolve them
- which tool or file to consult
- when any local cache or index must be refreshed

That makes wikilinks a higher-complexity opt-in mode rather than a natural default for generic agent workflows.

## Current Product Boundary

Today `docgarden` validates local Markdown links by default and supports opt-in style enforcement for backtick paths.

- `unresolved-link-path` fires by default for any local Markdown link that does not resolve within the repository, including heading fragments generated using GitHub-compatible slug semantics.
- `unresolved-backtick-path` is opt-in via `[[rules]].enable` and fires when a backtick path does not resolve, using entry-level `severity`.
- `prefer-links-for-local-paths` is opt-in via `[[rules]].enable` and rewrites backtick paths to Markdown links for repositories that prefer navigable link syntax.

Wikilinks are not just another spelling of the same feature. Supporting them would require explicit decisions about syntax, resolution rules, portability, and whether the product is still enforcing repository paths or has expanded into wiki-page identity.

The product question is therefore:

- Is `docgarden` a repository knowledge-base linter for git repos, where local references primarily mean repo-relative paths?
- Or is `docgarden` also a Markdown wiki tool for Obsidian-style knowledge bases, where local references primarily mean wiki page identities?

Those use cases overlap, but they optimize for different workflows. A repository knowledge base wants portable, filesystem-obvious references that agents can inspect and modify with standard shell tools. An Obsidian knowledge base wants graph navigation, backlinks, aliases, page identities, and wiki-aware tooling.

## Raw Sources Versus Repo-Authored Docs

Path-style policy should apply to repository-authored documents, not to raw imported sources.

Imported external material should be treated as source-derived input rather than as style-normalized first-party prose, even when those files contain path-shaped text or alternate link styles.

This distinction matters for configuration:

- repo-authored scopes may opt into `prefer-links-for-local-paths` enforcement
- imported reference scopes should use `disable = ["unresolved-link-path"]` to relax link resolution

## Open Questions

- Should `docgarden` stay focused on repository knowledge bases, or should it also become a Markdown wiki tool for Obsidian-style knowledge bases?
- If wikilinks are supported, should they live behind a separate wiki mode rather than inside the ordinary repository-path style policy?
- Should wikilink resolution use document identities, scope-specific titles, aliases, or explicit repository paths as the canonical target model?
- How much rendered-document ergonomics should matter relative to token efficiency in the default product posture?
