# Doc Gardener Linter

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Maintain this document in accordance with `docs/PLANS.md`. If that file exists in the target repository, re-read it before revising this plan and keep this plan aligned with its rules.

## Purpose / Big Picture

After this change, the repository will have a dedicated linter named `Doc Gardener Linter`, exposed as the executable `dglint`, that checks whether Markdown documentation refers to local repository files and directories using a repository-selected style policy and whether those references actually resolve to real paths in the working tree. A repository will be able to choose whether local file references should prefer compact backticked paths such as `docs/PLANS.md`, Markdown links such as `[docs/PLANS.md](docs/PLANS.md)`, or a narrower exception-based mix. The user-visible benefit is that teams can enforce one consistent convention, keep documentation mechanically valid, and still tune for either agent efficiency or human navigation.

You will know this is working when running `dglint lint .` against a repository reports real broken path references, reports style violations according to the configured local-reference policy, reports ambiguous or non-conforming file references, and exits successfully on a clean repository. The linter must be deterministic enough to run in local development and continuous integration, and durable enough to publish as a reusable open source command-line tool.

## Progress

- [x] (2026-03-06 18:20Z) Revise the ExecPlan so local file-reference style is configurable instead of implicitly hard-coded to backticks only.
- [x] (2026-03-06 18:40Z) Revise the ExecPlan so check-only mode explicitly reports when autofix is available and prints the corresponding autofix command plus a summary of fixable diagnostics.
- [ ] (2026-03-06 00:00Z) Scaffold the Rust crate for `Doc Gardener Linter` and expose a working `dglint` command-line executable without changing repository docs yet.
- [ ] (2026-03-06 00:00Z) Define the reference taxonomy: what counts as a file-path reference, what counts as a code-symbol reference, and what counts as an external navigation link.
- [ ] (2026-03-06 00:00Z) Implement Markdown parsing with `markdown-rs`, including extraction of inline code spans, fenced code blocks, Markdown links, and source positions from mdast nodes.
- [ ] (2026-03-06 00:00Z) Implement path classification rules for backticked text and repository-relative path resolution.
- [ ] (2026-03-06 00:00Z) Implement style-policy configuration so repositories can choose whether local references prefer backticks, links, or documented exceptions.
- [ ] (2026-03-06 00:00Z) Add ignore-pattern infrastructure for both whole-file exclusions and rule-specific exclusions, without hard-coding the final configuration syntax yet.
- [ ] (2026-03-06 00:00Z) Implement lint rules for local references that violate the configured style policy, unresolved local paths, and ambiguous inline path candidates.
- [ ] (2026-03-06 00:00Z) Add autofix support for the safe subset of violations, serializing edits back to Markdown using the Rust mdast serialization stack.
- [ ] (2026-03-06 00:00Z) Add fixture-based tests covering valid references, invalid references, edge cases, and false-positive protection.
- [ ] (2026-03-06 00:00Z) Wire `dglint` into repository scripts and continuous integration.
- [ ] (2026-03-06 00:00Z) Update repository documentation to explain the selected local-reference style policy and the limited exceptions that remain allowed.
- [ ] (2026-03-06 00:00Z) Capture final evidence, update this plan, and move it to `docs/exec-plans/completed/` when the linter is adopted.

## Surprises & Discoveries

- Observation: None yet.
  Evidence: Implementation has not started.

## Decision Log

- Decision: Support a configurable local-reference style policy, with backticks as the initial default and Markdown-link preference available through configuration.
  Rationale: The core product is mechanical validation of repository-local references, not a permanently hard-coded opinion about one representation. Backticks remain the best default for agent-oriented repositories, but the tool will be more reusable if repositories can opt into link-first behavior or narrow exception lists without forking rule logic.
  Date/Author: 2026-03-06 / Codex

- Decision: Use TOML as the configuration file format, while deferring the exact configuration key structure until the rule model and override ergonomics are clearer.
  Rationale: TOML matches Rust ecosystem expectations, is comfortable for hand-edited linter configuration, supports comments, and keeps room for readable nested settings and override blocks. Choosing the serialization format now reduces implementation ambiguity without prematurely locking in the user-facing config layout.
  Date/Author: 2026-03-06 / Codex

