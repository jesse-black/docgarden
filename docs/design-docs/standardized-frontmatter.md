---
description: "Working design draft for standardized YAML frontmatter in agent-oriented Markdown documents, including first-party `description` requirements, scope-specific schemas, and README or AGENTS exceptions; read when designing frontmatter policy or linting rules."
---

# Standardized Frontmatter

## Purpose

This document is a working draft for standardized YAML front matter in agent-oriented Markdown documents that `docgarden` may validate across repositories.

The draft starts with two scopes:

- imported references for externally sourced material
- skills under the configured skills directory

## Naming Convention

For `docgarden`-defined metadata, prefer `snake_case`.

For external schemas, preserve the field names defined by that schema. For example, Agent Skills uses `allowed-tools`, so this draft keeps that spelling for `skills`.

## Validation Direction

`docgarden` should lint required front matter fields for scopes that opt into front matter requirements.

Those requirements should vary by scope rather than forcing one universal schema across all Markdown files. Different document types have different needs:

- first-party repository docs should require `description` on all Markdown documents except `README.md` and any `AGENTS.md`
- imported external references may require provenance-focused fields such as `title`, `source`, `retrieved`, and `last_reviewed`
- some files may intentionally require no front matter at all when token efficiency or file role matters more than metadata uniformity

For first-party repository docs, `description` is the current default requirement because it provides the highest-value discovery and routing signal for agents at the lowest maintenance cost.

`README.md` and any `AGENTS.md` are the clearest exceptions. They are high-traffic entry-point documents, and any front matter requirement there should justify its token cost.

`README.md` also has a separate compatibility concern. GitHub's official documentation describes repository and profile READMEs as Markdown surfaces shown directly to visitors, while GitHub documents YAML front matter separately for GitHub Docs authoring and GitHub Pages or Jekyll sites. That makes front matter in repository `README.md` an uncertain fit for the repository front-page contract, so this draft keeps `README.md` out of the repository-wide requirement.

The decision on requiring `last_reviewed` for first-party repository docs remains pending. It may be valuable for freshness-sensitive scopes, but this draft does not yet require it repository-wide.

## Parser Direction

`docgarden` should not depend on a general-purpose YAML stack just to support frontmatter.

The product needs a small, deterministic parser that supports two usage modes:

- linting, where the full document may already be loaded in memory
- discovery or matching workflows, where `docgarden` should be able to read only the file prefix needed to parse frontmatter and avoid loading the Markdown body

The current direction should be one shared frontmatter parser with two entry points over the same logic:

- a streaming or buffered-reader entry point that reads only enough bytes to parse the frontmatter block or determine that no valid frontmatter block exists
- an in-memory entry point that operates on a full document string without reparsing through a separate implementation

Do not build separate "linter parser" and "matcher parser" implementations. That would create drift in accepted syntax, malformed-data handling, and field normalization. Discovery and linting should agree on what counts as valid frontmatter.

The parser should be strict enough that the linter can report malformed frontmatter which would otherwise make discovery results unreliable.

## Minimal Supported YAML Subset

The first implementation should support only the subset needed for repository frontmatter and frontmatter-driven discovery.

Treat frontmatter as present only when all of these are true:

- the file begins with a line that is exactly `---`
- a closing line that is exactly `---` appears before any non-frontmatter body content
- the content between those delimiters parses under the supported subset below

Any later `---` in the body is ordinary Markdown content, not frontmatter.

### Supported structure

- top-level mapping only
- nested mappings for field groups such as `metadata`
- scalar values:
  - plain strings on a single line
  - booleans `true` and `false`
  - integers in base 10
  - dates in ISO `YYYY-MM-DD` form, initially treated as strings unless or until a schema opts into stricter date typing
- sequences introduced with `- `, primarily for string lists

### Explicitly unsupported in v1

- anchors and aliases
- tags
- block scalars such as `|` and `>`
- flow-style collections such as `[a, b]` or `{a: b}`
- multi-document YAML
- duplicate keys in the same mapping
- comments as semantically meaningful content

Unsupported constructs should make the frontmatter invalid rather than falling back to partial parsing.

### Additional constraints

