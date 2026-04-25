---
description: "Working design draft for `docgarden` skill-specific validation, configured skills directories, and generated agent guidance; read when planning `skills validate`, skills directory config, or skill-related repository guidance generation."
---

# Skills Validation

## Purpose

This document is a short working design draft for `docgarden`'s skill-specific validation and related configuration.

The goal is to make skill collections locally checkable and configurable without forcing every repository to express skills through a generic document-family model first. Skill front matter remains part of this design because it is the natural validation surface and the same metadata that drives discovery elsewhere in the product.

## Why This Needs A Separate Design

Skills are a first-class product concept in `docgarden`.

They sit at the intersection of three needs:

- `docgarden skills validate ...` needs a canonical skills directory
- skill discovery should use the same front matter agents use for triggering, but that discovery design now lives with `list` and `match`
- repositories may want generated agent guidance that is templated to configured paths and policy

Those needs are related, but they are not the same as a broader document-family configuration model. The near-term design should keep the skills path explicit, keep the command surface small, and avoid baking this repository's layout into product behavior.

## Command Family

The proposed command family is:

- `docgarden skills validate <TARGET>`
- later, possibly `docgarden skills init` or `docgarden init --skills`

`validate` is the explicit skill-schema checking command for users who want the built-in skill defaults without applying repository-wide lint policy to their skills directory.

Skill discovery is still part of the product, but it no longer needs a separate `skills list` or `skills match` command family. Keep that discussion in `docs/design-docs/match-and-list.md`, where scope switches such as `docgarden list --skills` and `docgarden match --skills <QUERY>` belong.

## Skills Directory As Repo-Wide Config

`docgarden` should use a repo-wide top-level config for the skills directory.

The public TOML shape is:

    skills_dir = ".agents/skills"

This is similar to `path_style = "backticks"`: it is a foundational repository convention that should be easy to read without requiring an explicit catch-all scope entry.

The main reason is ergonomics. Skill-scoped discovery and validation need a default place to look, and `skills_dir` gives that concept a direct home in configuration.

The key is `skills_dir` because the configured value is a directory path, not a repository root. The spelling follows the rest of the new `docgarden` configuration by using snake_case in TOML.

## Inferred Skills Scope

Once the skills directory exists, `docgarden` can infer a built-in skills scope for validation and discovery purposes.

The practical direction is:

- `skills_dir` is enough to make skill-scoped discovery and `docgarden skills validate ...` work
- skills validation rules can apply automatically under that directory when a repository opts into linting those files
- rule configuration may later target the built-in `skills` scope explicitly when users need overrides

That keeps the common case ergonomic without requiring a public generic grouping layer first.

`skills_dir` remains a top-level setting rather than a member of a generic `[directories]` table. It is a first-class product convention, not just an arbitrary filesystem path.

## `skills validate`

`docgarden skills validate <TARGET>` is the skill-schema checking command. The target is a repository-relative or filesystem path to validate. It may point at a skill's SKILL.md file or at the skill directory containing that file.

The reason is conceptual separation. In `docgarden`, repository linting means "apply repository lint policy to selected files." A user may want to validate a skill package with `docgarden`'s built-in Agent Skills defaults without saying that the skill files are repo-authored documentation subject to the repository's normal lint rules.

That distinction matters for vendored or externally managed skills. For example, a repository may consume skills through an external tool such as `npx skills`. Those files might live in the working tree, but the user may not want `docgarden lint` to rewrite or enforce house-style documentation rules against them.

`docgarden skills validate <TARGET>` should therefore mean:

- resolve the target as a skill file or skill directory
- validate each skill file against the built-in skill schema
- report skill-specific diagnostics without applying unrelated repository documentation lint rules
- avoid safe rewrites unless a future explicit fix mode is added