- Decision: Treat `dglint` as a two-mode CLI with explicit check-only and autofix user experience requirements.
  Rationale: The plan already required a non-mutating scan mode and a mutating autofix mode, but adoption will be materially better if check-only mode tells users when safe rewrites are available, how to invoke them, and which reported violations are fixable. Making that guidance part of the CLI contract prevents a weaker implementation that technically supports autofix but hides discoverability.
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

No implementation work has been completed yet. The intended outcome is a linter that turns a repository’s documentation conventions into enforceable rules, reducing stale docs while allowing each repository to choose whether compactness or navigational link structure should be the default representation for local references. The main risk is overfitting the classifier or writing style rules that are too aggressive and create noisy false positives. A secondary risk is overscoping the first Rust implementation before the configurable rule semantics are proven on real repositories.

## Context and Orientation

This plan assumes a repository that uses Markdown documentation heavily and wants a structured knowledge base where `AGENTS.md` acts as a small map and deeper guidance lives elsewhere under `docs/`. The problem to solve is not generic prose linting. The problem is documentation drift in path references: a Markdown file says `docs/PLANS.md` exists, or links to `docs/architecture.md`, but the path is stale, inconsistent with the repository’s chosen style policy, or represented in a way the repository has decided not to allow.

In this plan, a "repository-local path reference" means a textual mention of a file or directory that is expected to exist in the same repository as the linter is running against. A "local-reference style policy" means the repository rule that decides whether those references should normally appear as backticked repository-relative paths, as Markdown links, or as one representation with documented exceptions. A "Markdown link" means standard Markdown link syntax such as `[Plans](docs/PLANS.md)`. An "external link" means a destination outside the repository, such as an `https://` URL; these remain valid and should never be rewritten according to local-path style rules. An "ambiguous inline code span" means backticked text that could be a path or could be a code fragment, for example `build`, `Program`, or `foo.bar`, where the linter cannot confidently infer that the author meant a repository path.

The linter should operate on Markdown files, probably under `docs/`, `README.md`, `AGENTS.md`, and other configured documentation roots. It must parse Markdown structure rather than relying only on regular expressions because fenced code blocks, inline code spans, link destinations, autolinks, and escaped punctuation behave differently. It should ignore fenced code blocks by default because those blocks often contain examples that are not intended to be live repository references. It should inspect prose, headings, list items, tables if present, and inline code spans.

The implementation should be repository-agnostic. It belongs in its own repository or crate so other repositories can adopt it. That means configuration must be explicit. The linter should accept a repository root to scan, a set of Markdown globs to include or exclude, and a small TOML configuration file describing allowed path extensions, documentation roots, local-reference style policy, and exceptions.

The Rust implementation should build on `markdown-rs` to parse source Markdown into mdast nodes with source positions. For autofix support, the implementation should use the Rust mdast serialization stack to write edited mdast trees back to Markdown text. This keeps parsing and serialization aligned around mdast rather than mixing unrelated text-manipulation approaches.

## Plan of Work

Start by creating a standalone Rust crate for `Doc Gardener Linter` that builds a CLI executable named `dglint`. Use Cargo as the package manager and test runner. The crate should expose a small command-line interface with at least these modes: scan and report violations, scan with machine-readable output, and scan with autofix for safe rewrites. Treat the user-facing CLI contract as two primary human workflows: a check-only mode that scans and reports without modifying files, and an autofix mode that applies the safe deterministic rewrites. In check-only mode, every diagnostic that is safe to rewrite should be marked as fixable. If any fixable diagnostics are found, the CLI must print a short fix summary after the diagnostics, including how many violations are autofix candidates, which rule kinds are fixable in the current run, and the exact autofix command the user should run next for the same repository root, globs, and configuration file. The command-line interface should accept a repository root, optional file globs, and a configuration file path. It should exit with code `0` when no violations are found, `1` when violations are found, and a distinct non-zero code when configuration or parser errors prevent a complete run.

