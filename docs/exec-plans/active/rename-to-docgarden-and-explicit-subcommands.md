# Rename `dglint` to `docgarden` and Restore Explicit Subcommands

Save this in-progress ExecPlan at `docs/exec-plans/active/rename-to-docgarden-and-explicit-subcommands.md`. Move it into the completed ExecPlan directory when the work is complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

After this change, the tool will present itself as `docgarden` instead of `dglint`, and its command surface will use explicit subcommands rather than the current implicit “lint by default, fix with `--fix`” model. A user will be able to run commands such as `docgarden lint .`, `docgarden fix README.md`, `docgarden init`, and `docgarden skill init` without the parser ambiguity that comes from mixing top-level positional targets with optional subcommands.

This change matters because the product is no longer just a narrow linter. The repository direction now includes repository bootstrap and skill scaffolding, so the executable name should describe the broader system and the command tree should scale cleanly as more setup-oriented workflows are added. You will know the work is complete when the binary name, help output, installation instructions, tests, and repository documentation all consistently describe `docgarden`, and when the explicit subcommands behave correctly in both local development and CI-style runs.

## Progress

- [x] (2026-03-21 00:00Z) Authored the initial ExecPlan in `docs/exec-plans/active/rename-to-docgarden-and-explicit-subcommands.md`.
- [ ] Add failing CLI tests that describe the new `docgarden lint` and `docgarden fix` command surface and lock in the intended help output.
- [ ] Refactor the CLI parser in `src/cli.rs` to use required explicit subcommands and route lint and fix through shared execution logic.
- [ ] Rename the package, executable, and configuration filename from `dglint` to `docgarden`.
- [ ] Update repository documentation, examples, CI guidance, and plan references so the checked-in docs describe `docgarden` and the explicit subcommand model consistently.
- [ ] Run the Rust validation stack and dogfood the updated documentation with the renamed binary, then record outcomes in this plan.

## Surprises & Discoveries

- Observation: The current CLI is intentionally small and flat. `src/cli.rs` derives one top-level parser with positional `targets` plus flags such as `--fix`, and `src/main.rs` simply calls `dglint::run()`.
  Evidence: `src/cli.rs` defines `Args` with `targets: Vec<PathBuf>` and `fix: bool`; `src/main.rs` contains only the call to `dglint::run()`.

- Observation: The repository already recorded and implemented the opposite command-shape decision once before: an older `lint`/`fix` subcommand model was replaced with the current default-lint plus `--fix` flow.
  Evidence: `docs/exec-plans/completed/doc-gardening-linter.md` contains the revision note stating that the earlier `lint` and `fix` subcommands were replaced by the more standard `dglint` and `dglint --fix` contract.

- Observation: The current name is spread through more than the parser. It appears in Cargo package metadata, config-file discovery, tests that reference `env!("CARGO_BIN_EXE_dglint")`, user-facing fix hints, README examples, and completed plan history.
  Evidence: `Cargo.toml`, `src/config.rs`, `src/cli.rs`, `src/main.rs`, `tests/cli.rs`, `tests/config.rs`, `tests/path_behavior.rs`, and multiple files under `docs/` all contain `dglint` or `dglint.toml`.

## Decision Log

- Decision: Rename the human-facing tool and primary executable to `docgarden`.
  Rationale: The product scope now includes repository bootstrap and skill scaffolding in addition to linting. `docgarden` better captures the full repository-knowledge workflow than a name that reads as “Doc Gardening Linter” only.
  Date/Author: 2026-03-21 / Codex

- Decision: Use explicit required subcommands for the main command tree rather than an optional-subcommand parser with default lint behavior.
  Rationale: The repository has already experienced friction with `clap` parsers that mix top-level positional targets and optional subcommands. A required subcommand enum keeps the parser simpler, removes target-versus-subcommand ambiguity, and leaves a clean place to add `init` and `skill init`.
  Date/Author: 2026-03-21 / Codex

