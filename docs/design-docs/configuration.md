---
description: "Working design draft for `docgarden` configuration, especially repository-wide conventions and path-targeted rule behavior; read when changing config shape, scope selection, rules entries, or other repository policy controls."
---

# Configuration

## Purpose

This document is a working design draft for how `docgarden` should configure repository-wide conventions and path-targeted rule behavior.

The main goal is to avoid baking one repository's directory layout into product behavior while keeping the configuration model as small as the current product needs.

## Why This Needs Its Own Design

Several `docgarden` feature areas depend on the same underlying question: which files should a feature operate on?

For near-term features, the concrete cases are skills and ExecPlans:

- `docgarden list --skills` and `docgarden match --skills <QUERY>` need to know where repository-local skills live.
- `docgarden list --plans`, `docgarden list --active-plans`, `docgarden list --completed-plans`, and `docgarden match --plans <QUERY>` need to know where ExecPlans live.
- Skills validation rules should apply automatically to that same directory.

Other future areas may need path-targeted configuration too, including front matter validation, context-budget defaults, imported-reference policy, and optional curated indexes. Those future needs are not enough by themselves to justify a broad generic grouping layer today.

## Core Direction

Prefer first-class configuration for first-class product concepts.

For skills and ExecPlans, that means top-level directory settings rather than a generic `[directories]` bucket or a broad document-family entry.

    skills-dir = ".agents/skills"
    plans-dir = "docs/exec-plans"

`skills-dir` is enough for skill-scoped discovery and default skills validation. Internally, `docgarden` derives skill-file paths from that directory, so users do not define the same directory twice.

`plans-dir` is enough for plan-scoped discovery. The active and completed plan directories are status partitions under that directory:

- active plans: `{plans-dir}/active/`
- completed plans: `{plans-dir}/completed/`

Those derived paths are not public v1 config keys. Keeping only `plans-dir` public avoids duplicate configuration and keeps active/completed plans tied to the ExecPlan concept.

## Scan Selection

Scan selection should stay separate from rule application.

Top-level `include` and `exclude` settings decide which Markdown files enter the lint run at all. They are repository-relative path patterns, not document-family declarations and not rule exceptions.

A conceptual example:

    include = ["docs/**", "README.md", "AGENTS.md", "*.md"]
    exclude = ["docs/references/**", "docs/generated/**"]

Use `exclude` for files that should not be linted by `docgarden`, such as generated output or source-derived material that is intentionally outside the repository-authored documentation contract.

Use `[[rules]].disable` instead when a file should still be discovered and linted, but a specific rule should not apply:

    [[rules]]
    path = "docs/references/**"
    disable = ["unresolved-link-path"]
    reason = "Imported source-derived docs may contain external or hypothetical paths."

This distinction matters because excluded files disappear from all lint checks, while rule disables preserve the file in discovery and relax only the named behavior.

## Why Not Generic Groups First

The near-term design should avoid both extremes:

- do not hard-code this repository's paths into product behavior
- do not introduce broad named groups before they earn their keep

Top-level `skills-dir` and `plans-dir` are first-class product conventions. They should not live under a generic `[directories]` table, because that would group settings by value type rather than product meaning.

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
    disable = ["unresolved-link-path"]

    [[rules]]
    path = ".agents/skills/**/SKILL.md"
    enable = ["skill-validation"]

This keeps exceptions and positive policy in one place instead of splitting them across unrelated tables.

Rule entries may also need their own `exclude` field. This is different from the top-level `exclude`: it narrows only that one rule entry after discovery has already selected the file. The file remains linted by other rules.

A conceptual example:

    [[rules]]
    path = "**/*.md"
    exclude = ["AGENTS.md"]
    max-tokens = 5000

That entry applies the token budget to Markdown files except `AGENTS.md`, while `AGENTS.md` can still be checked for local paths, front matter constraints, or any other matching rule entries.

Rule-specific options should also live in this layer instead of growing separate top-level tables for each feature.

