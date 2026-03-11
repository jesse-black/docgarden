# Doc Gardening Linter

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. If that file exists in the target repository, re-read it before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, the repository will have a dedicated linter named `Doc Gardening Linter`, exposed as the executable `dglint`, that checks whether Markdown documentation refers to local repository files and directories using a repository-selected style policy and whether those references actually resolve to real paths in the working tree. A repository will be able to choose whether local file references should prefer compact backticked paths, Markdown links, or a narrower exception-based mix. Example representations:

    `docs/PLANS.md`
    [docs/PLANS.md](docs/PLANS.md)

The user-visible benefit is that teams can enforce one consistent convention, keep documentation mechanically valid, and still tune for either agent efficiency or human navigation.

You will know this is working when running `dglint .` against a repository reports real broken path references, reports style violations according to the configured local-reference policy, reports ambiguous or non-conforming file references, and exits successfully on a clean repository. The linter must be deterministic enough to run in local development and continuous integration, and durable enough to publish as a reusable open source command-line tool.

## Progress

- [x] (2026-03-06 18:20Z) Revise the ExecPlan so local file-reference style is configurable instead of implicitly hard-coded to backticks only.
- [x] (2026-03-06 18:40Z) Revise the ExecPlan so check-only mode explicitly reports when autofix is available and prints the corresponding autofix command plus a summary of fixable diagnostics.
- [x] (2026-03-06 19:05Z) Revise hypothetical path examples so dogfooding on this repository does not lint narrative samples as live references, and document that authoring rule in `AGENTS.md`, including the allowance for short inline hypothetical examples in plain inline code.
- [x] (2026-03-06 19:20Z) Extend the ExecPlan to require CLI integration tests that copy a fixture repository into a temporary working directory and to require coverage reporting for the automated test suite.
- [x] (2026-03-06 19:35Z) Revise the ExecPlan so known file extensions are configurable but seeded from a robust built-in default set derived from a maintained extension corpus rather than a short handwritten list.
- [x] (2026-03-06 19:45Z) Commit the initial configuration shape to a dedicated root-level `dglint.toml` with Ruff-like top-level keys, `per-file-ignores`, and gitignore-style glob semantics.
- [x] (2026-03-06 19:55Z) Lock down the remaining v1 config contract details: config discovery, CLI-over-config precedence, explicit special filename keys, repo-root-relative matching, no config layering, default scan targets, and symlink behavior.
- [x] (2026-03-06 21:15Z) Replace the placeholder Rust crate with a working `dglint` CLI that supports check mode by default, `--fix` for autofix, config discovery, plain-text diagnostics, and JSON output.
- [x] (2026-03-06 21:15Z) Implement an initial reference taxonomy, Markdown parsing on `markdown-rs`, internal path resolution, and core diagnostics for unresolved paths, style mismatches, and ambiguous inline code.
- [x] (2026-03-06 21:15Z) Implement initial style-policy configuration, ignore-pattern infrastructure, and deterministic autofix for link-to-backtick rewrites and link-generation rewrites under the configured style policy.
- [x] (2026-03-06 21:15Z) Add CLI integration tests that copy a checked-in test repository into a temporary working directory and verify lint/fix behavior end to end.
- [x] (2026-03-06 22:05Z) Amend the ExecPlan so the initial CLI contract includes basic colored human-readable output with `--color auto|always|never`, while leaving richer terminal presentation work out of scope.
- [x] (2026-03-06 22:40Z) Expand the automated coverage beyond the original CLI happy path by adding JSON-output tests, explicit-config tests, `--color always` coverage, and distinct backtick-style and link-style fixture repositories.
- [x] (2026-03-06 22:40Z) Add a GitHub Actions workflow that runs format checks, `cargo check`, Clippy, tests, coverage, and a dogfooding lint pass.
- [ ] (2026-03-07 00:00Z) Add targeted tests for the highest-risk uncovered branches reported by `cargo llvm-cov` (completed: recorded that the current suite reaches 81.96% region coverage overall; remaining: add focused coverage for config edge cases and the main lint/reference normalization branches instead of chasing a repo-wide percentage for its own sake).
- [ ] (2026-03-06 22:40Z) Update repository documentation to explain the selected local-reference style policy and the limited exceptions that remain allowed (completed: added a top-level `README.md` with build/run/test commands; remaining: document the repo's chosen dogfooding config and style guidance once the false-positive policy is settled).
- [ ] (2026-03-06 00:00Z) Capture final evidence, update this plan, and move it to `docs/exec-plans/completed/` when the linter is adopted.

## Surprises & Discoveries

- Observation: Dogfooding the first implementation on this repository produces many false positives from example-oriented docs and the active ExecPlan.
  Evidence: `cargo run -- .` reported unresolved example paths such as the examples below even after excluding checked-in test fixtures through `dglint.toml`.

      crates/parser
      pyproject.toml
      docs/generated/**

- Observation: Canonicalization added more style noise than value during dogfooding because it required extra repo-specific policy without catching broken references.
  Evidence: The first implementation flagged `docs/`, `src/`, and `docs/exec-plans/` as `noncanonical-local-path` fixups even though those forms were understandable and resolvable, which led to removing the rule rather than documenting a canonical spelling policy.

- Observation: Hyphenated TOML keys and separate style fixtures make the configuration and fixture strategy much clearer for end-to-end tests than the initial single-fixture setup.
  Evidence: The test suite now passes with distinct repositories under `tests/test-repos/backticks/` and `tests/test-repos/links/`, plus fixture-local `dglint.toml` files that use `local-reference-style = "backticks"` and `local-reference-style = "links"`.

- Observation: The current coverage bar is technically met, but the remaining gaps cluster in the code that is most likely to produce user-visible lint regressions.
  Evidence: `cargo llvm-cov --summary-only` currently reports 81.96% region coverage overall, with the lowest module coverage in `src/config.rs`, `src/lint/mod.rs`, and `src/lint/references.rs`.

## Decision Log

- Decision: Support a configurable local-reference style policy, with backticks as the initial default and Markdown-link preference available through configuration.
  Rationale: The core product is mechanical validation of repository-local references, not a permanently hard-coded opinion about one representation. Backticks remain the best default for agent-oriented repositories, but the tool will be more reusable if repositories can opt into link-first behavior or narrow exception lists without forking rule logic.
  Date/Author: 2026-03-06 / Codex

- Decision: Use a dedicated `dglint.toml` file with root-level TOML keys for the initial configuration format.
  Rationale: TOML matches Rust ecosystem expectations and is comfortable for hand-edited linter configuration. A dedicated `dglint.toml` keeps the first open-source release simpler than supporting embedded configuration in shared tool files, while still leaving room for future embedding under a table such as `[tool.dglint]` if real demand appears later.
  Date/Author: 2026-03-06 / Codex

- Decision: Follow Ruff-like configuration ergonomics for discovery and overrides, while keeping the schema specific to doc linting.
  Rationale: Ruff’s configuration style is familiar, compact, and practical for root-level `include`, `exclude`, and per-file ignores. Adopting that shape makes `dglint` easier to learn without inheriting Python-specific concepts that do not fit this tool.
  Date/Author: 2026-03-06 / Codex

- Decision: Use gitignore-style glob semantics for include, exclude, and per-file ignore patterns.
  Rationale: Contributors already understand gitignore-style matching, especially in repositories that use `.gitignore`, `.ignore`, or similar tooling. Reusing that mental model reduces ambiguity and makes it easier to explain how patterns such as `docs/generated/**` or `node_modules/**` behave.
  Date/Author: 2026-03-06 / Codex

- Decision: Keep configuration discovery and precedence simple in v1: use one dedicated `dglint.toml`, make all patterns repo-root-relative, let CLI flags override config, and do not support layered or nested config files.
  Rationale: Discovery and precedence bugs are a common source of user confusion in new tools. A single discovered config file plus explicit `--config` override keeps behavior easy to explain and test. Repo-root-relative matching avoids cwd-dependent surprises, and refusing nested config files or inheritance prevents a large amount of accidental complexity in the first release.
  Date/Author: 2026-03-06 / Codex

- Decision: Do not follow directory symlinks during scanning in v1, and lint only real files within the repository root unless a future plan expands that behavior.
  Rationale: Symlink traversal can create duplicate work, loops, or confusing paths that appear to leave the repository tree. The safest initial behavior is to lint regular files selected from the repository root and skip symlinked directories.
  Date/Author: 2026-03-06 / Codex

- Decision: Treat `dglint` as a two-mode CLI with explicit check-only and autofix user experience requirements.
  Rationale: The plan already required a non-mutating scan mode and a mutating autofix mode, but adoption will be materially better if check-only mode tells users when safe rewrites are available, how to invoke them, and which reported violations are fixable. Making that guidance part of the CLI contract prevents a weaker implementation that technically supports autofix but hides discoverability.
  Date/Author: 2026-03-06 / Codex

- Decision: Include basic colored human-readable output in the initial CLI, but defer richer terminal presentation to future work.
  Rationale: Severity coloring and a small amount of styling materially improve day-to-day usability for a linter, while still being easy to keep deterministic and testable. More ambitious presentation ideas such as custom themes, hyperlinks, or rich terminal layouts are useful but should not expand the first implementation scope.
  Date/Author: 2026-03-06 / Codex

- Decision: Validate the CLI with copied test repositories and report coverage with LLVM-based Rust coverage tooling.
  Rationale: The most important behavior is command-line behavior against a real repository tree, not only internal rule functions. A dedicated test-repository fixture copied into a temporary working directory makes it possible to assert diagnostics, exit codes, and autofix behavior without mutating checked-in fixtures. Coverage should be collected with `cargo-llvm-cov` because plain Cargo does not emit useful coverage summaries by itself.
  Date/Author: 2026-03-06 / Codex

- Decision: Treat the next coverage work as targeted regression protection for uncovered linter and config branches, not as a generic percentage-raising exercise.
  Rationale: The repository already clears the stated 80% expectation, so a broad “raise coverage” task would invite low-value tests. The remaining risk is concentrated in path classification, normalization, and config edge cases, so the plan should direct effort there explicitly.
  Date/Author: 2026-03-07 / Codex

- Decision: Make known file extensions configurable, but seed the default extension and filename set from a vendored snapshot of GitHub Linguist data.
  Rationale: A short handwritten allowlist will underfit real repositories and create churn as new common file types are discovered. GitHub Linguist maintains a broad, widely used mapping of filenames and extensions across programming, markup, prose, and data formats, which makes it the best fast-start source for robust defaults. The linter should vendor a filtered snapshot of that data into the crate rather than depending on a live network fetch or a Ruby runtime at execution time.
  Date/Author: 2026-03-06 / Codex

- Decision: Name the tool `Doc Gardening Linter` and expose the executable as `dglint`.
  Rationale: The human-facing title matches the broader doc-gardening practice while avoiding the awkward stacked-agent phrasing of `Doc Gardener Linter`. The shorter executable name remains efficient for repeated local and CI use. This also leaves room for a future higher-level doc-gardening agent or workflow that may use `dglint` as one component.
  Date/Author: 2026-03-06 / Codex

- Decision: Implement the first version in Rust using `markdown-rs` for Markdown-to-mdast parsing.
  Rationale: The project targets durable open source adoption, and Rust provides strong single-binary distribution, performance, and CLI ergonomics. `markdown-rs` provides mdast parsing aligned with the broader remark ecosystem, which removes a major historical drawback of Rust for Markdown tooling.
  Date/Author: 2026-03-06 / Codex

- Decision: Do not treat every inline code span as a path reference.
  Rationale: Inline code spans are also used for commands, symbols, environment variables, and code fragments. The linter must classify only path-shaped strings as path references to avoid excessive false positives.
  Date/Author: 2026-03-06 / Codex

- Decision: Require repository-local path references to be repository-relative and path-shaped.
  Rationale: A stable grammar reduces ambiguity and allows mechanical validation. Representative examples are shown below.

      docs/PLANS.md
      web/src/app.tsx
      api/Program.cs
      terraform/modules/network

  Date/Author: 2026-03-06 / Codex

## Outcomes & Retrospective

The repository now contains a working first implementation of `dglint` rather than just a plan. The CLI supports check mode by default and `--fix` for autofix, loads a dedicated `dglint.toml`, discovers Markdown files with gitignore-style include and exclude patterns, parses Markdown with `markdown-rs`, emits diagnostics with file, line, and column, and applies a safe subset of autofixes through the mdast-to-Markdown serialization path. A copied-fixture CLI integration test proves the end-to-end lint and fix loop against a temporary working repository.

The current implementation is still an early milestone rather than the finished product described by the full plan. Dogfooding shows that the classifier is too eager on example-heavy documents and directory-style references, but the automated validation is now materially stronger: there are explicit backtick-style and link-style fixture repositories, JSON and color-mode coverage, explicit-config coverage, a CI workflow, and an 81.96% region-coverage baseline from `cargo llvm-cov`. The next iteration should focus on reducing false positives, deciding how to treat directory references with trailing slashes, adding targeted tests for the remaining high-risk uncovered branches, and documenting the repository’s dogfooding policy once the rule noise is low enough to make that policy stable.

## Context and Orientation

This plan assumes a repository that uses Markdown documentation heavily and wants a structured knowledge base where `AGENTS.md` acts as a small map and deeper guidance lives elsewhere under `docs/`. The problem to solve is not generic prose linting. The problem is documentation drift in path references: a Markdown file claims a repository path exists or links to a repository path, but the path is stale, inconsistent with the repository’s chosen style policy, or represented in a way the repository has decided not to allow. Example hypothetical references:

    `docs/PLANS.md`
    [docs/architecture.md](docs/architecture.md)

In this plan, a "repository-local path reference" means a textual mention of a file or directory that is expected to exist in the same repository as the linter is running against. A "local-reference style policy" means the repository rule that decides whether those references should normally appear as backticked repository-relative paths, as Markdown links, or as one representation with documented exceptions. A "Markdown link" means standard Markdown link syntax such as the example below. An "external link" means a destination outside the repository, such as the URL shown below; these remain valid and should never be rewritten according to local-path style rules. An "ambiguous inline code span" means backticked text that could be a path or could be a code fragment, for example `build`, `Program`, or `foo.bar`, where the linter cannot confidently infer that the author meant a repository path.

    [Plans](docs/PLANS.md)
    https://example.com

The linter should operate on Markdown files, probably under the configured documentation roots shown below. It must parse Markdown structure rather than relying only on regular expressions because fenced code blocks, inline code spans, link destinations, autolinks, and escaped punctuation behave differently. It should ignore fenced code blocks by default because those blocks often contain examples that are not intended to be live repository references. It should inspect prose, headings, list items, tables if present, and inline code spans.

    docs/
    README.md
    AGENTS.md

The implementation should be repository-agnostic. It belongs in its own repository or crate so other repositories can adopt it. That means configuration must be explicit. The linter should accept a repository root to scan, a set of repository-relative include and exclude patterns, and a small dedicated TOML configuration file named `dglint.toml` describing allowed path extensions, documentation roots, local-reference style policy, and exceptions. In v1, config discovery should be simple: if `--config` is provided, load exactly that file; otherwise look for `dglint.toml` at the repository root being scanned. Do not merge multiple config files, do not walk parent directories beyond the selected repository root, and do not support nested per-directory config files in the initial implementation.

The Rust implementation should build on `markdown-rs` to parse source Markdown into mdast nodes with source positions. For autofix support, the implementation should use the Rust mdast serialization stack to write edited mdast trees back to Markdown text. This keeps parsing and serialization aligned around mdast rather than mixing unrelated text-manipulation approaches.

## Plan of Work

Start by creating a standalone Rust crate for `Doc Gardening Linter` that builds a CLI executable named `dglint`. Use Cargo as the package manager and test runner. The crate should expose a small command-line interface with at least these modes: check mode by default when invoked as `dglint <path>`, machine-readable output, and autofix when invoked with `--fix`. Treat the user-facing CLI contract as two primary human workflows: a check-only mode that scans and reports without modifying files, and an autofix mode that applies the safe deterministic rewrites. In check-only mode, every diagnostic that is safe to rewrite should be marked as fixable. If any fixable diagnostics are found, the CLI must print a short fix summary after the diagnostics, including how many violations are autofix candidates, which rule kinds are fixable in the current run, and the exact autofix command the user should run next for the same repository root, globs, and configuration file. Human-readable output should support basic ANSI color styling with `--color auto|always|never`. The `auto` mode should colorize only when writing to a TTY, `always` should force colorized human-readable output, `never` should disable color, and machine-readable JSON output must remain uncolored regardless of the color setting. The command-line interface should accept a repository root, optional file globs, and a configuration file path. It should exit with code `0` when no violations are found, `1` when violations are found, and a distinct non-zero code when configuration or parser errors prevent a complete run.

Implement parsing on top of a real Markdown abstract syntax tree rather than trying to lint with line-oriented regular expressions. Use `markdown-rs` to parse Markdown into mdast, and preserve source positions for inline code spans, link nodes, text nodes, and other relevant nodes. Source positions matter because the linter needs to print file, line, and column for each violation and because safe autofix requires precise replacements or stable AST-to-Markdown regeneration.

Define the reference taxonomy in code before implementing rules. A repository-local path reference must satisfy all of the following conditions unless a configuration override says otherwise. It must be repository-relative, not absolute. It must be path-shaped. "Path-shaped" means at least one of these is true: it contains a `/`; it starts with `./` or `../`; it ends with a known file extension; or it exactly matches a configured special-case filename. The known-extension and special-filename defaults must be robust and configurable rather than hard-coded to a tiny starter list. Seed the built-in defaults from a vendored snapshot of GitHub Linguist's extension and filename data, then filter that corpus down to the text-like file kinds the linter should plausibly treat as repository-local references by default. The configuration layer must allow repositories to add, remove, or override extensions and special filenames without patching code. If a backticked token does not satisfy these rules, it is not automatically considered a path and should not be resolved against the filesystem.

Implement path resolution next. Resolve candidate references to internal repository-relative paths for existence checks while preserving the author-written display form unless a configured style rule requires a representation change. Relative same-directory references such as the examples below should remain valid if they resolve; they should not be flagged merely for being relative.

    ./foo.md
    ../bar.md

Normalize path separators internally for resolution, reject traversal that escapes the repository root, and resolve case sensitivity according to the running filesystem while warning when the literal spelling does not match the on-disk spelling on case-insensitive platforms. That warning prevents hidden drift that later breaks on Linux continuous integration.

With classification and normalization in place, implement configuration-backed style semantics before finalizing the core lint rules. The configuration should expose `local_reference_style` with at least `backticks` and `links` values, plus per-path exceptions for locations that intentionally diverge. Under `backticks`, the linter should prefer inline-code repository-relative paths for local references. Under `links`, the linter should prefer Markdown links for local references and treat bare backticked paths as style violations unless the configured exception policy allows them. Regardless of style, local references must still resolve to real repository paths.

Implement the core lint rules as style-aware rules. The first rule is `unresolved-local-path`: if either inline code or a Markdown link is classified as a repository-local path reference, the referenced file or directory must exist. The second rule is `prefer-backticks-for-local-paths`: when `local_reference_style` is `backticks`, a Markdown link that points to a repository-local path and does not add meaningful explanatory content beyond repeating the destination should be rewritten as a backticked path reference. The third rule is `prefer-links-for-local-paths`: when `local_reference_style` is `links`, a bare backticked path reference in prose should be rewritten to a Markdown link form when the repository wants navigable links by default. The fourth rule is `ambiguous-inline-code`: if inline code looks path-adjacent but does not satisfy the path grammar clearly enough, report it only in strict mode or as an informational warning, not as a hard failure at first.

Be conservative about style-enforcement rules. A link like the example below carries human-readable anchor text that might be valuable in prose. A backticked path like the second example below may also be intentional in a link-first repository if it appears inside a code-focused section or a configured exception file.

    [architecture guide](docs/architecture.md)
    `docs/architecture.md`

The safe initial rule is narrower: autofix only links whose label is equivalent to the destination, differs only by formatting, or is a trivial filename echo, and only autofix backticks to links when the canonical link text can be generated mechanically without changing the sentence meaning. For all other local style mismatches, report with a message but do not autofix until the rule has proven low-noise in fixtures and real repository trials.

Implement configuration so repositories can tune both the grammar and the style policy without editing code. The initial configuration file format should be a dedicated root-level `dglint.toml`. Follow a Ruff-like top-level structure rather than nested tool tables for v1 because the file is dedicated to this tool. At minimum, the file must support root-level keys such as `include`, `exclude`, `local-reference-style`, `extend-extensions`, `remove-extensions`, `extend-special-filenames`, `remove-special-filenames`, and a `[per-file-ignores]` table. The extension and filename settings must be layered on top of the robust built-in defaults rather than replacing them with an underspecified minimal list. Embedded configuration in shared files such as the examples below is explicitly out of scope for the initial implementation. CLI flags must override configuration-file values when both are present.

    pyproject.toml
    Cargo.toml

The built-in default scan target set should be explicit in the implementation and documentation. Start from repository-root-relative Markdown-oriented defaults that include:

    docs/**
    README.md
    AGENTS.md
    *.md

Then let configuration and CLI options widen or narrow that set. All `include`, `exclude`, and `[per-file-ignores]` patterns are matched relative to the repository root, not relative to the current working directory and not relative to the location of `dglint.toml`.

The intended shape is as follows:

    exclude = ["node_modules/**"]
    include = ["docs/**", "README.md", "AGENTS.md"]
    extend-extensions = [".proto"]
    remove-extensions = [".txt"]
    extend-special-filenames = ["Justfile"]
    remove-special-filenames = ["LICENSE"]

    [per-file-ignores]
    "docs/legacy.md" = ["prefer-backticks-for-local-paths"]
    "docs/generated/**" = ["ambiguous-inline-code"]

Build ignore-pattern infrastructure into the first configuration layer using that `dglint.toml` structure from the start. The implementation should support two concepts from the start. The first is a whole-file exclusion that removes matching files from all linting through root-level `exclude` patterns. The second is a rule-specific exclusion that still scans a file but suppresses named rules for matching paths through `[per-file-ignores]`. Pattern matching for `include`, `exclude`, and `[per-file-ignores]` must use gitignore-style glob semantics and must be documented with clear precedence rules. The initial precedence rule should be: start from the built-in default documentation targets, apply config-file `include` to widen or narrow the candidate set, apply config-file `exclude` to remove matches, apply CLI include or exclude overrides if present, then apply `[per-file-ignores]` only after a file is selected for scanning. Skip symlinked directories during file discovery in v1, and treat symlink handling as an explicit future extension if users need it.

After the core rules exist, add autofix only for deterministic rewrites. The safe autofix set is: replace local Markdown links whose label is equivalent to the destination with a backticked path reference when `local_reference_style` is `backticks`; and replace local backticked paths with a Markdown link when `local_reference_style` is `links` and the link text plus destination can be generated deterministically. Do not autofix unresolved references, because the tool cannot know the author’s intended target. Do not autofix ambiguous inline code. Every autofix should preserve surrounding punctuation and whitespace. Where a rewrite changes Markdown structure rather than a plain text slice, prefer regenerating only the affected node or file through the mdast serialization path rather than piecemeal string surgery. The diagnostics model should therefore distinguish between fixable and non-fixable violations so check-only mode can accurately explain what autofix would change without mutating any files.

Testing must use fixtures rather than only unit-level parser mocks. Create a fixture area such as the example path below containing small synthetic repositories or document sets with known pass and fail expectations. Each fixture should include Markdown files, target files on disk, expected diagnostics, and expected autofix output where applicable. Include cases for valid backticked paths, valid link-style local references, valid external links, valid symbol references, invalid stale paths, same-name links rewritten to backticks, bare backticked paths rewritten to links in link-first mode, links with meaningful labels left untouched, code blocks ignored, Windows-style path separators rejected or normalized, and case-mismatch warnings. Add tests that assert line and column numbers, because diagnostics without stable positions are difficult to use in editors and continuous integration. Add configuration tests that prove repositories can extend and shrink the default extension set, and snapshot tests that prove the vendored default extension corpus is loaded deterministically.

    fixtures/

The fixture set must explicitly cover both major local-reference styles as first-class repository scenarios, not just isolated rule cases. Add at least two checked-in test repositories: one configured for backtick-first local references and one configured for link-first local references. Each repository fixture should include passing examples, failing examples, and autofix candidates appropriate to its configured style so both configurations are exercised end to end.

In addition to rule-level and parser-level tests, add automated CLI integration tests under `tests/` that operate on checked-in test repository fixtures. Store representative sample repositories under paths such as `tests/test-repos/backticks/` and `tests/test-repos/links/`. Each integration test must copy the relevant fixture tree into a temporary working directory before invoking the compiled CLI so tests can freely run check-only mode, machine-readable mode, and autofix mode without mutating the checked-in source fixture. Those tests should assert process exit codes, emitted diagnostics, post-diagnostic autofix hints, rewritten file contents after autofix, and rerun idempotence after a fix has already been applied. The automated suite must exercise both style configurations, not just the backtick-first default.

Add coverage reporting as part of the documented validation flow. Use `cargo test` for normal automated test execution and `cargo llvm-cov` for coverage output over unit tests, fixture tests, and CLI integration tests. The plan does not need to lock in a minimum percentage yet, but it must require generating and recording a coverage summary so gaps in CLI-path testing are visible.

Once the linter is reliable in isolation, integrate it into the repository that adopts it. Add a command such as `cargo run -- .` during development or the installed CLI form `dglint .` in documentation and CI. Add continuous integration so every pull request validates documentation references. Update `AGENTS.md`, `README.md`, or the repository’s style guide to explain the selected convention in one short rule: repository-local file and directory references in prose should follow the configured local-reference style policy, and exceptions should be explicit rather than ad hoc.

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

    Expected outcome: Tests prove the examples below are classified correctly.

        `docs/PLANS.md` -> local path
        `Program` -> not a path
        `foo/bar` -> path candidate
        `https://example.com` -> external

    Additional expected outcome: Tests prove the built-in default extension set recognizes common documentation and source-code paths without requiring per-repository configuration, while TOML overrides can add or remove specific extensions or special filenames.

5. Implement style-policy configuration, ignore-pattern infrastructure, and lint rules.

    Working directory: crate root

    Example commands:

        cargo test config
        cargo test ignores
        cargo test rules

    Expected outcome: Fixture tests prove unresolved paths, style-policy violations, and ignore-pattern behavior are reported with file, line, column, rule id, and message.
    Additional expected outcome: Configuration tests prove `dglint.toml` root-level `include` and `exclude` keys, gitignore-style matching, `extend-special-filenames` and `remove-special-filenames`, CLI-over-config precedence, and `[per-file-ignores]` behave as documented.

6. Implement autofix for the safe subset.

    Working directory: crate root

    Example command:

        cargo test fix

    Expected transcript excerpt:

        running 3 tests
        test fix::rewrites_identical_local_markdown_links_to_backticked_paths ... ok
        test fix::rewrites_backticked_paths_to_links_in_link_mode ... ok
        test fix::normalizes_dot_slash_paths ... ok

7. Add CLI integration tests that use copied test repositories.

    Working directory: crate root

    Example commands:

        cargo test cli
        cargo test integration

    Expected outcome: Integration tests copy checked-in test repository fixtures into temporary working directories, run the compiled `dglint` binary against those copies, and prove the CLI emits the expected diagnostics, exit codes, autofix hints, and file rewrites without mutating the source fixtures. At least one integration path must cover a backtick-style repository and at least one must cover a link-style repository. Include at least one CLI-output assertion for `--color never` so the baseline human-readable output stays testable without ANSI escape noise.

8. Generate coverage output for the full test suite.

    Working directory: crate root

    Example commands:

        cargo llvm-cov --summary-only
        cargo llvm-cov --html

    Expected outcome: Coverage output includes parser tests, rule tests, autofix tests, and CLI integration tests. Record the summary in this plan’s `Artifacts and Notes` section so future contributors can see whether end-to-end CLI paths are exercised, and use that summary to choose the next targeted tests for uncovered branches in `src/config.rs`, `src/lint/mod.rs`, and `src/lint/references.rs`.

9. Run the linter against the repository itself and refine configuration to reduce false positives.

    Working directory: repository root

    Example commands:

        cargo run --manifest-path dglint/Cargo.toml -- .
        cargo run --manifest-path dglint/Cargo.toml -- . --fix

    Expected outcome: Initial failures expose real stale references and a manageable number of convention mismatches. After fixes and configuration tuning, `dglint .` exits successfully on the repository.

10. Add continuous integration.

    Working directory: repository root

    Example outcome: The repository’s pull request validation runs `cargo test`, runs `cargo llvm-cov --summary-only` or an equivalent coverage command, runs `dglint .`, and fails the build when violations or test regressions are introduced.

11. Update documentation.

    Working directory: repository root

    Files to update will typically include `AGENTS.md`, `README.md`, and a docs style guide under `docs/`.

    Expected outcome: Contributors can read one concise rule and examples that match the repository’s chosen `dglint` policy.

## Validation and Acceptance

Validation is complete only when all of the following are true.

Running the linter against fixture repositories proves correct behavior for both passing and failing cases. At minimum, there must be fixtures covering valid backticked file paths, valid link-style local references, broken local paths in both representations, local Markdown links that should be rewritten to backticks in backtick mode, backticked paths that should be rewritten to links in link mode, local Markdown links that should remain links because their label adds meaning, code spans that are symbols rather than paths, commands that are not paths, code blocks that are ignored, and path normalization edge cases.

Those fixtures must include two repository-level style configurations as first-class scenarios: one fixture repository configured for backticks and one fixture repository configured for links. Acceptance is not complete until both styles are exercised as full-repository test cases rather than only as isolated unit fixtures.

The automated tests must also prove that the default extension and filename corpus is robust enough for common documentation and source repositories out of the box, and that configuration can override those defaults safely. At minimum, add tests for a common documentation extension, a common programming extension, a special filename with no extension, and a repository-specific override that adds one extension and removes another.

Configuration acceptance is not complete until tests prove that `dglint.toml` is discovered and parsed in its dedicated root-level form, that `--config` overrides discovery, that `include` and `exclude` use gitignore-style glob semantics rooted at the repository root, that CLI settings override config-file settings, that `extend-special-filenames` and `remove-special-filenames` work as documented, that no nested config layering occurs, and that `[per-file-ignores]` suppresses only the named rules for matching files that were otherwise selected for scanning.

File-discovery acceptance is not complete until tests prove the built-in default scan target set is applied as documented and that symlinked directories are skipped during v1 scanning.

Automated CLI integration tests must use checked-in test repository fixtures that are copied into temporary working directories before each test run. Acceptance is not complete until those tests prove check-only mode, machine-readable mode, and autofix mode against both a backtick-style repository fixture and a link-style repository fixture, including exit codes, stdout or stderr diagnostics, autofix-hint output, rewritten file contents after autofix, and a second run that confirms autofix idempotence.

CLI output acceptance is not complete until tests prove that human-readable output honors `--color auto|always|never`, that JSON output remains uncolored, and that severity, rule identifiers, or fixable markers are the only elements receiving the initial styling treatment.

Coverage validation is also required. Run `cargo llvm-cov` across the automated test suite and record the resulting summary in `Artifacts and Notes`. The current baseline is 81.96% region coverage overall. The goal is not a hard numeric gate yet; the goal is to prove that coverage reporting exists, that the CLI integration tests contribute to exercised code paths, and that the next test additions are chosen to cover currently underexercised config and lint branches rather than to inflate the percentage with low-signal tests.

Running the linter against the repository itself must demonstrate at least one of these observable outcomes. Either it finds real documentation drift that you then fix, or it passes cleanly after confirming existing docs already satisfy the convention. In both cases, the command and result must be recorded in this plan’s `Artifacts and Notes` section.

Acceptance should be phrased in behavior, not implementation details:

- When a Markdown file contains the example below in backticks, the file exists, and `local_reference_style` is `backticks`, `dglint` emits no error.

      docs/PLANS.md

- When a Markdown file contains the example below as a Markdown link, the file exists, and `local_reference_style` is `links`, `dglint` emits no error.

      [docs/PLANS.md](docs/PLANS.md)

- When a Markdown file contains one of the examples below, `dglint` reports `unresolved-local-path` with file, line, and column.

      `docs/DOES-NOT-EXIST.md`
      [docs/DOES-NOT-EXIST.md](docs/DOES-NOT-EXIST.md)

- When a Markdown file contains the example below and `local_reference_style` is `backticks`, `dglint` reports `prefer-backticks-for-local-paths` and autofix rewrites it to the backticked form.

      [docs/PLANS.md](docs/PLANS.md)

- When a Markdown file contains the example below in backticks and `local_reference_style` is `links`, `dglint` reports `prefer-links-for-local-paths` and autofix rewrites it to a Markdown link with the same destination or another configured canonical label form.

      docs/PLANS.md

- When a Markdown file contains the example below, `dglint` either leaves it untouched under the repository’s exception policy or reports it without autofix, depending on configured strictness.

      [planning guide](docs/PLANS.md)
- When a Markdown file contains `Program` or `npm test`, `dglint` does not incorrectly report these as missing files.
- When `dglint` runs in check-only mode and finds one or more fixable violations, it prints a post-diagnostic summary that states how many violations are fixable, identifies the fixable rule kinds found in that run, and prints the exact autofix command for the same invocation shape, including `--config` or path/glob arguments when present.
- When `dglint` runs in check-only mode and finds only non-fixable violations such as `unresolved-local-path`, it does not advertise autofix for those violations.
- When an integration test copies a checked-in test repository fixture into a temporary working directory and runs `dglint` against that copy, the CLI returns the expected exit code and diagnostics without mutating the source fixture directory.
- When an integration test runs autofix against the copied test repository, the expected files change in the working copy, and a second `dglint` run over that same working copy confirms the fix was applied cleanly.
- When the test suite runs under `cargo llvm-cov`, the coverage summary includes the CLI integration-test run and is captured in the plan artifacts.
- When the repository’s continuous integration runs `dglint` on a pull request, a newly introduced broken repository-local path causes the job to fail.

## Idempotence and Recovery

The linter must be safe to run repeatedly. Check-only mode must never modify files. Autofix mode must only apply deterministic rewrites and must produce identical output on subsequent runs once the first fix has been applied. If an autofix attempt cannot be applied cleanly because the source document changed after parsing, the linter should fail that file with a clear message rather than partially rewriting content. The check-only summary that advertises autofix must also be deterministic: rerunning the same command on the same tree should produce the same fixable-count summary and the same suggested autofix command text.

If rule tuning creates too many false positives during rollout, lower-risk recovery is to keep `unresolved-local-path` as an error and downgrade `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, and `prefer-links-for-local-paths` to warnings temporarily. Another safe rollout option is to scope enforcement to a subset of files such as `AGENTS.md`, `README.md`, and `docs/**/*.md`, then expand once the rule quality is proven.

If a repository has many existing local Markdown links, enable autofix in a dedicated cleanup pull request before making the continuous integration job blocking. That keeps adoption incremental and reduces friction.

## Artifacts and Notes

Record concise evidence here as implementation proceeds. Replace the placeholders below with real transcripts and examples.

Expected diagnostic style example:

    docs/bootstrap-repository-knowledge.md:42:17  error  unresolved-local-path
    Local repository path `docs/architecture.md` does not exist from repository root.

Expected link-mode diagnostic example:

    docs/bootstrap-repository-knowledge.md:42:17  error  prefer-links-for-local-paths
    Local repository path `docs/architecture.md` should use Markdown link syntax under the configured style policy.

Expected check-only autofix hint example:

    docs/bootstrap-repository-knowledge.md:42:17  error  prefer-backticks-for-local-paths  fixable
    Local repository link `[docs/architecture.md](docs/architecture.md)` should use backticks under the configured style policy.

    1 fixable violation found across 1 file.
    Fixable rules in this run: prefer-backticks-for-local-paths
    Run `dglint . --fix --config dglint.toml` to apply safe rewrites.

Expected autofix example:

    Before:
    See [docs/PLANS.md](docs/PLANS.md) for execution plan rules.

    After:
    See `docs/PLANS.md` for execution plan rules.

Expected accepted forms:

    Use `docs/PLANS.md` when referring to the repository file.
    Use [docs/PLANS.md](docs/PLANS.md) when the repository is configured for link-first local references.
    Use `web/src/app.tsx` when naming the frontend entry point.
    Use [OpenAI API docs](https://platform.openai.com/docs/) when the destination is external.

Expected rejected or warned forms:

    Use [docs/PLANS.md](docs/PLANS.md) for local file references.
    Use `docs/PLANS.md` for local file references when the repository is configured to require links.
    Use `PLANS` when you really mean `docs/PLANS.md`.
    Use `./docs/PLANS.md` if the repository convention requires repo-relative paths without leading `./`.

At the bottom of this plan, append a revision note every time the plan changes materially, describing what changed and why.

Revision note: Initial plan updated to set the title to `Doc Gardener Linter`, set the CLI executable name to `dglint`, and commit to a Rust implementation built on `markdown-rs` and the Rust mdast serialization stack.

Revision note: Updated the plan to treat local file-reference style as configurable, added a style-policy milestone and rule set, and revised validation examples so `dglint` can enforce either backtick-first or link-first repository conventions.

Revision note: Added ignore-pattern infrastructure to the current plan as foundational configuration work, while intentionally deferring the exact user-facing syntax until the configuration shape is better understood.

Revision note: Selected TOML as the configuration file format for `dglint`, while still leaving the detailed key structure and override syntax open for further design work.

Revision note: Clarified the CLI contract as two primary user workflows, check-only and autofix, and required check-only mode to mark fixable diagnostics, summarize what autofix can change, and print the exact follow-up autofix command whenever safe rewrites are available.

Revision note: Moved hypothetical path examples in this plan into indented code blocks so the repository can dogfood `dglint` on its own docs without linting narrative samples as live references.

Revision note: Added automated CLI integration-test requirements based on copying checked-in test repositories into temporary working directories, and added `cargo llvm-cov` coverage reporting to the validation and CI expectations.

Revision note: Replaced the tiny example-based extension list with a requirement for configurable extension detection backed by a robust built-in default corpus, with GitHub Linguist named as the intended seed source for vendored defaults.

Revision note: Committed the initial config format to a dedicated root-level `dglint.toml`, adopted Ruff-like root-level keys and `per-file-ignores`, declared embedded config out of scope for v1, and specified gitignore-style glob semantics plus precedence expectations.

Revision note: Added the remaining initial config-contract details: explicit config discovery and `--config` behavior, CLI-over-config precedence, special-filename add/remove keys, repo-root-relative pattern evaluation, no config layering in v1, explicit default scan targets, and a conservative symlink policy.

Revision note: Renamed the human-facing tool from `Doc Gardener Linter` to `Doc Gardening Linter` while keeping the executable name `dglint`, so the linter name reads naturally alongside a separate `Doc Gardener` agent persona.

Revision note: Replaced the placeholder Rust binary with a working first implementation of `dglint`, including config loading, Markdown parsing, core lint rules, deterministic autofix for the safe subset, and copied-fixture CLI integration tests; also recorded the first dogfooding false-positive findings for the next implementation pass.

Revision note: Clarified that the fixture strategy must include two repository-level style fixtures, one for backtick-first local references and one for link-first local references, and that both configurations must be exercised by the automated CLI tests.

Revision note: Added a narrow initial CLI styling requirement covering ANSI color support for human-readable output with `--color auto|always|never`, while explicitly leaving richer terminal presentation work out of scope for the first implementation.

Revision note: Updated the CLI contract to use the more standard linter shape where `dglint` without subcommands runs checks and `dglint --fix` applies safe rewrites, replacing the earlier `lint` and `fix` subcommands.

Revision note: Expanded the implementation and validation story to include separate backtick-style and link-style fixture repositories, JSON/config/color tests, repository-level `dglint.toml` dogfooding exclusions for `tests/**`, a brief top-level `README.md`, and a GitHub Actions workflow for formatting, linting, testing, coverage, and dogfooding.

Revision note: Updated the repository authoring guidance so short inline hypothetical examples may use plain inline code in addition to indented code blocks, and rewrote several noisy hypothetical examples in this plan so dogfooding output better matches that guidance.

Revision note: Added a scoped follow-up item to use the recorded `cargo llvm-cov` baseline for targeted regression coverage in the main config and lint branches, rather than adding an open-ended goal to raise the overall percentage.

Revision note: Removed `noncanonical-local-path` from the active plan and implementation after concluding that canonicalization adds style noise without enough value to justify a repository-level policy requirement.

Revision note: Removed the `relative-path-policy` configuration surface after deciding that link destinations should simply follow standard Markdown editor semantics instead of exposing a second repository policy knob.
