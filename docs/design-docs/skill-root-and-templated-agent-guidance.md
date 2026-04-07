# Skills Directory And Templated Agent Guidance

## Purpose

This document is a short working design draft for how `docgarden` should handle skills_directory configuration and how that configuration should feed generated agent guidance such as a Doc Gardener skill.

## Why This Needs A Separate Design

Two product needs intersect here:

- `docgarden skills ...` commands need a canonical skills directory
- repositories may want generated agent guidance that is templated to the repo's configured paths and policy

Those are related, but they are not the same as a broader document-family configuration model.

## Skills Directory As Repo-Wide Config

`docgarden` should likely have a repo-wide top-level config for the skills directory.

A plausible shape is:

    skills_dir = ".agents/skills"

This is similar to `path_style = "backticks"`: it is a foundational repository convention that should be easy to read without requiring an explicit catch-all scope entry.

The main reason is ergonomics. The `docgarden skills list` and `docgarden skills match <QUERY>` commands need a default place to look, and requiring every repository to express that only through a `[[documents]]` entry feels too indirect.

The name `skills_dir` may be clearer than `skills_root` because the configured value is a directory path, not a repository root. The final spelling should follow the rest of the new `docgarden` configuration by using snake_case in TOML.

## Inferred Skills Scope

Once the skills directory exists, `docgarden` can infer a built-in skills scope for rule and discovery purposes.

The practical direction is:

- `skills_dir` is enough to make `docgarden skills ...` commands work
- skills validation rules apply automatically under that directory
- rule configuration may later target the built-in `skills` scope explicitly when users need overrides

That keeps the common case ergonomic without requiring a public generic grouping layer first.

## Templated Agent Guidance

The same configuration that drives linting should also be able to drive generated agent guidance.

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

## Interactive Versus Scriptable Init

If `init` takes on template generation, it should remain scriptable.

Interactive prompting can be useful when a human is setting up a repository for the first time, but quiz-only setup would be awkward for automation and for agent-driven bootstrap flows.

The safer direction is:

- support interactive prompts when running in a TTY
- also support non-interactive operation from flags and existing config
- treat generated files as repository-owned outputs that users can edit afterward

## Current Direction

The current design direction is:

- add a repo-wide skills directory config
- prefer the public TOML spelling `skills_dir` unless a better name emerges
- let that config power `docgarden skills ...` commands directly
- infer a built-in `skills` scope from that directory for default skill validation
- treat generated agent guidance as a derived artifact rendered from configuration
- let a future `docgarden init` write bundled, templated skill files and related guidance into the repository

## Open Questions

- Should the public key be `skills_dir`, `skills_root`, or something else?
- Should the skills directory be a single path, or should repositories eventually support multiple skill directories?
- Should the inferred `skills` scope exist only internally, or should it be surfaced in diagnostics and command output?
- How should repositories mark configured raw/source-derived paths or scopes so generated skills can tell agents not to edit them?
- Should generated skills be fully regenerated on demand, or only scaffolded once and then left entirely to repository owners?
- Which generated artifacts belong in `init`, and which should remain manual or opt-in?
