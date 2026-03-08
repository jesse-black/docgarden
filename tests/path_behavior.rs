use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn directory_trailing_slash_is_accepted() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "See `docs/` for repository documentation.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noncanonical-local-path").not());
}

#[test]
fn relative_inline_path_is_accepted_in_backtick_mode() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "See `./real.md` for the current guide.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noncanonical-local-path").not());
}

#[test]
fn workspace_root_backtick_path_is_accepted() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("README.md"),
        "See `/docs/real.md` for the current guide.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn whitespace_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example comment: `// test test_name`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn double_slash_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example token: `//foo`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn ellipsis_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example token: `/Users/alice/...`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn colon_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example token: `C:/tmp/file.txt`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn bare_slash_only_inline_reference_is_not_treated_as_missing_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/parser`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn ambiguous_inline_code_is_quiet_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/base_db`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn ambiguous_inline_code_can_be_enabled_explicitly() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/base_db`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ambiguous-inline-code"));
}

#[test]
fn glob_pattern_inline_code_is_ignored() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\nreport-ambiguous-inline-code = true\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Pattern: `docs/**/*.md`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn glob_pattern_markdown_link_is_reported_as_broken_link() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"links\"\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "[docs glob](docs/**/*.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-local-path"))
        .stdout(predicate::str::contains("docs/**/*.md"));
}

#[test]
fn same_directory_markdown_link_resolves_relative_to_current_file() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"links\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/architecture-md.md"), "# Architecture\n").unwrap();
    fs::write(
        root.join("docs/repository-knowledge-system.md"),
        "[Architecture](architecture-md.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn workspace_root_markdown_link_resolves_from_repo_root() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"links\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/architecture-md.md"), "# Architecture\n").unwrap();
    fs::write(
        root.join("docs/repository-knowledge-system.md"),
        "[Architecture](/docs/architecture-md.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}
