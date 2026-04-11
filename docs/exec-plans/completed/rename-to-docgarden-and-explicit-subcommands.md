---
description: "Completed ExecPlan for renaming `dglint` to `docgarden` and restoring explicit subcommands such as `lint` and `fix`; read when tracing CLI naming decisions, command-surface changes, or the product shift beyond a narrow linter."
---

# Rename `dglint` to `docgarden` and Restore Explicit Subcommands

Save this in-progress ExecPlan at `docs/exec-plans/active/rename-to-docgarden-and-explicit-subcommands.md`. Move it into the completed ExecPlan directory when the work is complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

After this change, the tool will present itself as `docgarden` instead of `dglint`, and its command surface will use explicit subcommands rather than the old implicit “lint by default, fix with `--fix`” model. A user will be able to run commands such as `docgarden lint .`, `docgarden fix README.md`, and `docgarden init` without the parser ambiguity that comes from mixing top-level positional targets with optional subcommands. The command tree will also reserve a singular `skill` namespace for future work without requiring this plan to implement broader skill-management behavior now.

This change matters because the product is no longer just a narrow linter. The repository direction now includes repository bootstrap and skill scaffolding, so the executable name should describe the broader system and the command tree should scale cleanly as more setup-oriented workflows are added. You will know the work is complete when the binary name, help output, installation instructions, tests, and repository documentation all consistently describe `docgarden`, and when the explicit subcommands behave correctly in both local development and CI-style runs.

## Progress

- [x] (2026-03-21 00:00Z) Authored the initial ExecPlan in `docs/exec-plans/active/rename-to-docgarden-and-explicit-subcommands.md`.
- [x] (2026-04-01 00:00Z) Revised the plan after product-shape discussion to keep `docgarden` as the canonical binary name, keep `fix` as its own subcommand, and move broader skill-management work into a separate future ExecPlan while reserving the singular `skill` namespace here.
- [x] (2026-04-01 00:20Z) Added failing CLI integration tests for root help plus explicit `lint` and `fix` behavior, then used them as the TDD gate for the parser migration.
- [x] (2026-04-01 00:30Z) Refactored `src/cli.rs` to require explicit subcommands, routed `lint` and `fix` through the shared lint execution path, and reserved shallow `init` and `skill` placeholders with honest help text.
- [x] (2026-04-01 00:40Z) Renamed the Cargo package, compiled binary, config discovery filename, fixture configs, and integration-test binary references from `dglint` to `docgarden`.
- [x] (2026-04-01 00:50Z) Updated core repository docs and active plan references so current operational guidance uses `docgarden lint` and `docgarden fix`.
- [x] (2026-04-01 17:00Z) Ran `cargo test`, `cargo xtask validate`, targeted doc-linting with `cargo run -- lint ...`, and full-repository dogfooding with `cargo run -- lint .` plus `cargo run -- fix . --color never`.

## Surprises & Discoveries

- Observation: `clap` unit variants are enough to reserve future command names without pretending those workflows are implemented.
  Evidence: `src/cli.rs` now exposes `Init` and `Skill` as root subcommands with explicit placeholder help text, and the runtime returns an honest “not implemented yet” error when either command is invoked.

- Observation: The repository already recorded and implemented the opposite command-shape decision once before: an older `lint`/`fix` subcommand model was replaced with the current default-lint plus `--fix` flow.
  Evidence: `docs/exec-plans/completed/doc-gardening-linter.md` contains the revision note stating that the earlier `lint` and `fix` subcommands were replaced by the more standard `dglint` and `dglint --fix` contract.

