# docgarden

[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/docgarden/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/docgarden/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)

Repository knowledge tooling for agentic engineering repositories — routes agents to the right context with minimum tokens, and enforces the doc hygiene that makes routing reliable.

## Why

Agent-first repositories treat their Markdown — design docs, product specs, skills, agent instructions — as the system of record. The hard part isn't storing knowledge — it's loading the *right* knowledge without blowing up context. `docgarden` approaches this as a routing problem: every document carries a `description` frontmatter field, and `docgarden match` does a BM25F search over that metadata to return ranked paths. No central routing table to maintain; the docs route themselves.

`docgarden lint` enforces the invariants that keep routing accurate: `description` frontmatter on every doc, and size budgets on high-traffic entry-points like `AGENTS.md` so agents spend fewer tokens just getting oriented. To paraphrase [Ryan Lopopolo](https://openai.com/index/harness-engineering/): "give agents a map, not a 1,000-page instruction manual."

## Philosophy

`docgarden` is designed to work *with* agents: it handles everything that can be decided mechanically and deterministically from repository contents, so agents can spend their tokens and context on work that actually requires judgment. Anything that needs summarization, interpretation, or natural-language reasoning belongs to the agent, not the tool.

## Usage

### `match` — route an agent to the right document

```
docgarden match <QUERY>
```

Ranks repository Markdown documents by how well their frontmatter fields match the query. Returns `path | name | description` by default.

```
# find the most relevant doc for a task
docgarden match "auth middleware session tokens"

# path-only for piping into agent context
docgarden match -p -n 3 "deployment rollback"

# show scoring diagnostics
docgarden match --explain "rate limiting"
```

Options: `-n <LIMIT>` to cap results, `-p` / `--path-only` for plain paths, `--explain` for BM25F score breakdown.

### `lint` — enforce repository knowledge hygiene

```
docgarden lint [TARGETS]
```

Lints without modifying files. Targets can be the repository root, specific directories, or individual Markdown files (defaults to `.`).

```
# lint the whole repo
docgarden lint

# lint a specific subtree
docgarden lint docs/
```

Checks include: unresolved local references, missing `description` frontmatter, `AGENTS.md` line/token budget violations, and path style policy (backtick vs Markdown link).

## Configuration

`docgarden.toml` at the repository root controls include/exclude patterns, rule behavior, and size budgets. See [docs/design-docs/configuration.md](docs/design-docs/configuration.md) for details.

## Contributing

Contributions are welcome! If you want to add support for a new repository-knowledge check, please ensure it aligns with our philosophy: rules must be deterministic, repository-local, and enforceable without model inference.

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