The built-in schema should follow the [Agent Skills specification](https://agentskills.io/specification).

Structural validity means:

- a directory target contains a SKILL.md file
- a file target is named SKILL.md
- SKILL.md contains YAML front matter followed by Markdown content
- required front matter fields `name` and `description` are present
- optional front matter fields `license`, `compatibility`, `metadata`, and `allowed-tools` are accepted when present
- unknown front matter fields are rejected
- `metadata`, when present, is a mapping
- the `name` value matches the parent skill directory name
- `name` values are normalized with Unicode NFKC before format and directory-name comparison
- file references in the skill body are relative to the skill root

Field and size limits should also be part of validation:

- `name` is 1-64 characters
- `name` contains lowercase Unicode letters, numbers, and hyphens only
- `name` does not start or end with a hyphen
- `name` does not contain consecutive hyphens
- `description` is 1-1024 characters
- `compatibility`, when present, is 1-500 characters
- the main skill file stays under 500 lines
- the main skill file stays under 5000 tokens

Diagnostics should collect all validation errors for the target rather than stopping at the first field error. When the target is a SKILL.md file, validation should resolve the skill root as that file's parent directory before checking the directory-name match.

## Relationship To Repository Linting

Repository linting and skill validation should share parsing and diagnostic infrastructure where practical, but they should not be identical user stories.

`docgarden lint` should continue to mean that the repository has selected files for normal documentation policy. If the repository includes its skills directory in lint selection, skill-specific checks can run as part of that policy.

`docgarden skills validate <TARGET>` should be usable even when the target directory is excluded from normal lint selection. This gives users a clean command for externally sourced or vendored skills:

    docgarden skills validate .agents/skills/pdf-processing
    docgarden skills validate .agents/skills/pdf-processing/SKILL.md

That command should not imply that `.agents/skills/pdf-processing` is part of the repo-authored documentation contract.

## Templated Agent Guidance

The same configuration that drives skills commands should also be able to drive generated agent guidance.

For example, a repository may want a Doc Gardener skill that tells an agent:

- where skills live
- which paths or scopes are repo-authored
- which paths or scopes are imported raw sources and must never be modified in place
- which path style the repository uses
- which `docgarden` commands to run for validation and repair

That guidance should not be hard-coded to this repository's layout. It should be templated from `docgarden.toml`.

For example, if a repository configures imported references under some path other than `docs/references/`, the generated skill should tell the agent to avoid modifying that configured path rather than baking in one repository-specific path.

## `init` As Template Materialization

This suggests a useful role for `docgarden init`.

Rather than only initializing config, `init` could also materialize bundled templates into repository files such as:

- a Doc Gardener skill
- maybe supporting guidance snippets or example config

The executable can bundle those templates and render them using the repository's actual configuration.

That would let `docgarden` provide a stronger out-of-the-box workflow:

- configure repository knowledge structure
- generate agent guidance from that structure
- keep the generated guidance aligned with the actual lint policy

If `init` takes on template generation, it should remain scriptable. Interactive prompting can be useful when a human is setting up a repository for the first time, but quiz-only setup would be awkward for automation and for agent-driven bootstrap flows.

The safer direction is:

- support interactive prompts when running in a TTY
- also support non-interactive operation from flags and existing config
- treat generated files as repository-owned outputs that users can edit afterward

## Current Direction

The current design direction is:

- add a repo-wide skills directory config
- use the public TOML spelling `skills_dir`
- let that config power skill-scoped discovery and validation directly
- keep skill discovery under `docgarden list` and `docgarden match` via scope switches documented in `docs/design-docs/match-and-list.md`
- add `docgarden skills validate <TARGET>` as the built-in skill-schema validation path
- use the Agent Skills structural, character, line, and token limits for `skills validate`
- infer a built-in `skills` scope from the skills directory for default skill validation and future rule targeting
- treat generated agent guidance as a derived artifact rendered from configuration
- let a future `docgarden init` write bundled, templated skill files and related guidance into the repository

## Open Questions

- Should the inferred `skills` scope exist only internally, or should it be surfaced in diagnostics and command output?
- How should repositories mark configured raw/source-derived paths or scopes so generated skills can tell agents not to edit them?
- Should generated skills be fully regenerated on demand, or only scaffolded once and then left entirely to repository owners?
- Which generated artifacts belong in `init`, and which should remain manual or opt-in?
