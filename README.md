# dglint

`dglint` is the `Doc Gardening Linter`, a Rust CLI for checking repository-local file references in Markdown documentation.

## Build

From the repository root:

    cargo build

To build and run the CLI in one step:

    cargo run -- --help

## Run

Lint the current repository:

    cargo run -- .

Lint a single file or an explicit file list:

    cargo run -- docs/exec-plans/active/doc-gardening-linter.md
    cargo run -- README.md AGENTS.md

Apply safe autofixes:

    cargo run -- . --fix

Use an explicit config file:

    cargo run -- . --config dglint.toml

Enable optional warnings for path-adjacent inline backticks such as `` `crates/parser` ``:

    report-ambiguous-inline-code = true

## Test

Run the automated test suite:

    cargo test

Run the quiet form used for quick local verification:

    cargo test --quiet

Generate coverage if `cargo-llvm-cov` is installed:

    cargo llvm-cov --summary-only
