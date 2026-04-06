# Configuration

## Purpose

This document is a working design draft for how `docgarden` should describe repository document families and how rule behavior should be configured against them.

The main goal is to avoid baking one repository's directory layout into product behavior. Features should operate on configured document families and configured scopes, with repo-local paths serving only as examples.

## Why This Needs Its Own Design

Several `docgarden` feature areas depend on the same underlying question:

- discovery commands need to know which documents belong to which family
- front matter validation needs to know which schema applies where
- context-budget defaults need to know which limits apply to which files
- imported-reference policy needs to know which paths are source-derived and which are repo-authored
- optional curated indexes need to know which family they describe

If each feature invents its own path-based configuration, the product will fragment quickly.

This should instead be a shared substrate for multiple features.

## Core Direction

`docgarden` should understand configured document families, not only raw path globs.

A repository may define families such as:

- references
- plans
- design docs
- generated docs
- skills

These families can then drive:

- discovery commands such as `list`, `tree`, and `match`
- family-specific front matter validation
- context-budget defaults
- optional curated-index validation
- imported-reference policy
- future rule targeting and reporting

For example, this repository may map a reference family to `docs/references/`, but another repository might use `knowledge/raw/` or `sources/`. The feature should operate on the configured family, not on the literal path name.

## Why Not One-Off Directory Keys

A top-level configuration key such as `raw_directory = "docs/references"` is easy to understand at first, but it does not scale well.

That approach tends to multiply into more one-off keys over time:

- `skills_directory`
- `references_directory`
- `plans_directory`
- `generated_directory`

This creates two problems:

- feature-specific configuration becomes inconsistent
- shared behavior across document families becomes harder to express

The stronger direction is to define families once and let multiple features reuse them.

## Document-Family Layer

The first configuration layer should declare the repository's document families and how they are identified.

The exact syntax is still open, but the conceptual model is:

- define a family name
- identify which files belong to it
- optionally declare a kind or schema reference
- optionally attach family-level metadata

For path matching, gitignore-style globs should be treated as the standard.

This matches the current implementation direction in `docgarden`, where include, exclude, and per-file ignore behavior already rely on gitignore-style pattern matching rather than on a separate glob dialect.

A repository-local example might look like:

    [[documents]]
    name = "references"
    match = "docs/references/**"
    kind = "reference"

    [[documents]]
    name = "skills"
    match = ".agents/skills/*/SKILL.md"
    kind = "skill"

The important point is not the final syntax. The important point is that the repository defines portable concepts first, then features use those concepts.

## Rule-Application Layer

The second configuration layer should describe how rule sets apply to families or paths.

This is broader than a narrow `[[ignore]]` table.

The product likely needs a way to:

- enable rules
- disable rules
- override defaults
- scope behavior to one family or path subset

That suggests a general rule-application table, perhaps something like `[[rules]]`, rather than a config model that can only express ignores.

A conceptual example:

    [[rules]]
    match = "references"
    disable = ["unresolved-local-path"]

    [[rules]]
    match = "skills"
    enable = ["frontmatter", "matchable-metadata", "context-budget"]

This keeps exceptions and positive policy in one configuration family instead of splitting them across unrelated tables.

Rule-specific options should also live in this layer instead of growing separate top-level tables for each feature.

For example, context-budget limits should be expressed as configuration for the `context-budget` rule, not as a separate `[[limits]]` table with its own scoping model:

    [[rules]]
    match = "skills"
    rule = "context-budget"
    max-lines = 500
    max-tokens = 5000
    severity = "warn"

    [[rules]]
    match = "AGENTS.md"
    rule = "context-budget"
    max-tokens = 1200
    severity = "error"

    [[rules]]
    match = "references"
    rule = "context-budget"
    enabled = false
    reason = "Imported source-derived docs preserve source fidelity over compactness."

The exact field names are still open, but the direction is that rule behavior and rule options share the same targeting layer. This lets budget checks reuse configured document families, keeps severity and exception reasons close to the rule they affect, and avoids a second path-pattern language that would duplicate `[[documents]]` and `[[rules]]`.

## Repository-Wide Defaults

Configured document families and rule-application entries are not a complete replacement for repo-wide defaults.

Some policy choices are foundational enough that repositories should be able to state them once at the top level instead of expressing them indirectly through a catch-all family or a broad rule entry. A local path style default is the clearest example: repositories may want to say "this is a backticks repo" or "this is a links repo" as a global convention, then override that default only for selected families.

The current design direction should therefore be layered:

- repository-wide defaults establish the main posture for the repo
- configured document families describe which document groups exist
- rule-application entries refine or override behavior for narrower scopes

More specific scopes should win over the repo-wide default when they conflict.

For the path-style tradeoffs behind that example, see `docs/design-docs/path-style-policy.md`.

A conceptual example:

    path_style = "backticks"

    [[documents]]
    name = "references"
    match = "docs/references/**"
    kind = "reference"

    [[rules]]
    match = "references"
    disable = ["prefer-backticks-for-local-paths", "prefer-links-for-local-paths"]
    reason = "Imported source-derived docs are not normalized for repo-authored style."

## `reason` For Exceptions

Exception-oriented configuration should likely support a `reason` field.

This is especially valuable when a repository disables or relaxes rules for a family or path scope. A short human-readable reason makes the policy easier to review, easier for agents to preserve intentionally, and easier to revisit later when the repository evolves.

The strongest case for `reason` is disable or override behavior, not ordinary default rule application.

A plausible example:

    [[rules]]
    match = "references"
    disable = ["unresolved-local-path"]
    reason = "Imported source-derived docs may contain hypothetical or external paths that should not be treated as repository errors."

The current design direction is:

- `reason` should exist for exception-oriented entries
- `reason` does not need to be required for ordinary positive rule application
- a future lint rule could warn when broad disable entries omit a reason

## Relationship To Discovery Commands

Discovery commands should depend on this configuration model rather than reinventing their own scope configuration.

That means:

- `docgarden list`, `tree`, and `match` should operate over configured discovery families or configured knowledge roots
- `docgarden skills list` and `docgarden skills match` should operate over the configured skill family or skill root
- optional curated indexes should be declared against families rather than assumed from path naming alone

This keeps command behavior portable across repositories with different structures.

## Relationship To Front Matter Policy

Front matter validation should also reuse document-family configuration.

That means the product can say things like:

- the `references` family requires provenance fields
- the `skills` family uses the Agent Skills schema
- some families require no front matter by default

This is cleaner than attaching schema behavior to arbitrary path conventions scattered across features.

## Relationship To Imported References

Imported-reference behavior should be described in terms of configured source-derived families, not hard-coded directories.

In this repository, `docs/references/` is one example of such a family. Another repository might use a different path entirely.

The rule behavior should target the configured family, not the example path.

## Open Questions

- What is the smallest useful family-declaration syntax that still supports multiple features?
- Should the family declaration use `name`, `kind`, both, or different terminology?
- Should family membership be defined only by gitignore-style path patterns at first, or should explicit roots and file-role markers also be supported?
- Should rule-application entries target family names, path patterns, or both?
- Should `reason` be optional but recommended for disable or override entries, or required for any configuration that relaxes enforcement?
- Should discovery-specific configuration live inside family declarations, or should commands infer behavior entirely from family metadata plus rule configuration?
