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

It also does not treat imported external reference captures as normal style-managed docs. Files under `docs/references/` are raw source material in this repository and should not be rewritten merely to satisfy local path-style preferences.

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

An important caveat for Obsidian-style workflows is that "make backticks clickable" and "make backticks behave like native wikilinks" are probably different levels of integration.

The public Obsidian plugin API clearly exposes the vault and `MetadataCache`, and plugins can save their own data, but it is not clear from the documented API that a plugin can register a custom link syntax and have it participate in Obsidian's native backlink, graph, and metadata indexing pipeline the same way true internal links do. A forum request asking for a way to add links into `metadataCache` suggests this is a real limitation rather than just a documentation gap. See:

- https://github.com/obsidianmd/obsidian-api
- https://forum.obsidian.md/t/api-method-to-add-link-and-have-it-parsed-into-metadatacache/72046

So an Obsidian extension for backtick paths may still be very useful for human navigation, but it may need to maintain a parallel plugin-managed system rather than hooking backtick paths into the same IndexedDB-backed relationship model that powers native wikilinks.

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

Wiki-style links are common in tools such as Obsidian and in knowledge-base workflows inspired by personal wiki systems.

Strengths:

- compact compared with path-repeating Markdown links
- highly aligned with wiki-oriented graph navigation and backlink workflows
- familiar to users building Obsidian-native or Zettelkasten-like knowledge systems
- enables a human user to open the repository in Obsidian and navigate it as a wiki-like knowledge base
- aligns with the workflow described in `docs/references/llm-wiki.md`, where the wiki is browsed in Obsidian as the "IDE" while the LLM maintains the files

Weaknesses:

- not part of standard Markdown
- constrained by parser support: the `markdown-rs` library used by `docgarden` has a closed-as-not-planned issue for core wikilink support ([wooorm/markdown-rs#62](https://github.com/wooorm/markdown-rs/issues/62)), so native support would likely require workarounds or future extension hooks rather than a straightforward core-parser option
- less portable across editors, renderers, and repository-hosting environments
- usually requires repository-specific parsing and resolution semantics
- typically encodes page identity rather than literal repository path identity
- requires a resolution layer that maps page identity, basename, title, or alias to an actual file path rather than directly naming the repository path in the link text

Wikilinks are therefore appealing for some repository knowledge systems, but they are a broader product decision than simply choosing between backticks and standard Markdown links.

### Path To Enabling Wikilinks

Supporting wikilinks in `docgarden` would likely require more than a parser tweak.

#### Parser And AST Support

There is an open issue about custom plugins or extensions in `markdown-rs` ([wooorm/markdown-rs#32](https://github.com/wooorm/markdown-rs/issues/32)), which suggests a possible future path, but there do not appear to be immediate plans to add that capability in the core library today.

That said, `markdown-rs` is an active open source project with explicit contribution guidance in the repository, so if wikilink support becomes important enough for `docgarden`, one possible path would be to contribute the extension-hook capability discussed in [wooorm/markdown-rs#32](https://github.com/wooorm/markdown-rs/issues/32) and then build a wikilinks plugin or equivalent integration on top of it.

#### Local Resolution Cache Or Index

Even with parser support, `docgarden` could not treat wikilinks as ordinary repository-path strings.

It would need a local cache or index that can resolve wiki identity to actual files. At minimum, that likely means tracking some combination of:

- canonical page title
- basename or stem
- configured aliases
- file path
- document family or scope
- maybe headings or section anchors if deep wikilinks are ever in scope

That index would need to be built deterministically from repository contents and configuration, not from an editor-private database. In practice, this probably means `docgarden` would scan the configured document families, parse front matter and maybe selected headings, and build its own mapping layer for link resolution, rename checks, and ambiguity detection.

#### Agent Ergonomics

Wikilinks also create an agent-usage problem.

Agents can read and emit raw Markdown, but they cannot mechanically follow a wikilink such as `[[Path Style Policy]]` unless they also have a tool or local index that resolves that identity to a concrete file path. Backticks and Markdown links are directly actionable from the filesystem alone; wikilinks are not.

So if `docgarden` ever supports wikilinks, it would likely also need to provide tool-level support for resolving them, listing candidates, and maybe rendering canonical targets for agent use. Otherwise the repository could be pleasant for a human inside Obsidian while still being awkward for agents operating only on raw files.

This also creates a useful skepticism about the workflow described in `docs/references/llm-wiki.md`. The gist strongly suggests an Obsidian-oriented environment, but it does not explain how agents navigate and maintain wikilinks without a supporting tool of their own. That absence does not prove Karpathy is not using wikilinks, but it does mean the public description leaves an implementation gap that `docgarden` would need to solve explicitly if it wanted first-class wikilink support.

Two plausible implementation directions are:

- a gitignored local cache plus a resolver command such as a future `docgarden` subcommand that maps wikilink identity to file path
- a persisted on-filesystem index or database file that agents can read or query directly

Both approaches are technically workable, but both also weaken one of the nicest properties of backticks and Markdown links: those forms are self-describing and directly actionable from the repository filesystem without extra repository-specific tooling.

So the main cost of wikilinks is not only parser work or cache maintenance. It is also reduced default agent legibility. A repository that adopts wikilinks would likely need to teach agents, probably through `AGENTS.md` or equivalent repo guidance:

- that wikilinks are part of the repository convention
- how to resolve them
- which tool or file to consult
- when any local cache or index must be refreshed

That makes wikilinks feel like a higher-complexity opt-in mode rather than a natural default for generic agent workflows.

## Current Product Boundary

Today `docgarden` supports a local reference style policy of `backticks` or `links`.

That boundary is pragmatic:

- both forms fit naturally into Markdown AST processing
- both forms can be linted and autofixed mechanically in a repository-local way
- both map cleanly onto the current product focus on repo-relative path integrity

Wikilinks are not just another spelling of the same feature. Supporting them would likely require explicit decisions about syntax, resolution rules, portability, and whether the product is still enforcing repository paths or has expanded into wiki-page identity.

## Raw Sources Versus Repo-Authored Docs

Path-style policy should apply to repository-authored documents, not to raw imported sources.

In this repository, files under `docs/references/` are local captures of external material. Even when they contain path-shaped text or alternate link styles, those files should be treated as source-derived inputs rather than as style-normalized first-party prose.

This distinction matters for future configuration:

- repo-authored document families may opt into a strict path-style policy
- imported reference families may require provenance checks while relaxing path-style rewrites

## Initial Design Direction

The current design direction is:

- keep `backticks` and `links` as the actively supported style-policy options
- preserve the repository's rationale for starting with backticks in agent-first, token-sensitive docs
- treat Markdown links as the main alternative for repositories that prioritize explicit navigation or rendered readability
- treat wikilinks as a separate future design topic rather than as an immediate third style option
- avoid applying style rewrites to imported raw-source families such as `docs/references/`

## Open Questions

- Should `docgarden` continue to model style policy only as `backticks` versus `links`, or should it eventually grow a more general path-style enum that could include wikilinks?
- If wikilinks are ever supported, should they resolve to repository paths, document identities, or family-scoped titles?
- Should repositories be able to define different path-style policies for different configured document families?
- Should style policy be enforced only in repo-authored docs by default, with imported-reference families automatically exempted from rewrites?
- How much rendered-document ergonomics should matter relative to token efficiency in the default product posture?
