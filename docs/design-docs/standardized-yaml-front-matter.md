# Standardized YAML Front Matter

## Purpose

This document is a working draft for standardized YAML front matter in agent-oriented Markdown documents that `docgarden` may validate across repositories.

The draft starts with two document families:

- `references` for externally sourced material
- `skills` for Agent Skills files

## Naming Convention

For `docgarden`-defined metadata, prefer `snake_case`.

For external schemas, preserve the field names defined by that schema. For example, Agent Skills uses `allowed-tools`, so this draft keeps that spelling for `skills`.

## Validation Direction

`docgarden` should lint required front matter fields for document families that opt into front matter requirements.

Those requirements should vary by document family rather than forcing one universal schema across all Markdown files. Different document types have different needs:

- some first-party repository docs may require fields such as `description`, `owner`, or `last_reviewed`
- imported external references may require provenance-focused fields such as `title`, `source`, `retrieved`, and `last_reviewed`
- some files may intentionally require no front matter at all when token efficiency or file role matters more than metadata uniformity

`AGENTS.md` is the clearest example of a file that may intentionally have no required front matter. It is a high-traffic routing document, and any front matter requirement should justify its token cost.

## References

Use the `references` schema for externally sourced material such as local captures, summaries, or source-adjacent notes derived from web links.

| Field | Required | Description |
| --- | --- | --- |
| `title` | Yes | Human-readable title for the local reference document. |
| `source` | Yes | Canonical external URL for the referenced material. |
| `description` | Yes | Brief summary of why this reference matters in the repository. |
| `retrieved` | Yes | Date when the external source was fetched, copied, or reviewed for the local reference. |
| `last_reviewed` | Yes | Date when someone last checked that the local reference remained faithful and useful. |
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

- This draft does not yet define a canonical `docgarden` schema for first-party design docs, plans, or generated documents.
- This draft treats `skills` as an external schema based on the Agent Skills specification rather than a `docgarden`-owned vocabulary.
- External-schema fields such as `allowed-tools` or `applyTo` should not be generalized into unrelated document families without an explicit design decision.
- Required-field linting should be driven by document family, not by a one-size-fits-all Markdown policy.

## Open Questions

- What first-party document families should exist beyond `references`, and which of them should require front matter at all?
- Should `AGENTS.md` explicitly remain front-matter-free by default, or should front matter be optional but discouraged there?
- Does an `owner` field create enough value to justify its maintenance cost for repo-authored docs?
- If `owner` exists, what does ownership mean for an agent-oriented repository? Is it a human team, a repository area, a workflow, or something else?
- If a skill is loaded to work on a document, should that skill identity ever appear as the `owner`, or is that mixing execution context with long-lived document stewardship?
