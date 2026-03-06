# Doc Gardener Linter

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. If that file exists in the target repository, re-read it before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, the repository will have a dedicated linter named `Doc Gardener Linter`, exposed as the executable `dglint`, that checks whether Markdown documentation refers to local repository files and directories consistently using backticks instead of Markdown links where possible, and whether those references actually resolve to real paths in the working tree. The user-visible benefit is that agent-oriented docs can conserve context tokens by preferring compact backticked paths such as `docs/PLANS.md` instead of heavier link syntax, while still getting mechanical validation that documentation stays current, cross-linked, and navigable.

You will know this is working when running `dglint lint .` against a repository reports real broken path references, reports ambiguous or non-conforming file references, and exits successfully on a clean repository. The linter must be deterministic enough to run in local development and continuous integration, and durable enough to publish as a reusable open source command-line tool.

## Progress

- [ ] (2026-03-06 00:00Z) Scaffold the Rust crate for `Doc Gardener Linter` and expose a working `dglint` command-line executable without changing repository docs yet.
- [ ] (2026-03-06 00:00Z) Define the reference taxonomy: what counts as a file-path reference, what counts as a code-symbol reference, and what counts as an external navigation link.
- [ ] (2026-03-06 00:00Z) Implement Markdown parsing with `markdown-rs`, including extraction of inline code spans, fenced code blocks, Markdown links, and source positions from mdast nodes.
- [ ] (2026-03-06 00:00Z) Implement path classification rules for backticked text and repository-relative path resolution.
- [ ] (2026-03-06 00:00Z) Implement lint rules for local Markdown links that should be replaced with backticked paths, unresolved backticked paths, and ambiguous backticks.
- [ ] (2026-03-06 00:00Z) Add autofix support for the safe subset of violations, serializing edits back to Markdown using the Rust mdast serialization stack.
- [ ] (2026-03-06 00:00Z) Add fixture-based tests covering valid references, invalid references, edge cases, and false-positive protection.
- [ ] (2026-03-06 00:00Z) Wire `dglint` into repository scripts and continuous integration.
- [ ] (2026-03-06 00:00Z) Update repository documentation to explain the backtick convention and the limited cases where Markdown links remain allowed.
- [ ] (2026-03-06 00:00Z) Capture final evidence, update this plan, and move it to `docs/exec-plans/completed/` when the linter is adopted.

## Surprises & Discoveries

- Observation: None yet.
  Evidence: Implementation has not started.

## Decision Log

- Decision: Use backticks as the default representation for local repository path references in agent-oriented Markdown docs, with Markdown links reserved for external destinations and rare intentional navigation cases.
  Rationale: Backticks are shorter, easier for agents to copy and pattern-match, and align with token-conserving documentation goals. The linter exists to make this convention machine-verifiable.
  Date/Author: 2026-03-06 / Codex

- Decision: Name the tool `Doc Gardener Linter` and expose the executable as `dglint`.
  Rationale: The human-facing title aligns with the broader doc-gardening concept, while the shorter executable name is efficient for repeated local and CI use. This also leaves room for a future higher-level doc-gardening agent or workflow that may use `dglint` as one component.
  Date/Author: 2026-03-06 / Codex

- Decision: Implement the first version in Rust using `markdown-rs` for Markdown-to-mdast parsing.
  Rationale: The project targets durable open source adoption, and Rust provides strong single-binary distribution, performance, and CLI ergonomics. `markdown-rs` provides mdast parsing aligned with the broader remark ecosystem, which removes a major historical drawback of Rust for Markdown tooling.
  Date/Author: 2026-03-06 / Codex

- Decision: Do not treat every inline code span as a path reference.
  Rationale: Inline code spans are also used for commands, symbols, environment variables, and code fragments. The linter must classify only path-shaped strings as path references to avoid excessive false positives.
  Date/Author: 2026-03-06 / Codex

- Decision: Require repository-local path references to be repository-relative and path-shaped.
  Rationale: A stable grammar reduces ambiguity and allows mechanical validation. Examples include `docs/PLANS.md`, `web/src/app.tsx`, `api/Program.cs`, and `terraform/modules/network`.
  Date/Author: 2026-03-06 / Codex

## Outcomes & Retrospective

No implementation work has been completed yet. The intended outcome is a linter that turns the repository’s documentation conventions into enforceable rules, reducing both stale docs and unnecessary token cost in agent contexts. The main risk is overfitting the classifier or writing rules that are too aggressive and create noisy false positives. A secondary risk is overscoping the first Rust implementation before the rule semantics are proven on real repositories.

## Context and Orientation

