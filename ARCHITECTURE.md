---
description: "High-level code map, module boundaries, and architectural invariants for `docgarden`; read when changing CLI orchestration, discovery, metadata matching, lint pipelines, or other repository-scoped behavior."
---

# Architecture

`docgarden` is a Rust CLI for repository knowledge systems in agentic engineering repositories.

Today it has two shipped jobs:

- route agents to the right repository document with `docgarden match`
- enforce the mechanical documentation rules that make that routing reliable with `docgarden lint` and `docgarden fix`

The product stays intentionally local, deterministic, and repository-scoped. It works over repository Markdown, frontmatter, paths, and configured policy. It does not depend on network access, external indexes, or model inference.

## Bird's-Eye View

At a high level, the binary has one shared setup layer and two execution paths:

1. Parse CLI arguments and choose `match`, `lint`, or `fix`.
2. Infer the repository root from `docgarden.toml`, `.git`, or an explicit config path.
3. Load `docgarden.toml` and produce an effective repository policy.
4. Discover the Markdown files in scope, honoring include/exclude rules and gitignore behavior.
5. Run either:
   - metadata matching over each discovered document's frontmatter and path metadata
   - or linting/fixing over each discovered document's full Markdown source and AST

That shared setup is important: matching and linting should see the same repository root, config, and discovery set unless the command intentionally says otherwise.

## System Boundaries

The main architectural boundary is between repository-level orchestration and per-document analysis.

- `src/cli.rs`, `src/root.rs`, `src/config.rs`, and `src/discover.rs` decide what repository is being operated on and which Markdown files are in scope.
- `src/matching.rs`, `src/score.rs`, `src/analyzer.rs`, and `src/frontmatter.rs` implement metadata-based routing.
- `src/lint/` implements deterministic document validation and safe rewrites.

There is also a deliberate boundary between metadata discovery and body analysis:

- `match` ranks documents using frontmatter `name`, frontmatter `description`, and path-derived metadata.
- `lint` parses full Markdown and validates the document body plus frontmatter according to configured rules.

`docgarden` is not a full-text search engine. Body-text retrieval remains a job for tools like `rg`, while `match` stays focused on the metadata that should route an agent toward the right file.

## Code Map

### Entry points

`src/main.rs` is the binary entry point and delegates to `docgarden::run()`.

`src/lib.rs` wires together the repository-root, discovery, matching, analyzer, frontmatter, scoring, and lint modules.

### CLI orchestration

`src/cli.rs` owns the top-level command surface:

- `lint` for check-only validation
- `fix` for deterministic safe rewrites
- `match` for ranked metadata routing

It parses arguments with `clap`, infers color behavior, chooses the execution path, prints human-readable diagnostics, and decides the process exit behavior.

`src/root.rs` handles repository-root inference from explicit config paths and repository markers. This logic is shared so both linting and matching agree on which repository they are operating inside.

### Configuration and discovery

`src/config.rs` loads `docgarden.toml`, applies built-in defaults, and lowers repository policy into a form the rest of the tool can consume.

The config layer currently owns:

- repository-wide include and exclude patterns
- gitignore participation
- known path-like extensions and special filenames
- ordered `[[rules]]` entries for per-path rule behavior
- frontmatter requirements and field constraints
- file-level budgets such as `max_lines` and `max_tokens`

`src/defaults.rs` holds the stable built-in defaults for scan patterns and path classification.

`src/discover.rs` walks the repository and returns the Markdown files in scope. It is a shared traversal layer used by both `match` and `lint`, which keeps repository discovery behavior consistent across commands.

### Metadata matching

`src/frontmatter.rs` contains the shared frontmatter parser. This module is a key architectural seam because both matching and linting rely on the same parsing behavior; they should not drift on what counts as valid frontmatter.

`src/matching.rs` implements `docgarden match`. It:

- reads the discovered Markdown files
- extracts frontmatter `name` and `description`
- derives fallback metadata such as filename-based names and path prefixes
- builds the candidate set
- scores and sorts results
- renders either compact routing output or `--explain` diagnostics

`src/analyzer.rs` owns the shared lexical analysis used by `match`: separator and CamelCase splitting, apostrophe and possessive handling, stopword filtering, compound expansion, stemming, and display spans for highlighting.

`src/score.rs` owns the lexical ranking model used by `match`. The shipped scorer is combined-field BM25F over `name`, `path_prefix`, and `description`, using analyzer terms from `src/analyzer.rs`.

### Lint engine

`src/lint/mod.rs` owns per-file lint execution. It reads the source, parses Markdown into an AST, runs file-level checks, runs frontmatter checks, walks node-level rules, collects diagnostics, and applies edits in `fix` mode when the rewrite is explicitly safe.

`src/lint/rules/file.rs` contains file-level rules such as line and token budgets.

`src/lint/rules/frontmatter.rs` validates configured frontmatter requirements and field constraints against the shared parser output.

`src/lint/rules/local_paths.rs` evaluates repository-local path rules over inline code and Markdown link nodes.

`src/lint/references.rs` handles path classification, normalization, resolution, and fix rendering for repository-local references.

`src/lint/reporting.rs` converts rule findings into final diagnostics with positions, rule names, severities, and fixability.

### Shared support modules

`src/diagnostics.rs` defines the common diagnostic payloads and pattern-matching helpers used by configuration and reporting.

`src/paths.rs` provides repository-relative path helpers shared across discovery, matching, and linting.

## Architectural Invariants

- `docgarden` is repository-local. It reasons about files inside the inferred repository root and does not depend on remote services.
- Matching and linting share one discovery model. The same config and traversal rules should produce the same document corpus unless a command explicitly narrows or reshapes it.
- Frontmatter parsing is shared. `match` and `lint` should agree on valid, invalid, and absent frontmatter.
- `match` is metadata-first. It should rank from frontmatter and path-derived signals, not from arbitrary full-body search.
- `lint` is AST-based and deterministic. Rule decisions should come from repository contents plus configuration, not from heuristics that require language-model judgment.
- Fix mode is conservative. `docgarden` should only rewrite cases that can be transformed mechanically without guessing author intent.
- Diagnostics are file-local and automation-friendly. Every finding should anchor to one file and support stable human-readable output.

## Cross-Cutting Concerns

Repository relativity shows up everywhere. Root inference, discovery, path resolution, and diagnostic rendering all depend on a stable notion of the repository root.

Token efficiency also spans the product. `match` exists to help agents load the right docs instead of the whole repo, and lint budget rules exist to keep high-traffic documents small enough to stay useful in agent contexts.

Frontmatter quality is another cross-cutting concern. It affects both routing quality and lint enforcement, which is why the parser and policy layers are shared rather than duplicated.

The broader product constraint is that `docgarden` should complement agent reasoning, not replace it. The tool handles mechanical repository-knowledge checks and metadata routing; interpretation, summarization, and judgment stay with the agent.