Implement parsing on top of a real Markdown abstract syntax tree rather than trying to lint with line-oriented regular expressions. Use `markdown-rs` to parse Markdown into mdast, and preserve source positions for inline code spans, link nodes, text nodes, and other relevant nodes. Source positions matter because the linter needs to print file, line, and column for each violation and because safe autofix requires precise replacements or stable AST-to-Markdown regeneration.

Define the reference taxonomy in code before implementing rules. A repository-local path reference must satisfy all of the following conditions unless a configuration override says otherwise. It must be repository-relative, not absolute. It must be path-shaped. "Path-shaped" means at least one of these is true: it contains a `/`; it starts with `./` or `../`; it ends with a known file extension such as `.md`, `.ts`, `.tsx`, `.js`, `.jsx`, `.json`, `.yml`, `.yaml`, `.cs`, `.csproj`, `.tf`, `.sh`, `.sql`, `.rs`, `.toml`; or it exactly matches a configured special-case filename such as `README.md`, `AGENTS.md`, `LICENSE`, `Makefile`, or `Cargo.toml`. If a backticked token does not satisfy these rules, it is not automatically considered a path and should not be resolved against the filesystem.

Implement path normalization next. Normalize candidate references to repository-relative paths from the repository root rather than resolving relative to the current Markdown file. This is a deliberate convention because repo-relative references are easier for both agents and linters to reason about consistently. The linter should flag relative same-directory references such as `./foo.md` or `../bar.md` unless the repository explicitly chooses to allow them. Normalize path separators, reject traversal that escapes the repository root, and resolve case sensitivity according to the running filesystem while warning when the literal spelling does not match the on-disk spelling on case-insensitive platforms. That warning prevents hidden drift that later breaks on Linux continuous integration.

With classification and normalization in place, implement configuration-backed style semantics before finalizing the core lint rules. The configuration should expose `local_reference_style` with at least `backticks` and `links` values, plus per-path exceptions for locations that intentionally diverge. Under `backticks`, the linter should prefer inline-code repository-relative paths for local references. Under `links`, the linter should prefer Markdown links for local references and treat bare backticked paths as style violations unless the configured exception policy allows them. Regardless of style, local references must still resolve to real repository paths.

Implement the core lint rules as style-aware rules. The first rule is `unresolved-local-path`: if either inline code or a Markdown link is classified as a repository-local path reference, the referenced file or directory must exist. The second rule is `prefer-backticks-for-local-paths`: when `local_reference_style` is `backticks`, a Markdown link that points to a repository-local path and does not add meaningful explanatory content beyond repeating the destination should be rewritten as a backticked repository-relative path. The third rule is `prefer-links-for-local-paths`: when `local_reference_style` is `links`, a bare backticked path reference in prose should be rewritten to a Markdown link form when the repository wants navigable links by default. The fourth rule is `ambiguous-inline-code`: if inline code looks path-adjacent but does not satisfy the path grammar clearly enough, report it only in strict mode or as an informational warning, not as a hard failure at first. The fifth rule is `noncanonical-local-path`: if a repository-local path is written in noncanonical form, such as `./docs/PLANS.md` when the convention is `docs/PLANS.md`, report it and offer autofix.

Be conservative about style-enforcement rules. A link like `[architecture guide](docs/architecture.md)` carries human-readable anchor text that might be valuable in prose. A backticked path like `docs/architecture.md` may also be intentional in a link-first repository if it appears inside a code-focused section or a configured exception file. The safe initial rule is narrower: autofix only links whose label is equivalent to the destination, differs only by formatting, or is a trivial filename echo, and only autofix backticks to links when the canonical link text can be generated mechanically without changing the sentence meaning. For all other local style mismatches, report with a message but do not autofix until the rule has proven low-noise in fixtures and real repository trials.

Implement configuration so repositories can tune both the grammar and the style policy without editing code. The configuration file format should be TOML. The configuration should support concepts for `include`, `exclude`, allowed path extensions, special-case filenames, documentation roots, relative-path policy, ignored paths, local-reference style selection, and style exceptions. The exact TOML key structure does not need to be finalized yet, but the runtime model must cover those concepts so the syntax can be refined without reworking the engine.