For example, context-budget limits should be expressed as rule-specific fields in `[[rules]]`, not as a separate `[[limits]]` table with its own targeting model. Setting `max-tokens` or `max-lines` is enough to enable the corresponding budget check for that path pattern:

    [[rules]]
    path = ".agents/skills/**/SKILL.md"
    max-lines = 500
    max-tokens = 5000
    severity = "warn"

    [[rules]]
    path = "AGENTS.md"
    max-tokens = 1200
    severity = "error"

    [[rules]]
    path = "docs/**"
    exclude = ["docs/references/**"]
    max-lines = 300
    severity = "warn"

    [[rules]]
    path = "docs/references/**"
    disable = ["max-tokens", "max-lines"]
    reason = "Imported source-derived docs preserve source fidelity over compactness."

The direction is that rule behavior and rule options share the same targeting layer. Use explicit `path` targets rather than one overloaded `match` field, and keep public TOML keys in kebab-case.

For context-budget limits, `severity` is entry-level. If an entry includes both `max-tokens` and `max-lines`, the same severity applies to both diagnostics. Repositories that want different severities for the same path can use separate `[[rules]]` entries with the same `path`.

## `reason` For Exceptions

Exception-oriented configuration should likely support a `reason` field.

This is especially valuable when a repository disables or relaxes rules for a path pattern. A short human-readable reason makes the policy easier to review, easier for agents to preserve intentionally, and easier to revisit later when the repository evolves.

The strongest case for `reason` is disable or override behavior, not ordinary default rule application.

A plausible example:

    [[rules]]
    path = "docs/references/**"
    disable = ["unresolved-link-path"]
    reason = "Imported source-derived docs may contain hypothetical or external paths that should not be treated as repository errors."

The current design direction is:

- `reason` should exist for exception-oriented entries
- `reason` does not need to be required for ordinary positive rule application
- a future lint rule could warn when broad disable entries omit a reason

## Relationship To Discovery Commands

Discovery commands should depend on explicit repository conventions rather than reinventing their own targeting configuration.

That means:

- `docgarden list --skills` and `docgarden match --skills` should operate over `skills-dir`.
- `docgarden list --plans` and `docgarden match --plans` should operate over `plans-dir`.
- `docgarden list --active-plans` should operate over the `active/` directory under `plans-dir`.
- `docgarden list --completed-plans` should operate over the `completed/` directory under `plans-dir`.
- Optional curated indexes should avoid assuming this repository's path names.

This keeps command behavior portable across repositories with different structures.

## Relationship To Front Matter Policy

Front matter validation should reuse explicit repository conventions and path-targeted rule configuration.

That means the product can say things like:

- files under the configured skills directory use the Agent Skills schema
- files under the configured plans directory use the ExecPlan schema
- imported-reference paths require provenance fields
- some paths require no front matter by default

This is cleaner than attaching schema behavior to arbitrary path conventions scattered across features, but it does not require a generic grouping layer as the first implementation.

Field-level front matter requirements should use a nested rule-specific shape rather than many top-level `[[rules]]` fields.

A conceptual example:

    [[rules]]
    path = ".agents/skills/**/SKILL.md"

    [rules.frontmatter]
    schema = "agent-skill"
    required = ["description"]

    [rules.frontmatter.fields.description]
    max-chars = 1024

For a repository-wide description policy with an `AGENTS.md` exception, split "validate this field when present" from "require this field." That keeps the exception narrow:

    [[rules]]
    path = "**/*.md"

    [rules.frontmatter.fields.description]
    max-chars = 1024

    [[rules]]
    path = "**/*.md"
    exclude = ["AGENTS.md"]

    [rules.frontmatter]
    required = ["description"]

This means `AGENTS.md` may omit `description`, but if it has one, the same `max-chars` constraint still applies. Other Markdown files both require `description` and enforce the character limit.

## Relationship To Imported References

Imported-reference behavior should avoid hard-coded directories.

One repository might use `docs/references/`. Another repository might use a different path entirely.

The first useful configuration may be a narrow imported-reference path setting or path-targeted rules.