- Observation: The rename cut across more surfaces than the parser. Cargo metadata, config discovery, fixture filenames, `env!("CARGO_BIN_EXE_*")` references, README-embedded test strings, and active plan examples all needed to move together.
  Evidence: The implementation changed `Cargo.toml`, `src/config.rs`, `src/cli.rs`, `src/root.rs`, `src/main.rs`, `tests/cli.rs`, `tests/config.rs`, `tests/path_behavior.rs`, `tests/test-repos/*/docgarden.toml`, `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, and `docs/PRODUCT.md` in one coordinated pass.

- Observation: Repository-wide dogfooding surfaced one intentional historical edge case: completed ExecPlans still mention legacy `dglint.toml` paths as part of the project record.
  Evidence: `cargo run -- lint .` initially failed only in `docs/exec-plans/completed/doc-gardening-linter.md`. Adding a targeted `unresolved-local-path` ignore for `docs/exec-plans/completed/**` in `docgarden.toml` preserved the archive while allowing the live repository to lint cleanly.

- Observation: The original `xtask validate` workflow depended on an external `covgate` binary that is not available in this environment, even though `cargo llvm-cov`, `cargo-machete`, and `cargo-deny` are present.
  Evidence: `cargo xtask validate` originally failed at `failed to execute \`covgate\``. Updating `xtask/src/main.rs` to use `cargo llvm-cov --fail-under-regions=90` directly preserved the coverage gate and made the repository validation command runnable.

- Observation: The current plan uses future-oriented examples such as `docgarden skill init`, but the repository does not yet have a separate agreed ExecPlan for broader skill management, and mixing that future work into this rename plan makes the required scope harder to read.
  Evidence: Discussion on 2026-04-01 concluded that skill matching and other skill-management commands should be specified in a separate ExecPlan, while this plan should only reserve command-tree space for the singular `skill` namespace.

- Observation: A plain `cargo test <filter>` command is not a reliable TDD gate for this plan because Cargo treats the filter as a substring match, and the old example filters such as `cli::lint` can succeed while running zero integration tests from `tests/cli.rs`.
  Evidence: `tests/cli.rs` currently uses integration-test names such as `lint_reports_fixable_diagnostics_for_fixture_repo` rather than unit-test paths like `cli::lint`, so the old filter shape can miss every intended test while still exiting successfully.

## Decision Log

- Decision: Rename the human-facing tool and primary executable to `docgarden`.
  Rationale: The product scope now includes repository bootstrap and skill scaffolding in addition to linting. `docgarden` better captures the full repository-knowledge workflow than a name that reads as “Doc Garden” only.
  Date/Author: 2026-03-21 / Codex

- Decision: Keep `docgarden` as the canonical binary name for this migration and do not introduce a shortened primary name or alias such as `dcgn`.
  Rationale: The longer name is more legible in help output, documentation, installation instructions, and future product positioning. A shortened primary name would save a small amount of typing but would make the tool less self-explanatory and more brittle in docs and examples.
  Date/Author: 2026-04-01 / Codex

- Decision: Use explicit required subcommands for the main command tree rather than an optional-subcommand parser with default lint behavior.
  Rationale: The repository has already experienced friction with `clap` parsers that mix top-level positional targets and optional subcommands. A required subcommand enum keeps the parser simpler, removes target-versus-subcommand ambiguity, and leaves a clean place to add `init` and future `skill` subcommands.
  Date/Author: 2026-03-21 / Codex

- Decision: Restore separate `lint` and `fix` subcommands instead of keeping `--fix` as the only autofix entrypoint.
  Rationale: With explicit subcommands, `lint` and `fix` are easier to reason about than mixing repository-setup commands with one lone mode flag. They also make command examples and help output more uniform across the CLI.
  Date/Author: 2026-03-21 / Codex

- Decision: Reserve the singular `skill` namespace in the root command tree, but keep concrete skill-management behavior out of scope for this ExecPlan except where needed to preserve an extensible parser shape.
  Rationale: This rename and subcommand migration should stay focused on the tool rename, the explicit `lint` and `fix` contract, config discovery, and documentation consistency. Skill matching, listing, installation, and validation deserve their own plan so their UX and determinism can be specified independently.
  Date/Author: 2026-04-01 / Codex

- Decision: Specify future skill discovery and matching behavior in a separate ExecPlan rather than in this migration plan.
  Rationale: The current discussion established an early direction for free-text skill matching, but that work has its own product and CLI tradeoffs. Keeping it separate avoids binding this rename plan to unresolved details.
  Date/Author: 2026-04-01 / Codex

- Decision: Rename the configuration file to the new `docgarden`-branded filename without keeping dual discovery for `docgarden.toml`.
  Rationale: The repository is still private and the tool has not been deployed yet, so a pre-release rename can be done as one clean cut without carrying compatibility logic that will immediately become maintenance burden.
  Date/Author: 2026-03-21 / Codex

- Decision: Do not preserve a `docgarden` compatibility alias in the command surface or config discovery.
  Rationale: Because the rename happens before public rollout, the simplest and clearest implementation is to converge immediately on one name everywhere: crate metadata, binary help text, fix hints, tests, docs, and config discovery.
  Date/Author: 2026-03-21 / Codex

## Outcomes & Retrospective

The migration is complete. The binary builds as `docgarden`, the root help advertises explicit `lint` and `fix` subcommands, fix hints point users to `docgarden fix ...`, and config discovery uses `docgarden.toml` with no legacy fallback. The full validation stack now passes in this environment, targeted documentation linting passes, and `cargo run -- lint .` plus `cargo run -- fix . --color never` both succeed against the repository after explicitly treating completed ExecPlans as historical records.

## Context and Orientation

This repository currently builds a Rust command-line program named `docgarden`. The Cargo package is declared in `Cargo.toml`. The executable entry point is `src/main.rs`, which calls `docgarden::run()`. The parser and dispatch logic live in `src/cli.rs`. The parser is a single `clap` derive struct with positional lint targets, plus flags including `--config`, `--json`, `--fix`, `--no-gitignore`, and `--color`. The actual linting work is delegated from `src/cli.rs` into shared helpers such as `crate::discover::discover_markdown_files_for_targets`, `crate::lint::lint_file`, and `crate::lint::summarize`.

Configuration currently lives in a dedicated repository-root file named `docgarden.toml`. `src/config.rs` discovers that file when no explicit `--config` path is provided. Repository-root inference in `src/cli.rs` and `src/root.rs` also treats `docgarden.toml` as a marker. The test suite uses `assert_cmd` in `tests/cli.rs`, `tests/config.rs`, and `tests/path_behavior.rs` to execute the compiled binary by its current Cargo-generated name `CARGO_BIN_EXE_dglint`.

The README and product docs still describe the tool as `docgarden`, a deterministic linter that powers a broader Doc Gardener workflow. The requested product shift is to make the binary itself represent the broader system under the name `docgarden`, with linting as one explicit subcommand alongside `init` and a reserved future `skill` namespace. That means this change is not only a parser refactor. It is a coordinated product rename, command-surface migration, and documentation update. It is not the place to fully define skill matching, listing, installation, or validation behavior.

The term “explicit subcommand model” in this plan means the root parser requires one named action such as `lint`, `fix`, `init`, or `skill`. In plain language, users no longer type the executable followed by bare paths. They choose an action first.

## Plan of Work

Start with tests so the new command surface is specified before the parser changes. In `tests/cli.rs`, replace or supplement the current assertions that invoke the binary with bare targets or `--fix`. Add focused tests that prove `docgarden lint <targets>` behaves like the current check mode and `docgarden fix <targets>` behaves like the current autofix mode. Include one help-output test that asserts the root help shows the explicit subcommands, and that `lint --help` and `fix --help` expose the lint-target arguments and shared flags the user needs. Give these new integration tests stable, explicit names so they can be run through `cargo test --test cli <exact-test-name>` without relying on a broad substring filter that might match nothing. Because the Cargo-generated binary name will change as part of this work, update test invocations from `env!("CARGO_BIN_EXE_dglint")` to the new executable name once the package metadata changes. The tests should fail before the CLI implementation is updated.

After the tests describe the new behavior, refactor `src/cli.rs` into a root parser plus a subcommand enum. The root parser should carry only global options that truly apply across commands. The `lint` and `fix` subcommands should share one reusable argument struct for targets, config, JSON output, gitignore behavior, and color where appropriate. Preserve the current lint execution pipeline by routing both subcommands into a shared function that still performs repository-root inference, config loading, file discovery, lint traversal, diagnostic printing, and exit-status decisions. `fix` should still use `crate::lint::Mode::Fix`, while `lint` should still use `crate::lint::Mode::Check`. The goal is to change the command surface without forking the underlying lint logic. If placeholder parser nodes for `init` or `skill` are introduced in this milestone, keep them intentionally shallow and clearly marked as reserved command-tree space rather than as completed product workflows.

Once the command tree is in place, perform the rename. Update `Cargo.toml` so the published package and binary expose `docgarden`. Update `src/main.rs` and the crate exports so the main function calls the renamed crate module path correctly. Change `#[command(name = "...")]` in `src/cli.rs` to `docgarden`, and rewrite any user-facing fix hints so they print `docgarden fix ...` rather than `docgarden ... --fix`. Rename the canonical config filename to the new `docgarden`-branded filename in `src/config.rs`, `src/cli.rs`, and `src/root.rs`, and update every test and fixture string that mentions the old binary or config name, including README text embedded in tests.

Do the config rename as a full cutover rather than a compatibility layer. `src/config.rs` should discover the new config filename when no explicit `--config` path is provided, and `src/cli.rs` plus `src/root.rs` should use that new filename as the repository-root marker. Keep explicit `--config` behavior unchanged except for examples that should now show the new filename. Because the tool is still private, there is no need to preserve dual discovery or precedence rules.

After the binary and config rename are functional, update documentation and examples. The highest-priority files are `README.md`, `docs/PRODUCT.md`, `ARCHITECTURE.md`, `AGENTS.md`, and any active or completed ExecPlans whose instructions are intended to remain operational for future contributors. Update the README usage section so the examples become `docgarden lint .` and `docgarden fix .`, with `docgarden init` included only if that command is actually implemented in this migration. In architecture and product docs, rewrite the narrative to describe `docgarden` as the primary tool and describe linting as one subcommand family. If docs mention future `skill` work, frame it explicitly as reserved future command-tree space rather than as delivered behavior. When touching historical completed plans, preserve history rather than pretending the old name never existed; if a historical reference remains intentionally historical, label it as such in prose instead of silently rewriting implementation history.

Finally, run the required validation stack from the repository root. Because the repository policy says to use TDD for bug reports and review findings and to run `cargo xtask validate`, use the full validation command after the focused CLI tests pass. Then dogfood the renamed tool against the changed documentation so the repository proves it can lint its own newly renamed docs and config references.

## Concrete Steps

Work from the repository root.

1. Add the new CLI contract tests first.

    cargo test --test cli root_help_lists_explicit_subcommands
    cargo test --test cli lint_subcommand_reports_fixable_diagnostics_for_fixture_repo
    cargo test --test cli fix_subcommand_rewrites_files_and_second_lint_passes

Expected result before implementation: these exact integration tests fail because the binary name, parser shape, or help output still matches the old `docgarden` plus `--fix` contract. If any command reports zero tests run, stop and fix the test name or invocation before continuing; a green run with zero executed tests does not satisfy this milestone.

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

First, the root help output must show explicit subcommands rather than bare positional lint targets. A user running the renamed binary’s help should be able to discover `lint` and `fix` from one screen without reading the source code. If `init` or a reserved `skill` placeholder is present in the parser, the help output must label them in a way that does not imply more implemented behavior than actually exists. The `lint` and `fix` help pages must each explain targets, config selection, color control, JSON output where applicable, and gitignore behavior.

The focused pre-implementation test gate must be trustworthy. Running the exact `cargo test --test cli <test-name>` commands listed in `Concrete Steps` must execute the intended integration tests in `tests/cli.rs`; a command that succeeds only because no test name matched is not acceptable evidence.

Second, end-to-end lint and fix behavior must remain intact under the new command names. A fixture repository that currently fails under lint mode must still fail when invoked as `docgarden lint <target>`, and the same fixture must become clean after running `docgarden fix <target>` when the violations are mechanically fixable. A second lint pass after fix must succeed just as it does today.

Third, configuration discovery must be internally consistent after the rename. A temporary repository containing the new config filename must lint successfully without an explicit `--config` flag, and tests must prove that repository-root inference also uses that renamed file as the marker. Any tests or fixtures that still rely on `docgarden.toml` should be renamed as part of the cutover.

Fourth, the repository documentation must be self-consistent. The README installation command, usage examples, fix hints, architecture description, and active plan added here must all use `docgarden` and the explicit subcommand model where they are intended as live operational guidance. Historical completed-plan notes may mention `docgarden` when they are clearly framed as history.

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
      skill   Reserved namespace for future skill workflows

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
Revision note: Updated on 2026-04-01 to keep `docgarden` as the sole canonical binary name, confirm `fix` as a separate explicit subcommand, and move broader `skill` behavior into a separate future ExecPlan while keeping the singular `skill` namespace reserved in this plan.
Revision note: Updated on 2026-04-01 to replace unreliable `cargo test <filter>` examples with `cargo test --test cli <exact-test-name>` commands so the TDD gate cannot pass while running zero CLI integration tests.