This plan assumes a repository that uses Markdown documentation heavily and wants a structured knowledge base where `AGENTS.md` acts as a small map and deeper guidance lives elsewhere under `docs/`. The problem to solve is not generic prose linting. The problem is documentation drift in path references: a Markdown file says `docs/PLANS.md` exists, or links to `docs/architecture.md`, but the path is stale, inconsistent, or unnecessarily represented using long Markdown link syntax when a simple backticked path would be better for agents.

In this plan, a "repository-local path reference" means a textual mention of a file or directory that is expected to exist in the same repository as the linter is running against. The preferred representation is a backticked repository-relative path, for example `docs/PLANS.md`. A "Markdown link" means standard Markdown link syntax such as `[Plans](docs/PLANS.md)`. An "external link" means a destination outside the repository, such as an `https://` URL; these remain valid and should not be rewritten to backticks. An "ambiguous inline code span" means backticked text that could be a path or could be a code fragment, for example `build`, `Program`, or `foo.bar`, where the linter cannot confidently infer that the author meant a repository path.

The linter should operate on Markdown files, probably under `docs/`, `README.md`, `AGENTS.md`, and other configured documentation roots. It must parse Markdown structure rather than relying only on regular expressions because fenced code blocks, inline code spans, link destinations, autolinks, and escaped punctuation behave differently. It should ignore fenced code blocks by default because those blocks often contain examples that are not intended to be live repository references. It should inspect prose, headings, list items, tables if present, and inline code spans.

The implementation should be repository-agnostic. It belongs in its own repository or crate so other repositories can adopt it. That means configuration must be explicit. The linter should accept a repository root to scan, a set of Markdown globs to include or exclude, and a small rules configuration describing allowed path extensions, documentation roots, and exceptions.

The Rust implementation should build on `markdown-rs` to parse source Markdown into mdast nodes with source positions. For autofix support, the implementation should use the Rust mdast serialization stack to write edited mdast trees back to Markdown text. This keeps parsing and serialization aligned around mdast rather than mixing unrelated text-manipulation approaches.

## Plan of Work

Start by creating a standalone Rust crate for `Doc Gardener Linter` that builds a CLI executable named `dglint`. Use Cargo as the package manager and test runner. The crate should expose a small command-line interface with at least these modes: scan and report violations, scan with machine-readable output, and scan with autofix for safe rewrites. The command-line interface should accept a repository root, optional file globs, and a configuration file path. It should exit with code `0` when no violations are found, `1` when violations are found, and a distinct non-zero code when configuration or parser errors prevent a complete run.

Implement parsing on top of a real Markdown abstract syntax tree rather than trying to lint with line-oriented regular expressions. Use `markdown-rs` to parse Markdown into mdast, and preserve source positions for inline code spans, link nodes, text nodes, and other relevant nodes. Source positions matter because the linter needs to print file, line, and column for each violation and because safe autofix requires precise replacements or stable AST-to-Markdown regeneration.

Define the reference taxonomy in code before implementing rules. A repository-local path reference must satisfy all of the following conditions unless a configuration override says otherwise. It must be repository-relative, not absolute. It must be path-shaped. "Path-shaped" means at least one of these is true: it contains a `/`; it starts with `./` or `../`; it ends with a known file extension such as `.md`, `.ts`, `.tsx`, `.js`, `.jsx`, `.json`, `.yml`, `.yaml`, `.cs`, `.csproj`, `.tf`, `.sh`, `.sql`, `.rs`, `.toml`; or it exactly matches a configured special-case filename such as `README.md`, `AGENTS.md`, `LICENSE`, `Makefile`, or `Cargo.toml`. If a backticked token does not satisfy these rules, it is not automatically considered a path and should not be resolved against the filesystem.

Implement path normalization next. Normalize candidate references to repository-relative paths from the repository root rather than resolving relative to the current Markdown file. This is a deliberate convention because repo-relative references are easier for both agents and linters to reason about consistently. The linter should flag relative same-directory references such as `./foo.md` or `../bar.md` unless the repository explicitly chooses to allow them. Normalize path separators, reject traversal that escapes the repository root, and resolve case sensitivity according to the running filesystem while warning when the literal spelling does not match the on-disk spelling on case-insensitive platforms. That warning prevents hidden drift that later breaks on Linux continuous integration.

With classification and normalization in place, implement the core lint rules. The first rule is `unresolved-backtick-path`: if inline code is classified as a path reference, the referenced file or directory must exist. The second rule is `prefer-backticks-for-local-paths`: if a Markdown link points to a repository-local file or directory and the link text does not add meaningful explanatory content beyond repeating the destination, the linter should report that the link should be replaced with a backticked repository-relative path. The third rule is `ambiguous-inline-code`: if inline code looks path-adjacent but does not satisfy the path grammar clearly enough, report it only in strict mode or as an informational warning, not as a hard failure at first. The fourth rule is `noncanonical-local-path`: if a repository-local path is written in noncanonical form, such as `./docs/PLANS.md` when the convention is `docs/PLANS.md`, report it and offer autofix.

