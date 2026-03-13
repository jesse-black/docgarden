# Doc Gardening Linter

[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/dglint/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/dglint/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)


`dglint` is a blazing-fast, zero-dependency Rust CLI for enforcing structural repository-knowledge invariants in an agent-first world.

When autonomous coding agents treat your repository's Markdown files (`AGENTS.md`, `ARCHITECTURE.md`, `docs/`) as executable context, documentation stops being "nice to have" prose. Broken local references, monolithic instruction files, and stale context directly reduce agent reliability and waste expensive context windows. 

`dglint` validates repository-local references, enforces machine-readable styling, and emits deterministic diagnostics and safe autofixes. It ensures your repository remains a highly legible, progressively loadable operating system for your agents.

## The "Agent-Legible" Philosophy

**Give agents a map, not a 1,000-page instruction manual.**

In early agentic workflows, teams often try to stuff every rule, constraint, and context clue into a single, massive `AGENTS.md` file. This fails predictably: it crowds out the actual code in the context window, it rots instantly because it cannot be mechanically verified, and agents start optimizing for the wrong constraints.

To achieve high agent throughput, the repository itself must become the system of record. Knowledge is divided into a structured, cross-linked map of Markdown files, allowing agents to use progressive disclosure—starting with a small entry point and navigating to specific design docs or execution plans only when needed.



`dglint` is the mechanical enforcer of this map. It focuses exclusively on the structural invariants that agents and CI systems must be able to trust absolutely:

* Does this repository-local path actually exist?
* Is this reference written in the configured machine-readable style?
* Is the document structured correctly for a coding agent to parse?

## The Doc Gardener Workflow

`dglint` is designed to be the deterministic discovery engine for a larger **Doc Gardener** agent workflow. 

Because agents are bottlenecked by context windows and token costs, having an LLM read thousands of lines of documentation just to find broken links or style violations is highly inefficient. 

Instead, `dglint` runs instantly and freely in CI or locally to identify exactly what needs gardening. It emits machine-readable JSON that is fed directly to an autonomous Doc Gardener agent (acting as a repository skill). The agent then uses its context window exclusively for what it does best: applying semantic, natural-language fixes to the specific files `dglint` flagged.

## Installation

Because `dglint` is a statically linked Rust binary, it runs instantly in your CI pipeline without requiring any language runtimes.

**Via Cargo (Local Development):**
```bash
cargo install dglint
```

*(Alternatively, build from source: `cargo build --release`)*

## Usage

Run `dglint` locally or in your CI pipeline to continuously garden your repository knowledge.

### CLI Arguments

* `[TARGETS]...`: Repository root, directories, or explicit Markdown files to lint. Defaults to `.`
* `--config <FILE>`: Use an explicit `dglint.toml` configuration file.
* `--json`: Emit machine-readable diagnostics for agents or CI parsers.
* `--fix`: Apply deterministic safe rewrites to resolve violations.
* `--color <auto|always|never>`: Control colored human-readable output.

### Examples

**Lint the entire repository:**
```bash
dglint .
```

**Lint specific high-traffic entry points:**
```bash
dglint README.md AGENTS.md docs/exec-plans/
```

**Apply safe autofixes to the working tree:**
```bash
dglint . --fix
```

**Example CI Usage (GitHub Actions):**
```yaml
- name: Lint repository knowledge
  run: dglint .
```

## How does `dglint` compare to existing tools?

The category of agent-first repository tooling is still early. While there are a few adjacent tools in this space, `dglint` is optimized entirely around token efficiency and context window management.

**Standard Markdown Linters (e.g., `markdownlint`)**
Standard linters are designed for human typography and styling (e.g., heading levels, trailing spaces). They do not understand the repository filesystem and cannot validate whether a referenced path actually exists in the working tree. `dglint` guarantees that when an agent tries to read a referenced file, it will be there.

**Heuristic-Based Prompt Linters**
Tools are emerging to lint agent instruction files using basic heuristics. These tools generally focus on the syntax of the prompt itself. `dglint`, by contrast, secures the integrity of the *entire repository knowledge graph*. 

**The `dglint` Advantage**
By enforcing strict cross-linking hygiene and local reference accuracy, `dglint` guarantees that your repository knowledge base is highly optimized for progressive disclosure. Agents can confidently navigate your repository via references instead of forcing you to load massive, monolithic instruction files into every single prompt, saving significant token costs and improving agent focus.

## Architecture & Product Vision

For a deeper dive into the internal design and the broader vision of CI-enforced doc-gardening workflows, see:

* [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - The current code map and architectural invariants.
* [`docs/PRODUCT.md`](docs/PRODUCT.md) - The product framing, core workflows, and non-goals.

## Contributing

Contributions are welcome! If you want to add support for a new repository-knowledge check, please ensure it aligns with our philosophy: rules must be deterministic, repository-local, and enforceable without model inference.

## License

Apache 2.0. See [LICENSE](LICENSE) for details.