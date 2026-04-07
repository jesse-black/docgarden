# Configuration

## Purpose

This document is a working design draft for how `docgarden` should configure repository-wide conventions and path-targeted rule behavior.

The main goal is to avoid baking one repository's directory layout into product behavior while keeping the configuration model as small as the current product needs.

## Why This Needs Its Own Design

Several `docgarden` feature areas depend on the same underlying question: which files should a feature operate on?

For near-term features, the most concrete case is skills:

- `docgarden skills list` and `docgarden skills match <QUERY>` need to know where repository-local skills live.
- Skills validation rules should apply automatically to that same directory.

Other future areas may need path-targeted configuration too, including front matter validation, context-budget defaults, imported-reference policy, and optional curated indexes. Those future needs are not enough by themselves to justify a broad generic grouping layer today.

## Core Direction

Prefer first-class configuration for first-class product concepts.

For skills, that means a top-level skills directory setting rather than requiring users to express skills through a generic document-family entry. The exact key name is still open; `skills_dir` may be clearer than `skills_root` because it describes the value as a directory.

    skills_dir = ".agents/skills"

That single setting should be enough for `docgarden skills ...` commands and for default skills validation. Internally, `docgarden` may derive skill-file paths from that directory, but users should not have to define the same directory twice.

## Why Not Generic Groups First

A top-level configuration key such as `raw_directory = "docs/references"` is easy to understand at first, but it does not scale well.

That approach tends to multiply into more one-off keys over time:

- `skills_directory`
- `references_directory`
- `plans_directory`
- `generated_directory`

That concern is real, but it is not enough to justify a generic grouping layer before the product has multiple concrete consumers for it. The near-term design should avoid both extremes:

- do not hard-code this repository's paths into product behavior
- do not introduce broad named groups before they earn their keep

`[[documents]]` may become useful later for user-defined groups such as imported references, generated docs, or curated indexes. It should stay deferred until at least two concrete features need the same named group.

## Rule-Application Layer

The rule-application layer should describe how rule behavior applies to explicit repository-relative paths and path patterns.

This is broader than a narrow `[[ignore]]` table.

The product likely needs a way to:

- enable rules
- disable rules
- override defaults
- apply behavior to a path subset

That suggests a general rule-application table, perhaps something like `[[rules]]`, rather than a config model that can only express ignores.

A conceptual example:

    [[rules]]
    path = "docs/references/**"
    disable = ["unresolved-local-path"]

    [[rules]]
    path = ".agents/skills/**/SKILL.md"
    enable = ["skill-validation"]

This keeps exceptions and positive policy in one place instead of splitting them across unrelated tables.

Rule-specific options should also live in this layer instead of growing separate top-level tables for each feature.

For example, context-budget limits should be expressed as rule-specific fields in `[[rules]]`, not as a separate `[[limits]]` table with its own targeting model. Setting `max_tokens` or `max_lines` is enough to enable the corresponding budget check for that path pattern:

    [[rules]]
    path = ".agents/skills/**/SKILL.md"
    max_lines = 500
    max_tokens = 5000
    severity = "warn"

    [[rules]]
    path = "AGENTS.md"
    max_tokens = 1200
    severity = "error"

    [[rules]]
    path = "docs/references/**"
    disable = ["max_tokens", "max_lines"]
    reason = "Imported source-derived docs preserve source fidelity over compactness."

The direction is that rule behavior and rule options share the same targeting layer. Use explicit `path` targets rather than one overloaded `match` field, and keep public TOML keys in snake_case.

## Repository-Wide Defaults

Rule-application entries are not a complete replacement for repo-wide defaults.

Some policy choices are foundational enough that repositories should be able to state them once at the top level instead of expressing them indirectly through broad rule entries. A local path style default is the clearest example: repositories may want to say "this is a backticks repo" or "this is a links repo" as a global convention, then override that default only for selected paths.

The current design direction should therefore be layered:

- repository-wide defaults establish the main posture for the repo
- rule-application entries refine or override behavior for narrower scopes

More specific path patterns should win over the repo-wide default when they conflict.

For the path-style tradeoffs behind that example, see `docs/design-docs/path-style-policy.md`.

A conceptual example:

    path_style = "backticks"

    [[rules]]
    path = "docs/references/**"
    disable = ["prefer-backticks-for-local-paths", "prefer-links-for-local-paths"]
    reason = "Imported source-derived docs are not normalized for repo-authored style."

## `reason` For Exceptions

Exception-oriented configuration should likely support a `reason` field.

This is especially valuable when a repository disables or relaxes rules for a path pattern. A short human-readable reason makes the policy easier to review, easier for agents to preserve intentionally, and easier to revisit later when the repository evolves.

The strongest case for `reason` is disable or override behavior, not ordinary default rule application.

A plausible example:

    [[rules]]
    path = "docs/references/**"
    disable = ["unresolved-local-path"]
    reason = "Imported source-derived docs may contain hypothetical or external paths that should not be treated as repository errors."

The current design direction is:

- `reason` should exist for exception-oriented entries
- `reason` does not need to be required for ordinary positive rule application
- a future lint rule could warn when broad disable entries omit a reason

## Relationship To Discovery Commands

Discovery commands should depend on explicit repository conventions rather than reinventing their own targeting configuration.

That means:

- `docgarden skills list` and `docgarden skills match` should operate over the configured skills directory.
- Broader `docgarden list`, `tree`, and `match` commands may later need configured knowledge roots or named groups.
- Optional curated indexes should avoid assuming this repository's path names.

This keeps command behavior portable across repositories with different structures.

## Relationship To Front Matter Policy

Front matter validation should reuse explicit repository conventions and path-targeted rule configuration.

That means the product can say things like:

- files under the configured skills directory use the Agent Skills schema
- imported-reference paths require provenance fields
- some paths require no front matter by default

This is cleaner than attaching schema behavior to arbitrary path conventions scattered across features, but it does not require a generic grouping layer as the first implementation.

## Relationship To Imported References

Imported-reference behavior should avoid hard-coded directories.

In this repository, `docs/references/` is one example. Another repository might use a different path entirely.

The first useful configuration may be a narrow imported-reference path setting or path-targeted rules.

## Open Questions

- Should the skills directory key be named `skills_dir`, `skills_root`, or something else?
- What is the first feature that truly needs user-defined groups beyond explicit path patterns?
- Should rule-application entries continue to require `path` as the only public target field?
- Should `reason` be optional but recommended for disable or override entries, or required for any configuration that relaxes enforcement?
- Should broader discovery configuration use named groups, explicit roots, or path patterns?
