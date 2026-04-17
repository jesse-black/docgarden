use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn directory_trailing_slash_is_accepted() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(
        root.join("README.md"),
        "See `docs/` for repository documentation.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn missing_directory_reference_is_ignored_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(
        root.join("README.md"),
        "Plans move through `docs/exec-plans/active/` before completion.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn relative_inline_path_is_accepted_when_backtick_rule_enabled() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
    )
    .unwrap();
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
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn workspace_root_backtick_path_is_silent_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
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
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn broken_backtick_path_is_silent_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("README.md"), "See `docs/missing.md` for more.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn broken_backtick_path_reports_when_rule_enabled() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "See `docs/missing.md` for more.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-backtick-path"))
        .stdout(predicate::str::contains("docs/missing.md"));
}

#[test]
fn broken_backtick_path_honors_configured_severity() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\nseverity = \"warn\"\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "See `docs/missing.md` for more.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path"))
        .stdout(predicate::str::contains("warning"));
}

#[test]
fn whitespace_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
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
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn double_slash_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Example token: `//foo`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn ellipsis_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
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
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn colon_backtick_token_is_not_treated_as_a_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
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
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn bare_slash_only_inline_reference_is_silent_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("README.md"), "Example crate: `crates/parser`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn broken_local_link_reports_unresolved_link_path_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("README.md"), "[Missing](missing.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-link-path"));
}

#[test]
fn rule_application_disables_unresolved_link_path_for_path_scope_only() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs/references")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/references/**"
disable = ["unresolved-link-path"]
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
        .stdout(predicate::str::contains("unresolved-link-path").count(1))
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("docs/references/source.md").not());
}

#[test]
fn rule_application_directory_path_matches_descendants_without_glob_suffix() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs/references")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/references"
disable = ["unresolved-link-path"]
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
        .stdout(predicate::str::contains("unresolved-link-path").count(1))
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("docs/references/source.md").not());
}

#[test]
fn rule_application_enables_unresolved_backtick_path_for_matching_paths_only() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "See `docs/missing.md` for more.\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "See `docs/missing.md` for more.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-backtick-path").count(1))
        .stdout(predicate::str::contains("docs/guide.md"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn prefer_links_for_local_paths_appears_only_when_explicitly_enabled() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "docs/**"
enable = ["prefer-links-for-local-paths"]
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
        "[[rules]]\npath = \"**\"\nenable = [\"unresolved-backtick-path\"]\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "Pattern: `docs/**/*.md`.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unresolved-backtick-path").not());
}

#[test]
fn glob_pattern_markdown_link_is_reported_as_broken_link() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("README.md"), "[docs glob](docs/**/*.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-link-path"))
        .stdout(predicate::str::contains("docs/**/*.md"));
}

#[test]
fn same_directory_markdown_link_resolves_relative_to_current_file() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
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
        .stdout(predicate::str::contains("unresolved-link-path").not());
}

#[test]
fn workspace_root_markdown_link_resolves_from_repo_root() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
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
        .stdout(predicate::str::contains("unresolved-link-path").not());
}

#[test]
fn broken_link_in_readme_reports_once() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(
        root.join("README.md"),
        "* [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unresolved-link-path").count(1))
        .stdout(predicate::str::contains(
            "Local repository link `[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)` does not resolve within the repository.",
        ));
}

#[test]
fn disable_then_enable_restores_unresolved_backtick_path_diagnostics() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
enable = ["unresolved-backtick-path"]

[[rules]]
path = "**/*.md"
disable = ["unresolved-backtick-path"]

[[rules]]
path = "docs/**"
enable = ["unresolved-backtick-path"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("docs/guide.md"),
        "See `docs/missing.md` for more.\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "See `docs/missing.md` for more.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        // docs/guide.md: enabled, disabled, re-enabled — should report
        .stdout(predicate::str::contains("docs/guide.md"))
        // README.md: enabled then disabled, no later re-enable — should be silent
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn disable_then_enable_restores_prefer_links_diagnostics() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/real.md"), "# Real\n").unwrap();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
enable = ["prefer-links-for-local-paths"]

[[rules]]
path = "**/*.md"
disable = ["prefer-links-for-local-paths"]

[[rules]]
path = "docs/**"
enable = ["prefer-links-for-local-paths"]
"#,
    )
    .unwrap();
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
        // docs/guide.md: enabled, disabled, re-enabled — should report
        .stdout(predicate::str::contains("docs/guide.md"))
        // README.md: enabled then disabled, no later re-enable — should be silent
        .stdout(predicate::str::contains("README.md").not());
}
