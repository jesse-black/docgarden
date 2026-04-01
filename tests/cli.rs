use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

mod common;

use common::fixture_repo;

#[test]
fn root_help_lists_explicit_subcommands() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: docgarden <COMMAND>"))
        .stdout(predicate::str::contains("lint"))
        .stdout(predicate::str::contains("fix"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("skill"))
        .stdout(predicate::str::contains("[TARGETS]").not())
        .stdout(predicate::str::contains("--fix").not());
}

#[test]
fn lint_and_fix_help_list_shared_flags() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[TARGETS]..."))
        .stdout(predicate::str::contains("--config <CONFIG>"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--no-gitignore"))
        .stdout(predicate::str::contains("--color <COLOR>"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[TARGETS]..."))
        .stdout(predicate::str::contains("--config <CONFIG>"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--no-gitignore"))
        .stdout(predicate::str::contains("--color <COLOR>"));
}

#[test]
fn lint_subcommand_reports_fixable_diagnostics_for_fixture_repo() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains("fixable"))
        .stdout(predicate::str::contains("Run `docgarden fix "))
        .stdout(predicate::str::contains("--config").not());
}

#[test]
fn explicit_file_target_reports_fixable_diagnostics() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains(
            "Run `docgarden fix docs/guide.md`",
        ));
}

#[test]
fn fix_subcommand_rewrites_files_and_second_lint_passes() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["fix", root.to_str().unwrap()])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("`docs/real.md`"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn explicit_file_fix_rewrites_and_second_passes() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["fix", "docs/guide.md"])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("`docs/real.md`"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md"])
        .assert()
        .success();
}

#[test]
fn explicit_file_list_is_supported() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md", "docs/real.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains(
            "Run `docgarden fix docs/guide.md docs/real.md`",
        ));
}

#[test]
fn explicit_directory_target_does_not_scan_outside_directory() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/docgarden-0.1.0/docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/docgarden-0.1.0/docs/stale.md"),
        "See `scripts/setup-jules.sh`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "docs"])
        .assert()
        .success();
}

#[test]
fn gitignored_files_are_skipped_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/docgarden-0.1.0/docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/docgarden-0.1.0/docs/stale.md"),
        "See `scripts/setup-jules.sh`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "."])
        .assert()
        .success();
}

#[test]
fn no_gitignore_flag_scans_gitignored_files() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/docgarden-0.1.0/docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/docgarden-0.1.0/docs/stale.md"),
        "See `scripts/setup-jules.sh`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", ".", "--no-gitignore", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "target/package/docgarden-0.1.0/docs/stale.md",
        ))
        .stdout(predicate::str::contains("unresolved-local-path"));
}

#[test]
fn config_can_opt_out_of_gitignore_support() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/docgarden-0.1.0/docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "respect-gitignore = false\n").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/docgarden-0.1.0/docs/stale.md"),
        "See `scripts/setup-jules.sh`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", ".", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "target/package/docgarden-0.1.0/docs/stale.md",
        ))
        .stdout(predicate::str::contains("unresolved-local-path"));
}

#[test]
fn git_root_is_used_when_no_docgarden_toml_is_found() {
    let temp = tempdir().unwrap();
    let workspace = temp.path();
    let root = workspace.join("repo");
    let outside = workspace.join("outside");
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&outside)
        .args(["lint", root.join("docs/guide.md").to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn fix_rewrites_backticks_to_links_in_link_mode() {
    let (_temp, root) = fixture_repo("links");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["fix", root.to_str().unwrap()])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("[./real.md](real.md)"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn fix_preserves_unrelated_readme_formatting() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/PRODUCT.md"), "# Product\n").unwrap();
    fs::write(root.join("LICENSE"), "Apache-2.0\n").unwrap();
    let readme = root.join("README.md");
    let original = concat!(
        "# Doc Garden\n",
        "\n",
        "[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/docgarden/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/docgarden/actions/workflows/ci.yml)\n",
        "[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)\n",
        "\n",
        "\n",
        "Text with a trailing space. \n",
        "\n",
        "For more, see [docs/PRODUCT.md](docs/PRODUCT.md) and [LICENSE](LICENSE).\n",
    );
    let expected = concat!(
        "# Doc Garden\n",
        "\n",
        "[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/docgarden/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/docgarden/actions/workflows/ci.yml)\n",
        "[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)\n",
        "\n",
        "\n",
        "Text with a trailing space. \n",
        "\n",
        "For more, see `docs/PRODUCT.md` and `LICENSE`.\n",
    );
    fs::write(&readme, original).unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["fix", "README.md"])
        .assert()
        .success();

    let rewritten = fs::read_to_string(readme).unwrap();
    assert_eq!(rewritten, expected);
}