Build ignore-pattern infrastructure into the first configuration layer even if the final TOML structure is still under discussion. The implementation should support two concepts from the start. The first is a whole-file exclusion that removes matching Markdown files from all linting. The second is a rule-specific exclusion that still scans a file but suppresses named rules for matching paths. The plan should stay neutral about the final TOML key layout or override shape, but the runtime behavior should be part of the first implementation because repositories will need it immediately for files such as `AGENTS.md`, generated docs, or future plan directories.

After the core rules exist, add autofix only for deterministic rewrites. The safe autofix set is: replace local Markdown links whose label is equivalent to the destination with a backticked normalized repository-relative path when `local_reference_style` is `backticks`; replace local backticked paths with a canonical Markdown link when `local_reference_style` is `links` and the link text can be generated deterministically; normalize local path spelling to the canonical repository-relative form; and optionally remove leading `./` from otherwise valid paths. Do not autofix unresolved references, because the tool cannot know the author’s intended target. Do not autofix ambiguous inline code. Every autofix should preserve surrounding punctuation and whitespace. Where a rewrite changes Markdown structure rather than a plain text slice, prefer regenerating only the affected node or file through the mdast serialization path rather than piecemeal string surgery. The diagnostics model should therefore distinguish between fixable and non-fixable violations so check-only mode can accurately explain what autofix would change without mutating any files.

Testing must use fixtures rather than only unit-level parser mocks. Create a `fixtures/` area containing small synthetic repositories or document sets with known pass and fail expectations. Each fixture should include Markdown files, target files on disk, expected diagnostics, and expected autofix output where applicable. Include cases for valid backticked paths, valid link-style local references, valid external links, valid symbol references, invalid stale paths, same-name links rewritten to backticks, bare backticked paths rewritten to links in link-first mode, links with meaningful labels left untouched, code blocks ignored, Windows-style path separators rejected or normalized, and case-mismatch warnings. Add tests that assert line and column numbers, because diagnostics without stable positions are difficult to use in editors and continuous integration.

Once the linter is reliable in isolation, integrate it into the repository that adopts it. Add a command such as `cargo run -- lint .` during development or the installed CLI form `dglint lint .` in documentation and CI. Add continuous integration so every pull request validates documentation references. Update `AGENTS.md`, `README.md`, or the repository’s style guide to explain the selected convention in one short rule: repository-local file and directory references in prose should follow the configured local-reference style policy, and exceptions should be explicit rather than ad hoc.

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

5. Implement style-policy configuration, ignore-pattern infrastructure, and lint rules.

    Working directory: crate root

    Example commands:

        cargo test config
        cargo test ignores
        cargo test rules

    Expected outcome: Fixture tests prove unresolved paths, style-policy violations, ignore-pattern behavior, and noncanonical local paths are reported with file, line, column, rule id, and message.

6. Implement autofix for the safe subset.

    Working directory: crate root

    Example command:

        cargo test fix

    Expected transcript excerpt:

        running 3 tests
        test fix::rewrites_identical_local_markdown_links_to_backticked_paths ... ok
        test fix::rewrites_backticked_paths_to_links_in_link_mode ... ok
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

    Expected outcome: Contributors can read one concise rule and examples that match the repository’s chosen `dglint` policy.

## Validation and Acceptance

Validation is complete only when all of the following are true.

Running the linter against fixture repositories proves correct behavior for both passing and failing cases. At minimum, there must be fixtures covering valid backticked file paths, valid link-style local references, broken local paths in both representations, local Markdown links that should be rewritten to backticks in backtick mode, backticked paths that should be rewritten to links in link mode, local Markdown links that should remain links because their label adds meaning, code spans that are symbols rather than paths, commands that are not paths, code blocks that are ignored, and path normalization edge cases.

Running the linter against the repository itself must demonstrate at least one of these observable outcomes. Either it finds real documentation drift that you then fix, or it passes cleanly after confirming existing docs already satisfy the convention. In both cases, the command and result must be recorded in this plan’s `Artifacts and Notes` section.