- Decision: Restore separate `lint` and `fix` subcommands instead of keeping `--fix` as the only autofix entrypoint.
  Rationale: With explicit subcommands, `lint` and `fix` are easier to reason about than mixing repository-setup commands with one lone mode flag. They also make command examples and help output more uniform across the CLI.
  Date/Author: 2026-03-21 / Codex

- Decision: Rename the configuration file to the new `docgarden`-branded filename without keeping dual discovery for `dglint.toml`.
  Rationale: The repository is still private and the tool has not been deployed yet, so a pre-release rename can be done as one clean cut without carrying compatibility logic that will immediately become maintenance burden.
  Date/Author: 2026-03-21 / Codex

- Decision: Do not preserve a `dglint` compatibility alias in the command surface or config discovery.
  Rationale: Because the rename happens before public rollout, the simplest and clearest implementation is to converge immediately on one name everywhere: crate metadata, binary help text, fix hints, tests, docs, and config discovery.
  Date/Author: 2026-03-21 / Codex

## Outcomes & Retrospective

No implementation work has been completed yet. The expected outcome is a renamed tool whose command surface scales cleanly to linting, fixing, repository initialization, and skill scaffolding, while remaining deterministic and CI-friendly. The largest risk is inconsistency: the code can compile while tests, docs, config discovery, and installation instructions still point at the old name. This plan exists to keep those surfaces synchronized during one pre-release cutover.

## Context and Orientation

This repository currently builds a Rust command-line program named `dglint`. The Cargo package is declared in `Cargo.toml`. The executable entry point is `src/main.rs`, which calls `dglint::run()`. The parser and dispatch logic live in `src/cli.rs`. The parser is a single `clap` derive struct with positional lint targets, plus flags including `--config`, `--json`, `--fix`, `--no-gitignore`, and `--color`. The actual linting work is delegated from `src/cli.rs` into shared helpers such as `crate::discover::discover_markdown_files_for_targets`, `crate::lint::lint_file`, and `crate::lint::summarize`.

Configuration currently lives in a dedicated repository-root file named `dglint.toml`. `src/config.rs` discovers that file when no explicit `--config` path is provided. Repository-root inference in `src/cli.rs` and `src/root.rs` also treats `dglint.toml` as a marker. The test suite uses `assert_cmd` in `tests/cli.rs`, `tests/config.rs`, and `tests/path_behavior.rs` to execute the compiled binary by its current Cargo-generated name `CARGO_BIN_EXE_dglint`.

The README and product docs still describe the tool as `dglint`, a deterministic linter that powers a broader Doc Gardener workflow. The requested product shift is to make the binary itself represent the broader system under the name `docgarden`, with linting as one explicit subcommand alongside `init` and `skill init`. That means this change is not only a parser refactor. It is a coordinated product rename, command-surface migration, and documentation update.

The term “explicit subcommand model” in this plan means the root parser requires one named action such as `lint`, `fix`, `init`, or `skill`. In plain language, users no longer type the executable followed by bare paths. They choose an action first.

## Plan of Work

Start with tests so the new command surface is specified before the parser changes. In `tests/cli.rs`, replace or supplement the current assertions that invoke the binary with bare targets or `--fix`. Add focused tests that prove `docgarden lint <targets>` behaves like the current check mode and `docgarden fix <targets>` behaves like the current autofix mode. Include one help-output test that asserts the root help shows the explicit subcommands, and that `lint --help` and `fix --help` expose the lint-target arguments and shared flags the user needs. Because the Cargo-generated binary name will change as part of this work, update test invocations from `env!("CARGO_BIN_EXE_dglint")` to the new executable name once the package metadata changes. The tests should fail before the CLI implementation is updated.

