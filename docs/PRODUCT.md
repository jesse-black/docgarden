---
description: "Product overview for `docgarden`, covering users, current workflows, shipped scope, boundaries, and near-term product direction; read when making roadmap, scope, naming, or positioning decisions."
---

# Product

## Purpose

`docgarden` is repository knowledge tooling for agentic engineering repositories.

Its job is to help repositories keep Markdown knowledge both discoverable and mechanically trustworthy for agents. Today that means two complementary capabilities:

- `docgarden match` routes an agent toward the right document using frontmatter and path metadata
- `docgarden lint` and `docgarden fix` enforce the documentation rules that keep that routing reliable

The product is intentionally narrower than a general documentation platform. It focuses on repository-local knowledge, deterministic checks, and metadata that supports progressive disclosure.

## Primary Users

The primary users are:

- teams practicing agentic engineering
- repositories where agents frequently need to discover and load the right Markdown context
- maintainers who want repository knowledge quality enforced in CI
- tool builders shaping repository-local docs into a dependable operating surface for agents

`docgarden` is not aimed at every repository with Markdown. Its value is highest when documentation quality directly affects agent routing, context cost, and execution reliability.

## Current Product Surface

The currently shipped command surface is:

- `docgarden match <QUERY>`
- `docgarden lint [TARGETS]`
- `docgarden fix [TARGETS]`

`match` is a metadata-routing command. It ranks repository Markdown documents by metadata relevance and, by default, prints:

- `path`
- `name`
- `description`

The current ranking model uses frontmatter `name` when present, falls back to filename stem when needed, uses frontmatter `description`, and incorporates path-prefix signal for routing. It is intentionally metadata-first rather than full-text body search.

`lint` is the check-only enforcement command. `fix` applies the safe rewrite subset for issues that can be rewritten mechanically.

## Core Workflows

The main workflows today are:

- route an agent to the best candidate docs for a task before opening file bodies
- lint repository-authored Markdown in local development or CI
- keep local repository path references valid and consistently styled
- require `description` frontmatter across repository docs where the repository has opted into that policy
- enforce frontmatter constraints such as required fields and maximum field length for targeted document types
- enforce explicit file-level size budgets on high-traffic docs such as `AGENTS.md` or `SKILL.md`
- apply deterministic safe rewrites for the subset of style issues that do not require intent inference
- tune discovery and rule behavior with repository-owned `docgarden.toml`

## Shipped Capabilities

Capabilities that are clearly part of the product today:

- repository-root inference from `docgarden.toml`, `.git`, or an explicit config path
- shared Markdown discovery for both matching and linting, with include/exclude patterns and gitignore-aware traversal
- shared frontmatter parsing used by both routing and linting
- metadata ranking over `name`, `path_prefix`, and `description`
- compact match output plus `--path-only`, `--limit`, and `--explain` modes
- deterministic validation of repository-local path references in Markdown prose
- configurable style-policy enforcement for repository-local references
- frontmatter validation for required fields and field-length constraints
- explicit `max_lines` and `max_tokens` budget rules
- human-readable CLI output for routing and diagnostics

## Repository Conventions The Product Assumes

`docgarden` works best when a repository treats Markdown as a maintained knowledge system rather than a loose pile of notes.

The product assumes repositories will:

- keep important operational knowledge in versioned repo documents
- attach short, useful `description` frontmatter to documents that should be discoverable through `match`
- encode exceptions in configuration rather than rely on ad hoc tool behavior
- distinguish between repository-authored docs and raw imported references

This is why `docgarden` is best understood as repository knowledge tooling, not just a Markdown linter.

## Near-Term Direction

Design docs in `docs/design-docs/` describe likely next capabilities, but they should not be mistaken for shipped scope.

Near-term directions that fit the current product shape include:

- additional discovery commands such as `list`
- scope-specific commands such as `skills`
- broader frontmatter schemas beyond the current required-field and max-length checks
- more repository-knowledge rules that stay deterministic and repository-local
- richer informational commands such as repository stats

Those ideas belong to the product direction, not the current release contract.

## Non-Goals

The current product is not trying to be:

- a general-purpose Markdown style linter
- a full-text search engine for Markdown bodies
- a semantic documentation reviewer driven by natural-language understanding
- a documentation site generator
- a hosted repository knowledge platform
- a tool that needs network access or model inference to decide whether a rule passes

If a task requires interpretation, summarization, or broader judgment, that remains the job of the agent using `docgarden`, not `docgarden` itself.

## Product Boundaries That Affect Architecture

Some product choices directly constrain the implementation:

- discovery and enforcement should both stay repository-local
- the same repository config should shape both routing and linting behavior
- routing should reward well-maintained metadata rather than encourage body-text search as the primary path
- rule behavior should remain deterministic and CI-friendly
- fixes should stay conservative and mechanically safe
- repositories should own policy through `docgarden.toml`, not through remote services or hidden defaults

For the current module layout and code boundaries, see `ARCHITECTURE.md`. For command-specific design direction beyond the shipped product, see the relevant files under `docs/design-docs/`.
