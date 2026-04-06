# Skill Root And Templated Agent Guidance

## Purpose

This document is a short working design draft for how `docgarden` should handle skill-root configuration and how that configuration should feed generated agent guidance such as a Doc Gardener skill.

## Why This Needs A Separate Design

Two product needs intersect here:

- `docgarden skills ...` commands need a canonical skills root
- repositories may want generated agent guidance that is templated to the repo's configured document families and policy

Those are related, but they are not exactly the same as the broader document-family and rule-application configuration model.

## Skill Root As Repo-Wide Config

`docgarden` should likely have a repo-wide top-level config for the skill root.

A plausible shape is:

    skills_root = ".agents/skills"

This is similar to `path_style = "backticks"`: it is a foundational repository convention that should be easy to read without requiring an explicit catch-all document family entry.

The main reason is ergonomics. The `docgarden skills list` and `docgarden skills match <QUERY>` commands need a default place to look, and requiring every repository to express that only through a `[[documents]]` entry feels too indirect.

## Inferred Family Versus Explicit Family

Once `skills_root` exists, `docgarden` has to decide whether that automatically creates a skill document family for rule and discovery purposes.

The most practical direction is probably:

- `skills_root` is enough to make `docgarden skills ...` commands work
- `docgarden` may infer a built-in `skills` family from that root for default behavior
- explicit `[[documents]]` or `[[rules]]` entries can still override or refine that inferred family

That keeps the common case ergonomic without preventing repositories from taking full control.

## Templated Agent Guidance

The same configuration that drives linting should also be able to drive generated agent guidance.

For example, a repository may want a Doc Gardener skill that tells an agent:

- where skills live
- which document families are repo-authored
- which document families are imported raw sources and must never be modified in place
- which path style the repository uses
- which `docgarden` commands to run for validation and repair

That guidance should not be hard-coded to this repository's layout. It should be templated from `docgarden.toml`.

For example, if a repository configures a raw/reference family under some path other than `docs/references/`, the generated skill should tell the agent to avoid modifying that configured raw family rather than baking in one repository-specific path.

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

- add a repo-wide `skills_root` config
- let that config power `docgarden skills ...` commands directly
- likely infer a default `skills` family from `skills_root`, while allowing explicit config to override it
- treat generated agent guidance as a derived artifact rendered from configuration
- let a future `docgarden init` write bundled, templated skill files and related guidance into the repository

## Open Questions

- Should `skills_root` be a single path, or should repositories eventually support multiple skill roots?
- Should the inferred `skills` family exist only internally, or should it be surfaced in diagnostics and command output as a first-class family?
- How should repositories mark configured raw/source-derived families so generated skills can tell agents not to edit them?
- Should generated skills be fully regenerated on demand, or only scaffolded once and then left entirely to repository owners?
- Which generated artifacts belong in `init`, and which should remain manual or opt-in?
