# docgarden

[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/docgarden/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/docgarden/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/docgarden.svg)](https://crates.io/crates/docgarden)
[![docs.rs](https://img.shields.io/docsrs/docgarden)](https://docs.rs/docgarden)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)

Repository knowledge tooling for agentic engineering repositories: route agents to the right context with minimum tokens, and enforce the doc hygiene that makes routing reliable.

## Why

Agent-first repositories keep their working knowledge in Markdown: design docs, product specs, skills, and agent instructions. `docgarden` applies progressive disclosure to that knowledge base, so agents load the right context instead of the whole repo. Every document carries a `description` frontmatter field, and `docgarden match` uses BM25F to rank the right paths. Instead of maintaining a central routing table, the docs route themselves.

`docgarden lint` checks the rules that keep routing accurate: `description` frontmatter on every doc, plus size budgets on high-traffic entry points like `AGENTS.md` so agents do not burn tokens just getting oriented. To paraphrase [Ryan Lopopolo](https://openai.com/index/harness-engineering/): give agents a map, not a 1,000-page instruction manual.

## Philosophy

`docgarden` is meant to work with agents as a context engineering tool for repository knowledge. It handles the parts that can be done deterministically, so agents can spend their context on work that actually requires judgment. If something depends on summarization, interpretation, or natural-language reasoning, it belongs to the agent, not the tool.

## Using

### `match`: route an agent to the right document

```
docgarden match <QUERY>
```

Ranks repository Markdown documents by how well their frontmatter fields match the query. Returns `path | name | description` by default.

```
# route a review task to the active-plan workflow
docgarden match review against the active plan

# path-only for piping the top few candidates into agent context
docgarden match -p -n 3 implement from the active plan

# show scoring diagnostics
docgarden match --explain docgarden match scoring
```

Options: `-n <LIMIT>` to cap results, `-p` / `--path-only` for plain paths, `--explain` for BM25F score breakdown.

Example `AGENTS.md` instruction for non-code context discovery:

```md
## Step 0 (required before keyword-searching for documentation)
Run `docgarden match <query>` before using `rg`, `grep`, `find`, or agents to locate Markdown documentation, plans, repository guidance, or repository-local skills.
Use this when your first instinct is to search docs or guidance by keyword.
Do not repeat this step when the relevant file is already named by the user, listed in this file, or still in active context.
Do not use this step for code-first work, code symbol searches, test names, compiler errors, or known file paths; inspect and search code directly.
```

### `lint`: enforce repository knowledge hygiene

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

`docgarden` looks for `docgarden.toml` at the repository root. If you do not create one, it uses this built-in default configuration:

```toml
[[rules]]
path = "*.md"

[rules.frontmatter.fields.description]
max_chars = 1024

[[rules]]
path = "*.md"
exclude = ["AGENTS.md", "CLAUDE.md", "GEMINI.md", "README.md"]

[rules.frontmatter]
required = ["description"]

[[rules]]
path = "AGENTS.md"
max_lines = 100
max_tokens = 1000

[[rules]]
path = "SKILL.md"
max_lines = 500
max_tokens = 5000

[rules.frontmatter]
required = ["name", "description"]
```

That is a good starting point to copy into `docgarden.toml` and tweak for your repository.

Top-level `exclude` removes files from the lint run entirely. For example:

```toml
exclude = ["docs/generated/**"]
```

## Contributing

Contributions are welcome. If you want to add a new repository-knowledge check, keep it aligned with the project's philosophy: rules should be deterministic, repository-local, and enforceable without model inference.

## License

Apache 2.0. See [LICENSE](LICENSE) for details.