Be conservative about `prefer-backticks-for-local-paths`. A link like `[architecture guide](docs/architecture.md)` carries human-readable anchor text that might be valuable in prose. A link like `[docs/PLANS.md](docs/PLANS.md)` or `[PLANS](docs/PLANS.md)` in an agent-oriented repository may still be unnecessary. The safe initial rule is narrower: autofix only links where the destination is local and the label is identical to the destination, differs only by formatting, or is a trivial filename echo. For all other local links, report with a message but do not autofix until the rule has proven low-noise in fixtures and real repository trials.

Implement configuration so repositories can tune the grammar without editing code. The configuration should support `include`, `exclude`, `allowed_extensions`, `special_filenames`, `doc_roots`, `allow_relative_paths`, `ignore_paths`, and `link_exceptions`. `link_exceptions` should be a list of file-and-pattern overrides for places where Markdown links are intentionally preferred, such as table-of-contents documents meant for human browsing. `ignore_paths` should allow temporary suppression for generated docs or imported third-party Markdown.

After the core rules exist, add autofix only for deterministic rewrites. The safe autofix set is: replace local Markdown links whose label is equivalent to the destination with a backticked normalized repository-relative path; normalize backticked path spelling to the canonical repository-relative form; and optionally remove leading `./` from otherwise valid paths. Do not autofix unresolved references, because the tool cannot know the author’s intended target. Do not autofix ambiguous inline code. Every autofix should preserve surrounding punctuation and whitespace. Where a rewrite changes Markdown structure rather than a plain text slice, prefer regenerating only the affected node or file through the mdast serialization path rather than piecemeal string surgery.

Testing must use fixtures rather than only unit-level parser mocks. Create a `fixtures/` area containing small synthetic repositories or document sets with known pass and fail expectations. Each fixture should include Markdown files, target files on disk, expected diagnostics, and expected autofix output where applicable. Include cases for valid backticked paths, valid external links, valid symbol references, invalid stale paths, same-name links rewritten to backticks, links with meaningful labels left untouched, code blocks ignored, Windows-style path separators rejected or normalized, and case-mismatch warnings. Add tests that assert line and column numbers, because diagnostics without stable positions are difficult to use in editors and continuous integration.

Once the linter is reliable in isolation, integrate it into the repository that adopts it. Add a command such as `cargo run -- lint .` during development or the installed CLI form `dglint lint .` in documentation and CI. Add continuous integration so every pull request validates documentation references. Update `AGENTS.md`, `README.md`, or the repository’s style guide to explain the convention in one short rule: repository-local file and directory references in prose should use backticks and repository-relative paths; Markdown links should be reserved for external URLs or explicit human-navigation exceptions.

The final implementation must be documented with clear operational guidance. A contributor should be able to install Rust, build the crate, run `dglint`, understand diagnostics, run autofix, and know when a warning is intentional instead of a bug. The docs should include examples of accepted and rejected forms, because examples will prevent style drift more effectively than abstract rule descriptions alone.

## Concrete Steps

Run the following commands from the repository root that will contain the linter.

1. Inspect the target repository to determine where the new Rust crate should live and what documentation scope should be linted first.

    Working directory: repository root

    Example commands:

        rg --files
        sed -n '1,220p' AGENTS.md
        sed -n '1,220p' docs/PLANS.md

    Expected outcome: You identify the repository’s documentation roots, how contributors are expected to run tooling, and whether the linter should initially scan all Markdown files or a narrower set.

2. Scaffold the Rust crate and CLI.

    Working directory: repository root or dedicated crate directory

    Example commands:

        cargo new --bin dglint
        cargo test
        cargo run -- --help

    Example outcome:

        dglint/
        dglint/Cargo.toml
        dglint/src/main.rs
        dglint/src/
        dglint/tests/
        dglint/fixtures/

    Expected outcome: The repository builds and runs a stub command like `dglint --help`.

3. Implement Markdown parsing and source position extraction with `markdown-rs`.

    Working directory: crate root

    Example command:

        cargo test parser

    Expected transcript excerpt:

        running 3 tests
        test parser::captures_inline_code_positions ... ok
        test parser::captures_markdown_links ... ok
        test parser::ignores_fenced_code_blocks_by_default ... ok

4. Implement classification and normalization.

    Working directory: crate root

    Example commands:

        cargo test classify
        cargo test normalize

    Expected outcome: Tests prove that `docs/PLANS.md` is classified as a local path, `Program` is not, `foo/bar` is a path candidate, and `https://example.com` is external.

5. Implement lint rules and diagnostics.

    Working directory: crate root

    Example command:

        cargo test rules

    Expected outcome: Fixture tests prove unresolved paths, unnecessary local links, and noncanonical local paths are reported with file, line, column, rule id, and message.