After the tests describe the new behavior, refactor `src/cli.rs` into a root parser plus a subcommand enum. The root parser should carry only global options that truly apply across commands. The `lint` and `fix` subcommands should share one reusable argument struct for targets, config, JSON output, gitignore behavior, and color where appropriate. Preserve the current lint execution pipeline by routing both subcommands into a shared function that still performs repository-root inference, config loading, file discovery, lint traversal, diagnostic printing, and exit-status decisions. `fix` should still use `crate::lint::Mode::Fix`, while `lint` should still use `crate::lint::Mode::Check`. The goal is to change the command surface without forking the underlying lint logic.

Once the command tree is in place, perform the rename. Update `Cargo.toml` so the published package and binary expose `docgarden`. Update `src/main.rs` and the crate exports so the main function calls the renamed crate module path correctly. Change `#[command(name = "...")]` in `src/cli.rs` to `docgarden`, and rewrite any user-facing fix hints so they print `docgarden fix ...` rather than `dglint ... --fix`. Rename the canonical config filename to the new `docgarden`-branded filename in `src/config.rs`, `src/cli.rs`, and `src/root.rs`, and update every test and fixture string that mentions the old binary or config name, including README text embedded in tests.

Do the config rename as a full cutover rather than a compatibility layer. `src/config.rs` should discover the new config filename when no explicit `--config` path is provided, and `src/cli.rs` plus `src/root.rs` should use that new filename as the repository-root marker. Keep explicit `--config` behavior unchanged except for examples that should now show the new filename. Because the tool is still private, there is no need to preserve dual discovery or precedence rules.

After the binary and config rename are functional, update documentation and examples. The highest-priority files are `README.md`, `docs/PRODUCT.md`, `ARCHITECTURE.md`, `AGENTS.md`, and any active or completed ExecPlans whose instructions are intended to remain operational for future contributors. Update the README usage section so the examples become `docgarden lint .`, `docgarden fix .`, and later `docgarden init` / `docgarden skill init` where relevant. In architecture and product docs, rewrite the narrative to describe `docgarden` as the primary tool and describe linting as one subcommand family. When touching historical completed plans, preserve history rather than pretending the old name never existed; if a historical reference remains intentionally historical, label it as such in prose instead of silently rewriting implementation history.

Finally, run the required validation stack from the repository root. Because the repository policy says to use TDD for bug reports and review findings and to run `cargo xtask validate`, use the full validation command after the focused CLI tests pass. Then dogfood the renamed tool against the changed documentation so the repository proves it can lint its own newly renamed docs and config references.

## Concrete Steps

Work from the repository root.

1. Add the new CLI contract tests first.

    cargo test cli::lint
    cargo test cli::fix

Expected result before implementation: the new tests fail because the binary name, parser shape, or help output still matches the old `dglint` plus `--fix` contract.

2. Refactor the parser and dispatch logic.

    cargo fmt
    cargo check

Expected result: the crate builds with a required subcommand parser, and the focused CLI tests begin to pass once dispatch is correctly wired to the existing lint engine.

3. Rename the package, executable, and configuration file.

    cargo test

Expected result: tests that previously referenced `CARGO_BIN_EXE_dglint` now use the renamed binary, and config-loading tests prove that the new config filename is the only implicitly discovered config filename.

4. Update repository documentation and examples.

    cargo run -- lint README.md AGENTS.md ARCHITECTURE.md docs/PRODUCT.md docs/exec-plans/active/rename-to-docgarden-and-explicit-subcommands.md --color never

Expected result: the updated docs lint cleanly and no longer contain accidental stale references that should have been migrated.

5. Run the repository validation stack.

    cargo xtask validate

Expected result: formatting, linting, tests, and any repository-level validation performed by `xtask` all succeed with the renamed command surface.

6. Dogfood the new executable shape directly.

    cargo run -- lint .
    cargo run -- fix . --color never

Expected result: `cargo run -- lint .` behaves like the former default check mode, and `cargo run -- fix . --color never` behaves like the former `--fix` mode without parser ambiguity.

## Validation and Acceptance

Acceptance is behavioral and must be observable from the command line and from checked-in documentation.

