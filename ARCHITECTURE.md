# Architecture

This document describes the high-level architecture of `dglint`.

`dglint` is a Rust CLI for enforcing mechanical repository-knowledge invariants in agentic engineering repositories.

Today, the implementation is centered on Markdown-local path integrity and style. It scans repository Markdown files, parses them into an AST, classifies repository-local references found in inline code and links, resolves those references against the repository root, and reports or fixes violations according to the configured style policy.

This makes `dglint` part of the repository agent operating system rather than a general-purpose Markdown linter. The tool is intended to support progressive context loading and CI-enforced doc-gardening workflows in repositories that treat in-repo documentation as the system of record. As the rule set expands, new checks should remain deterministic, repository-local, and mechanically enforceable without model inference.

## Bird's-Eye View

At the highest level, `dglint` has a simple pipeline:

1. Parse CLI arguments and determine the effective repository root and execution mode.
2. Load configuration from `dglint.toml` or built-in defaults.
3. Discover the Markdown files that should be linted for this invocation.
4. Parse each file into a Markdown AST.
5. Walk the AST and classify inline code and links that might represent repository-local paths.
6. Resolve candidate paths against the repository root, emit diagnostics, and optionally apply safe rewrites.
7. Render diagnostics to text or JSON and exit non-zero when violations remain.

The tool is intentionally local and repository-scoped. It does not fetch remote state, require LLM support, or depend on Git metadata beyond optionally using `.git` to infer the repository root. Future repository-knowledge checks may broaden what is validated, but they should preserve this same operating model.

## Code Map

### Entry points

`src/main.rs` is the binary entry point. It delegates directly to `dglint::run()`.

`src/lib.rs` is a small crate root that exposes the CLI runner and keeps the executable thin.

### CLI orchestration

`src/cli.rs` is the top-level coordinator for a run. It is responsible for:

- parsing command-line arguments with `clap`
- selecting check mode versus fix mode
- canonicalizing explicit targets
- inferring the repository root from `dglint.toml`, `.git`, or the current working directory
- loading configuration
- choosing between full discovery and explicit target handling
- running the linter over each selected file
- printing diagnostics and fix hints
- determining the process exit condition

If you want to understand the end-to-end control flow of the binary, start here.

### Configuration and defaults

`src/config.rs` loads `dglint.toml`, merges it with built-in defaults, and produces the effective `Config`.

The config model currently controls:

- include and exclude scan patterns
- known file extensions and special filenames used for path classification
- per-file ignored rules
- the local reference style policy: `backticks` or `links`
- whether path-adjacent inline code should produce warnings

`src/defaults.rs` contains the stable built-in defaults for scan patterns, known extensions, and special filenames. Keeping these defaults separate makes the policy surface easy to review without reading the rest of the linter.

### File discovery and diagnostics

`src/discover.rs` handles file selection. It walks the filesystem under the selected roots, applies include and exclude patterns relative to the repository root, and returns a sorted list of files to lint.

`src/diagnostics.rs` defines the diagnostic payload that the rest of the tool emits, plus the glob-style matcher used for include, exclude, and per-file ignore behavior.

### Lint engine

`src/lint/mod.rs` owns the AST walk and the high-level lint rules. It reads the Markdown source, parses it with `markdown`, traverses the tree, emits diagnostics, and writes the file back when fix mode made an allowed rewrite.

Today the lint engine focuses on repository-local reference correctness and style:

- `unresolved-local-path`
- `prefer-links-for-local-paths`
- `prefer-backticks-for-local-paths`
- `ambiguous-inline-code`

`src/lint/references.rs` contains the path-oriented logic that decides whether a string looks like a repository-local reference, resolves it relative to the current file or repository root, normalizes paths, and renders replacement text for fixes.

`src/lint/reporting.rs` is a small adapter that turns rule findings plus source positions into final diagnostics while respecting per-file ignored rules.

As the product grows into broader repository-knowledge checks such as front matter, cross-linking, freshness, or size limits, that work should fit into this layer as additional deterministic rule families rather than introducing a separate interpretation engine.

## Architectural Invariants

These invariants are important to preserve even if the internal implementation changes.

- `dglint` is repository-local. It reasons about paths that should resolve within the current repository root and does not depend on network access.
- `dglint` is mechanically enforced for agent workflows. It should not require natural-language understanding or model-backed judgments to decide whether a rule passes.
- `dglint` assumes repository knowledge should be encoded in versioned repo artifacts. Rules should reinforce discoverable, cross-linked documentation rather than rely on external context.
- Markdown AST traversal is the source of truth for linting. The tool does not rely on regex-only scanning for rule decisions.
- Resolution is path-based, not symbol-based. `dglint` verifies whether a referenced repository path exists; it does not infer semantic meaning from the target file contents.
- Fix mode is conservative. The tool only rewrites cases where the local-reference style policy can be applied mechanically without inventing new target semantics.
- Configuration affects classification and scope, not repository contents. The linter never mutates anything outside files explicitly being fixed.
- Diagnostics are file-local. Each reported issue is anchored to a single Markdown file position and can be emitted in human-readable or JSON form.
- High-traffic repository entry points such as `AGENTS.md` are part of the intended design target. New rules should support progressive disclosure and agent legibility rather than push repositories back toward large monolithic instruction files.

## Boundaries

The most important boundary in the codebase is between orchestration and lint logic:

- `src/cli.rs` decides what files to lint and how results are presented.
- `src/lint/` decides whether a specific Markdown node is valid, invalid, or fixable.

There is also a useful policy boundary between configuration and rule execution:

- `src/config.rs` defines the effective policy inputs.
- `src/lint/` consumes those inputs but does not own config-file parsing.

This split keeps the core linter logic testable without coupling it to argument parsing or filesystem discovery details.

## Cross-Cutting Concerns

Path normalization and repository relativity appear across several modules. The tool consistently treats the inferred repository root as the frame of reference for discovery, ignore matching, and path existence checks.

Human-oriented output and machine-oriented output share the same diagnostic model. Text and JSON output differ only at the presentation layer, which helps keep rule behavior consistent across local development and automation use cases.

The broader product direction is CI-first and agent-first. That means error messages, rule naming, and fix behavior should stay legible both to humans reviewing failures and to future Doc Gardener-style agents consuming the results mechanically.