Acceptance should be phrased in behavior, not implementation details:

- When a Markdown file contains `docs/PLANS.md` in backticks, the file exists, and `local_reference_style` is `backticks`, `dglint` emits no error.
- When a Markdown file contains `[docs/PLANS.md](docs/PLANS.md)`, the file exists, and `local_reference_style` is `links`, `dglint` emits no error.
- When a Markdown file contains `docs/DOES-NOT-EXIST.md` in backticks or `[docs/DOES-NOT-EXIST.md](docs/DOES-NOT-EXIST.md)`, `dglint` reports `unresolved-local-path` with file, line, and column.
- When a Markdown file contains `[docs/PLANS.md](docs/PLANS.md)` and `local_reference_style` is `backticks`, `dglint` reports `prefer-backticks-for-local-paths` and autofix rewrites it to `docs/PLANS.md`.
- When a Markdown file contains `docs/PLANS.md` in backticks and `local_reference_style` is `links`, `dglint` reports `prefer-links-for-local-paths` and autofix rewrites it to `[docs/PLANS.md](docs/PLANS.md)` or another configured canonical label form.
- When a Markdown file contains `[planning guide](docs/PLANS.md)`, `dglint` either leaves it untouched under the repository’s exception policy or reports it without autofix, depending on configured strictness.
- When a Markdown file contains `Program` or `npm test`, `dglint` does not incorrectly report these as missing files.
- When `dglint` runs in check-only mode and finds one or more fixable violations, it prints a post-diagnostic summary that states how many violations are fixable, identifies the fixable rule kinds found in that run, and prints the exact autofix command for the same invocation shape, including `--config` or path/glob arguments when present.
- When `dglint` runs in check-only mode and finds only non-fixable violations such as `unresolved-local-path`, it does not advertise autofix for those violations.
- When the repository’s continuous integration runs `dglint` on a pull request, a newly introduced broken repository-local path causes the job to fail.

## Idempotence and Recovery

The linter must be safe to run repeatedly. Check-only mode must never modify files. Autofix mode must only apply deterministic rewrites and must produce identical output on subsequent runs once the first fix has been applied. If an autofix attempt cannot be applied cleanly because the source document changed after parsing, the linter should fail that file with a clear message rather than partially rewriting content. The check-only summary that advertises autofix must also be deterministic: rerunning the same command on the same tree should produce the same fixable-count summary and the same suggested autofix command text.

If rule tuning creates too many false positives during rollout, lower-risk recovery is to keep `unresolved-local-path` as an error and downgrade `ambiguous-inline-code`, `prefer-backticks-for-local-paths`, and `prefer-links-for-local-paths` to warnings temporarily. Another safe rollout option is to scope enforcement to a subset of files such as `AGENTS.md`, `README.md`, and `docs/**/*.md`, then expand once the rule quality is proven.

If a repository has many existing local Markdown links, enable autofix in a dedicated cleanup pull request before making the continuous integration job blocking. That keeps adoption incremental and reduces friction.

## Artifacts and Notes

Record concise evidence here as implementation proceeds. Replace the placeholders below with real transcripts and examples.

Expected diagnostic style example:

    docs/repository-knowledge/repository-knowledge-system.md:42:17  error  unresolved-local-path
    Local repository path `docs/architecture.md` does not exist from repository root.

Expected link-mode diagnostic example:

    docs/repository-knowledge/repository-knowledge-system.md:42:17  error  prefer-links-for-local-paths
    Local repository path `docs/architecture.md` should use Markdown link syntax under the configured style policy.

Expected check-only autofix hint example:

    docs/repository-knowledge/repository-knowledge-system.md:42:17  error  prefer-backticks-for-local-paths  fixable
    Local repository link `[docs/architecture.md](docs/architecture.md)` should use backticks under the configured style policy.

    1 fixable violation found across 1 file.
    Fixable rules in this run: prefer-backticks-for-local-paths
    Run `dglint fix . --config dglint.toml` to apply safe rewrites.

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