- Keys are case-sensitive.
- For `docgarden`-owned schemas, prefer `snake_case` keys.
- A duplicate key within the same mapping is invalid.
- The parser should preserve field order only if needed for diagnostics or display; semantic interpretation should not depend on order.
- The parser should return byte offsets or line ranges for malformed input when practical so lint diagnostics can point at the broken frontmatter.

## Malformed Frontmatter Handling

The parser should distinguish among these cases:

- no frontmatter present
- valid supported frontmatter present
- malformed or unsupported frontmatter present at the start of the file

That distinction matters because discovery commands should ignore files without frontmatter, while linting should be able to report malformed leading frontmatter that would break discovery or schema validation.

The initial linting model should treat malformed leading frontmatter as its own parse failure class rather than quietly converting it into "missing required fields." A missing field and a syntactically broken frontmatter block are different problems and should remain distinguishable.

## Streaming Behavior

For discovery-oriented commands such as `list`, `tree`, and `match`, `docgarden` should parse frontmatter from the beginning of the file and stop reading once it has:

- parsed a valid closing frontmatter delimiter
- determined that the file does not begin with frontmatter
- or encountered malformed leading frontmatter

This keeps discovery fast and aligned with the broader progressive-disclosure goal: read metadata first, load body text only when a later workflow actually needs it.

## References

Use the `references` schema for externally sourced material such as local captures, summaries, or source-adjacent notes derived from web links.

| Field | Required | Description |
| --- | --- | --- |
| `title` | Yes | Human-readable title for the local reference document. |
| `source` | Yes | Canonical external URL for the referenced material. |
| `description` | Yes | Brief summary of why this reference matters in the repository. |
| `retrieved` | Yes | Date when the external source was fetched, copied, or reviewed for the local reference. |
| `last_reviewed` | No | Date when someone last checked that the local reference remained faithful and useful. |
| `author` | No | Author or publishing organization for the external source. |
| `license` | No | License name or short reference if reuse terms matter for the local copy. |

### Example

    ---
    title: Giving Codex a full observability stack in local dev
    source: https://example.com/post
    description: External reference describing a docs-as-system-of-record workflow and long-running agent feedback loops.
    retrieved: 2026-04-02
    last_reviewed: 2026-04-02
    author: Example Organization
    license: CC BY 4.0
    ---

## Skills

Use the `skills` schema for files that implement the Agent Skills specification.

| Field | Required | Description |
| --- | --- | --- |
| `name` | Yes | Max 64 characters. Lowercase letters, numbers, and hyphens only. Must not start or end with a hyphen. |
| `description` | Yes | Max 1024 characters. Non-empty. Describes what the skill does and when to use it. |
| `license` | No | License name or reference to a bundled license file. |
| `compatibility` | No | Max 500 characters. Indicates environment requirements such as intended product, system packages, or network access. |
| `metadata` | No | Arbitrary key-value mapping for additional metadata. |
| `allowed-tools` | No | Space-delimited list of pre-approved tools the skill may use. Experimental. |

### Example

    ---
    name: doc-link-audit
    description: Checks repository Markdown for broken local links and reports likely repair locations.
    compatibility: Requires ripgrep and repository-local file access. No network access required.
    allowed-tools: rg sed
    ---

## Notes

- This draft does not yet define a full canonical `docgarden` schema for first-party design docs, plans, or generated documents beyond the repository-wide `description` requirement and the explicit `README.md` and `AGENTS.md` exceptions.
- This draft treats `skills` as an external schema based on the Agent Skills specification rather than a `docgarden`-owned vocabulary.
- External-schema fields such as `allowed-tools` or `applyTo` should not be generalized into unrelated scopes without an explicit design decision.
- Required-field linting should be driven by explicit scopes, not by a one-size-fits-all Markdown policy.
- The frontmatter parser should be purpose-built for this supported subset rather than exposing full YAML compatibility by default.

## Open Questions

- What first-party scopes should exist beyond imported references, and which of them should require fields beyond `description`?
- Does `last_reviewed` create enough value to justify its maintenance cost for repo-authored docs, and if so, for which scopes?
- Does an `owner` field create enough value to justify its maintenance cost for repo-authored docs?
- If `owner` exists, what does ownership mean for an agent-oriented repository? Is it a human team, a repository area, a workflow, or something else?
- If a skill is loaded to work on a document, should that skill identity ever appear as the `owner`, or is that mixing execution context with long-lived document stewardship?