First, the root help output must show explicit subcommands rather than bare positional lint targets. A user running the renamed binary’s help should be able to discover `lint`, `fix`, `init`, and `skill` from one screen without reading the source code. The `lint` and `fix` help pages must each explain targets, config selection, color control, JSON output where applicable, and gitignore behavior.

Second, end-to-end lint and fix behavior must remain intact under the new command names. A fixture repository that currently fails under lint mode must still fail when invoked as `docgarden lint <target>`, and the same fixture must become clean after running `docgarden fix <target>` when the violations are mechanically fixable. A second lint pass after fix must succeed just as it does today.

Third, configuration discovery must be internally consistent after the rename. A temporary repository containing the new config filename must lint successfully without an explicit `--config` flag, and tests must prove that repository-root inference also uses that renamed file as the marker. Any tests or fixtures that still rely on `dglint.toml` should be renamed as part of the cutover.

Fourth, the repository documentation must be self-consistent. The README installation command, usage examples, fix hints, architecture description, and active plan added here must all use `docgarden` and the explicit subcommand model where they are intended as live operational guidance. Historical completed-plan notes may mention `dglint` when they are clearly framed as history.

## Idempotence and Recovery

This migration should be safe to repeat in small slices. Parser refactors, test updates, and documentation rewrites are additive and can be rerun with `cargo fmt`, `cargo test`, and `cargo xtask validate` until clean. Renaming strings in documentation is also repeatable as long as historical references are reviewed intentionally rather than mass-replaced blindly.

The riskiest part is the binary and config rename because it touches many surfaces at once. If the repository reaches a state where the new binary name compiles but tests or docs still mention the old one, use the test suite and targeted ripgrep searches to finish the migration before attempting any broader cleanup. If config discovery breaks during implementation, the safe fallback is to pause and complete the repository-wide rename in one pass rather than introducing temporary compatibility branches that the project does not need.

## Artifacts and Notes

Expected root help shape after implementation:

    $ docgarden --help
    Usage: docgarden <COMMAND>

    Commands:
      lint    Lint repository knowledge without modifying files
      fix     Apply deterministic safe rewrites
      init    Initialize repository-local docgarden configuration
      skill   Manage repository skill scaffolding

Expected fix hint shape after implementation:

    2 fixable violations found.
    Fixable rules in this run: prefer-backticks-for-local-paths
    Run `docgarden fix docs/guide.md` to apply fixes.

Expected config layout after implementation:

    repository-root/
      docgarden.toml
      README.md

In this case, `docgarden lint README.md` must discover the renamed config file automatically with no legacy fallback behavior.

## Interfaces and Dependencies

Keep the implementation inside the current Rust workspace. Continue using `clap` derive parsing in `src/cli.rs`; do not replace the CLI stack with a different parser. The root parser should expose a stable subcommand enum, and the lint-oriented subcommands should share an argument struct rather than duplicating target and config fields.

One acceptable parser shape is:

    #[derive(Parser, Debug)]
    #[command(name = "docgarden")]
    pub struct Args {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Subcommand, Debug)]
    enum Command {
        Lint(LintArgs),
        Fix(LintArgs),
        Init(InitArgs),
        Skill(SkillCommand),
    }

    #[derive(Args, Debug)]
    struct LintArgs {
        #[arg(default_value = ".", num_args = 0..)]
        targets: Vec<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        no_gitignore: bool,
        #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
        color: ColorChoice,
    }

The exact type names may differ, but the final design must preserve one shared execution path for lint-style commands and must not reintroduce optional-subcommand ambiguity at the root parser.

For the config rename, the implementation must continue to use `crate::config::Config::load` and `crate::root::infer_repository_root`, updated so the canonical discovered filename is the new `docgarden`-branded config file. Tests must exercise the renamed discovery path directly.

Revision note: Created this plan to capture the decision to rename the tool to `docgarden`, restore explicit subcommands, and implement the migration in a way that keeps the parser, config discovery, tests, and docs aligned.