6. Implement autofix for the safe subset.

    Working directory: crate root

    Example command:

        cargo test fix

    Expected transcript excerpt:

        running 2 tests
        test fix::rewrites_identical_local_markdown_links_to_backticked_paths ... ok
        test fix::normalizes_dot_slash_paths ... ok

7. Run the linter against the repository itself and refine configuration to reduce false positives.

    Working directory: repository root

    Example commands:

        cargo run --manifest-path dglint/Cargo.toml -- lint .
        cargo run --manifest-path dglint/Cargo.toml -- fix .

    Expected outcome: Initial failures expose real stale references and a manageable number of convention mismatches. After fixes and configuration tuning, `dglint lint .` exits successfully on the repository.

8. Add continuous integration.

    Working directory: repository root

    Example outcome: The repository’s pull request validation runs `dglint lint .` and fails the build when violations are introduced.

9. Update documentation.

    Working directory: repository root

    Files to update will typically include `AGENTS.md`, `README.md`, and a docs style guide under `docs/`.

    Expected outcome: Contributors can read one concise rule and examples that match `dglint`’s behavior.

## Validation and Acceptance

Validation is complete only when all of the following are true.

Running the linter against fixture repositories proves correct behavior for both passing and failing cases. At minimum, there must be fixtures covering valid backticked file paths, broken backticked file paths, local Markdown links that should be rewritten to backticks, local Markdown links that should remain links because their label adds meaning, code spans that are symbols rather than paths, commands that are not paths, code blocks that are ignored, and path normalization edge cases.

Running the linter against the repository itself must demonstrate at least one of these observable outcomes. Either it finds real documentation drift that you then fix, or it passes cleanly after confirming existing docs already satisfy the convention. In both cases, the command and result must be recorded in this plan’s `Artifacts and Notes` section.

Acceptance should be phrased in behavior, not implementation details:

- When a Markdown file contains `docs/PLANS.md` in backticks and the file exists, `dglint` emits no error.
- When a Markdown file contains `docs/DOES-NOT-EXIST.md` in backticks, `dglint` reports `unresolved-backtick-path` with file, line, and column.
- When a Markdown file contains `[docs/PLANS.md](docs/PLANS.md)`, `dglint` reports `prefer-backticks-for-local-paths` and autofix rewrites it to `docs/PLANS.md`.
- When a Markdown file contains `[planning guide](docs/PLANS.md)`, `dglint` either leaves it untouched under the repository’s exception policy or reports it without autofix, depending on configured strictness.
- When a Markdown file contains `Program` or `npm test`, `dglint` does not incorrectly report these as missing files.
- When the repository’s continuous integration runs `dglint` on a pull request, a newly introduced broken repository-local path causes the job to fail.

## Idempotence and Recovery

The linter must be safe to run repeatedly. Scanning mode must never modify files. Autofix mode must only apply deterministic rewrites and must produce identical output on subsequent runs once the first fix has been applied. If an autofix attempt cannot be applied cleanly because the source document changed after parsing, the linter should fail that file with a clear message rather than partially rewriting content.

If rule tuning creates too many false positives during rollout, lower-risk recovery is to keep `unresolved-backtick-path` as an error and downgrade `ambiguous-inline-code` and `prefer-backticks-for-local-paths` to warnings temporarily. Another safe rollout option is to scope enforcement to a subset of files such as `AGENTS.md`, `README.md`, and `docs/**/*.md`, then expand once the rule quality is proven.

If a repository has many existing local Markdown links, enable autofix in a dedicated cleanup pull request before making the continuous integration job blocking. That keeps adoption incremental and reduces friction.

## Artifacts and Notes

Record concise evidence here as implementation proceeds. Replace the placeholders below with real transcripts and examples.

Expected diagnostic style example:

    docs/repository-knowledge/repository-knowledge-system.md:42:17  error  unresolved-backtick-path
    Backticked repository path `docs/architecture.md` does not exist from repository root.

Expected autofix example:

    Before:
    See [docs/PLANS.md](docs/PLANS.md) for execution plan rules.

    After:
    See `docs/PLANS.md` for execution plan rules.

Expected accepted forms:

    Use `docs/PLANS.md` when referring to the repository file.
    Use `web/src/app.tsx` when naming the frontend entry point.
    Use [OpenAI API docs](https://platform.openai.com/docs/) when the destination is external.

Expected rejected or warned forms:

    Use [docs/PLANS.md](docs/PLANS.md) for local file references.
    Use `PLANS` when you really mean `docs/PLANS.md`.
    Use `./docs/PLANS.md` if the repository convention requires repo-relative paths without leading `./`.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan updated to set the title to `Doc Gardener Linter`, set the CLI executable name to `dglint`, and commit to a Rust implementation built on `markdown-rs` and the Rust mdast serialization stack.
