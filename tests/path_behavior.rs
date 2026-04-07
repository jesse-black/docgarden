use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn directory_trailing_slash_is_accepted() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(
        root.join("README.md"),
        "See `docs/` for repository documentation.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noncanonical-local-path").not());
}

#[test]
fn missing_directory_reference_is_ignored_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(
        root.join("README.md"),
        "Plans move through `docs/exec-plans/active/` before completion.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn relative_inline_path_is_accepted_in_backtick_mode() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "See `./real.md` for the current guide.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("noncanonical-local-path").not());
}

#[test]
fn workspace_root_backtick_path_is_accepted() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("README.md"),
        "See `/docs/real.md` for the current guide.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn whitespace_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example comment: `// test test_name`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
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
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example token: `//foo`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
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
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example token: `/Users/alice/...`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
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
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "Example token: `C:/tmp/file.txt`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn bare_slash_only_inline_reference_is_not_treated_as_missing_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/parser`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn ambiguous_inline_code_is_quiet_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "path_style = \"backticks\"\n").unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/base_db`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn ambiguous_inline_code_can_be_enabled_explicitly() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/base_db`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ambiguous-inline-code"));
}

#[test]
fn rule_application_disables_unresolved_paths_for_path_scope_only() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs/references")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/references/**"
disable = ["unresolved-local-path"]
reason = "Imported references may contain source-derived paths."
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/references/source.md"),
        "[Missing](missing.md)\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "[Missing](missing.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-local-path").count(1))
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("docs/references/source.md").not());
}

#[test]
fn rule_application_path_disable_suppresses_style_but_not_unresolved_paths() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
path_style = "backticks"

[[rules]]
path = "README.md"
disable = ["prefer-backticks-for-local-paths"]
reason = "README stays human-facing."
"#,
    )
    .unwrap();
    fs::write(root.join("docs/PRODUCT.md"), "# Product\n").unwrap();
    fs::write(
        root.join("README.md"),
        "For more, see [docs/PRODUCT.md](docs/PRODUCT.md) and [Missing](missing.md).\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-local-path"))
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths").not());
}

#[test]
fn rule_application_enables_ambiguous_inline_code_for_matching_paths_only() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/**"
enable = ["ambiguous-inline-code"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "Example crate: `crates/base_db`.\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/base_db`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ambiguous-inline-code").count(1))
        .stdout(predicate::str::contains("docs/guide.md"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn rule_application_local_reference_style_override_is_scoped() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
path_style = "backticks"

[[rules]]
path = "docs/**"
path_style = "links"
"#,
    )
    .unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "See `./real.md` for the current guide.\n",
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "See `docs/real.md` for the current guide.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-links-for-local-paths"))
        .stdout(predicate::str::contains("docs/guide.md"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn glob_pattern_inline_code_is_ignored() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "path_style = \"backticks\"\n\n[[rules]]\npath = \"**\"\nenable = [\"ambiguous-inline-code\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Pattern: `docs/**/*.md`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not())
        .stdout(predicate::str::contains("ambiguous-inline-code").not());
}

#[test]
fn glob_pattern_markdown_link_is_reported_as_broken_link() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "path_style = \"links\"\n").unwrap();
    fs::write(root.join("README.md"), "[docs glob](docs/**/*.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
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
    fs::write(root.join("docgarden.toml"), "path_style = \"links\"\n").unwrap();
    fs::write(root.join("docs/architecture-md.md"), "# Architecture\n").unwrap();
    fs::write(
        root.join("docs/repository-knowledge-system.md"),
        "[Architecture](architecture-md.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn workspace_root_markdown_link_resolves_from_repo_root() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "path_style = \"links\"\n").unwrap();
    fs::write(root.join("docs/architecture-md.md"), "# Architecture\n").unwrap();
    fs::write(
        root.join("docs/repository-knowledge-system.md"),
        "[Architecture](/docs/architecture-md.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-local-path").not());
}

#[test]
fn ignored_style_rule_in_readme_still_lints_backticked_link_as_one_link() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        concat!(
            "path_style = \"backticks\"\n",
            "\n",
            "[[rules]]\n",
            "path = \"README.md\"\n",
            "disable = [\"prefer-backticks-for-local-paths\"]\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "* [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-local-path").count(1))
        .stdout(predicate::str::contains(
            "Local repository link `[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)` does not resolve within the repository.",
        ))
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths").not());
}
