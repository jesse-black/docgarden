# docgarden

[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/docgarden/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/docgarden/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)


`docgarden` is a blazing-fast, zero-dependency Rust CLI for enforcing structural repository-knowledge invariants in an agent-first world.

When autonomous coding agents treat your repository's Markdown files (`AGENTS.md`, `ARCHITECTURE.md`, `docs/`) as executable context, documentation stops being "nice to have" prose. Broken local references, monolithic instruction files, and stale context directly reduce agent reliability and waste expensive context windows. 

`docgarden` validates repository-local references, enforces machine-readable styling, and emits deterministic diagnostics. It ensures your repository remains a highly legible, progressively loadable operating system for your agents.

## The "Agent-Legible" Philosophy

**Give agents a map, not a 1,000-page instruction manual.**

In early agentic workflows, teams often try to stuff every rule, constraint, and context clue into a single, massive `AGENTS.md` file. This fails predictably: it crowds out the actual code in the context window, it rots instantly because it cannot be mechanically verified, and agents start optimizing for the wrong constraints.

To achieve high agent throughput, the repository itself must become the system of record. Knowledge is divided into a structured, cross-linked map of Markdown files, allowing agents to use progressive disclosure—starting with a small entry point and navigating to specific design docs or execution plans only when needed.



`docgarden` is the mechanical enforcer of this map. It focuses exclusively on the structural invariants that agents and CI systems must be able to trust absolutely:

* Does this repository-local path actually exist?
* Does this document follow the configured mechanical style rules?
* Is the document structured correctly for a coding agent to parse?

## The Doc Gardener Workflow

`docgarden` is designed to be the deterministic discovery engine for a larger **Doc Gardener** agent workflow. 

Because agents are bottlenecked by context windows and token costs, having an LLM read thousands of lines of documentation just to find broken links or style violations is highly inefficient. 

Instead, `docgarden` runs instantly and freely in CI or locally to identify exactly what needs gardening. It emits machine-readable JSON that is fed directly to an autonomous Doc Gardener agent (acting as a repository skill). The agent then uses its context window exclusively for what it does best: applying semantic, natural-language fixes to the specific files `docgarden` flagged.

## Installation

Because `docgarden` is a statically linked Rust binary, it runs instantly in your CI pipeline without requiring any language runtimes.

**Via Cargo (Local Development):**
```bash
cargo install docgarden
```

*(Alternatively, build from source: `cargo build --release`)*

## Usage

Run `docgarden` locally or in your CI pipeline to continuously garden your repository knowledge.

### Commands

* `docgarden lint [TARGETS]...`: Lint repository knowledge without modifying files.

Shared lint flags:

* `--config <FILE>`: Use an explicit `docgarden.toml` configuration file.
* `--json`: Emit machine-readable diagnostics for agents or CI parsers.
* `--no-gitignore`: Ignore `.gitignore`, `.ignore`, and related git exclude files during discovery.
* `--color <auto|always|never>`: Control colored human-readable output.

### Configuration

`docgarden.toml` can apply existing lint rules to repository-relative paths or gitignore-style path patterns:

```toml
[[rules]]
path = "docs/references/**"
disable = ["unresolved-link-path"]
reason = "Imported references may preserve source-derived paths."

[[rules]]
path = "docs/**"
enable = ["prefer-links-for-local-paths"]

[[rules]]
path = "internal/**"
enable = ["unresolved-backtick-path"]
severity = "warn"
```

Supported rule names today are `unresolved-link-path`, `unresolved-backtick-path`, `prefer-links-for-local-paths`, `max_tokens`, and `max_lines`.

### Examples

**Lint the entire repository:**
```bash
docgarden lint .
```

**Lint specific high-traffic entry points:**
```bash
docgarden lint README.md AGENTS.md docs/exec-plans/
```

**Lint files that are normally skipped by `.gitignore`:**
```bash
docgarden lint . --no-gitignore
```

**Example CI Usage (GitHub Actions):**
```yaml
- name: Lint repository knowledge
  run: docgarden lint .
```


## Contributing

Contributions are welcome! If you want to add support for a new repository-knowledge check, please ensure it aligns with our philosophy: rules must be deterministic, repository-local, and enforceable without model inference.

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
