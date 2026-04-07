# Product

## Purpose

`docgarden` is a documentation-maintenance CLI for agentic engineering repositories.

It is designed for repositories that treat in-repo documentation as the system of record within a repository agent operating system. In those environments, agents need documentation that is mechanically legible, cross-linked, fresh, and cheap to navigate in tokens and context.

Its job is to enforce repository-knowledge invariants that help agents load the right context progressively instead of pulling large, stale, or weakly structured docs into context by default. In practice, that means keeping references like `AGENTS.md`, `ARCHITECTURE.md`, and `docs/` accurate, structured, and suitable for CI enforcement and recurring doc-gardening automation.

## Primary Users

The primary users are:

- teams practicing agentic engineering
- heavily agent-led repositories where humans steer and agents execute
- agent-only or near-agent-only repositories that rely on repository-local documentation as executable context
- maintainers building a repository knowledge system for Codex-style agents and related tooling

`docgarden` is not primarily aimed at teams that merely have a lot of Markdown. Its value depends on agentic workflows where documentation quality directly affects agent reliability, token efficiency, and autonomy.

## Core Workflows

The product is built around a small set of workflows:

- lint a repository knowledge system in CI so agent-facing docs stay mechanically valid
- power a recurring Doc Gardener agent that loads doc-gardening skills, runs `docgarden`, and opens maintenance fixes
- fail CI when repository-local references are unresolved, stale, oversized, weakly structured, or insufficiently cross-linked
- enforce size limits on high-traffic auto-loaded files such as `AGENTS.md` so agents spend fewer tokens on entry-point context
- enforce YAML front matter used for agent navigation and repository operations, including fields such as `description`, `owner`, and `last_reviewed`
- enforce cross-linking so repository knowledge remains discoverable through file references instead of hidden in isolated documents
- enforce a consistent style policy for repository-local references: inline backticks or Markdown links; in agent-first repositories, backticks can be a strong default because agents often emit them naturally and they avoid the extra token cost of path-repeating Markdown labels
- apply safe autofixes for rules that can be rewritten mechanically
- tune scanning and rule behavior with `docgarden.toml`
- suppress specific rules for specific files when a repository intentionally makes an exception

## Current Capabilities

Capabilities that are clearly part of the current product:

- repository root inference from `docgarden.toml`, `.git`, or the current working directory
- Markdown file discovery from default or configured include and exclude patterns
- recognition of repository-local references in inline code and Markdown links
- existence checks for resolved local paths within the repository
- diagnostics for unresolved local paths
- diagnostics and autofixes for style-policy mismatches between backticks and links
- optional warnings for ambiguous inline code that looks path-adjacent
- explicit line-count and token-budget diagnostics configured with `max_lines` and `max_tokens`
- human-readable output and JSON output for automation

Durable product inputs that exist today:

- the repository filesystem
- Markdown files selected for linting
- `docgarden.toml` configuration when present

## Planned Or Intended Capabilities

These capabilities fit the product direction but should not be described as already complete:

- generated or built-in default budget policy for high-traffic docs such as `AGENTS.md`
- YAML front matter enforcement for `description`, `owner`, `last_reviewed`, and similar fields that improve agent routing and repository operations
- freshness checks for repository knowledge documents
- cross-linking enforcement across the repository knowledge graph
- rule messages and output shaped for a future Doc Gardener agent workflow in CI

## Non-Goals

The current product is not trying to be:

- a general-purpose Markdown style linter
- an external link checker
- a semantic documentation reviewer driven by natural-language understanding
- a documentation site generator
- a repository knowledge system on its own
- a tool that requires any LLM or natural-language runtime to make lint decisions

`docgarden` is intentionally mechanical. Any task that requires summarization, judgment, interpretation, or broader natural-language reasoning should be performed by the agent using this tool, not by the tool itself.

## Product Boundaries That Affect Architecture

Some product choices directly constrain the implementation:

- the tool should optimize for agent legibility rather than human-only reading convenience
- lint decisions should be deterministic from repository contents plus config, without remote dependencies or model inference
- the product should be reliable in CI because CI is the primary enforcement point
- the tool should support recurring autonomous maintenance runs, especially a future Doc Gardener agent workflow
- the product should help repositories conserve agent tokens and context on high-traffic entry-point docs
- fixes should remain conservative and mechanically safe
- the product should stay useful for plain CLI invocation as well as automation pipelines
- configuration should remain repository-owned rather than requiring external services

For the current technical structure and module boundaries, see `ARCHITECTURE.md`.
