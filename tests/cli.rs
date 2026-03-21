use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

mod common;

use common::fixture_repo;

#[test]
fn lint_reports_fixable_diagnostics_for_fixture_repo() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains("fixable"))
        .stdout(predicate::str::contains("Run `dglint "))
        .stdout(predicate::str::contains("--fix"))
        .stdout(predicate::str::contains("--config").not());
}

#[test]
fn explicit_file_target_reports_fixable_diagnostics() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(&root)
        .args(["docs/guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains("Run `dglint docs/guide.md --fix`"));
}

#[test]
fn fix_rewrites_files_and_second_lint_passes() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--fix"])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("`docs/real.md`"));

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn explicit_file_fix_rewrites_and_second_passes() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(&root)
        .args(["docs/guide.md", "--fix"])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("`docs/real.md`"));

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(&root)
        .args(["docs/guide.md"])
        .assert()
        .success();
}

#[test]
fn explicit_file_list_is_supported() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(&root)
        .args(["docs/guide.md", "docs/real.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-backticks-for-local-paths"))
        .stdout(predicate::str::contains(
            "Run `dglint docs/guide.md docs/real.md --fix`",
        ));
}

#[test]
fn explicit_directory_target_does_not_scan_outside_directory() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/dglint-0.1.0/docs")).unwrap();
    fs::write(root.join("dglint.toml"), "").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/dglint-0.1.0/docs/stale.md"),
        "See `scripts/setup-jules.sh`.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(root)
        .args(["docs"])
        .assert()
        .success();
}

#[test]
fn git_root_is_used_when_no_dglint_toml_is_found() {
    let temp = tempdir().unwrap();
    let workspace = temp.path();
    let root = workspace.join("repo");
    let outside = workspace.join("outside");
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(&outside)
        .arg(root.join("docs/guide.md"))
        .assert()
        .success();
}

#[test]
fn fix_rewrites_backticks_to_links_in_link_mode() {
    let (_temp, root) = fixture_repo("links");

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap(), "--fix"])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("[./real.md](real.md)"));

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .args([root.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn fix_preserves_unrelated_readme_formatting() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("dglint.toml"),
        "local-reference-style = \"backticks\"\n",
    )
    .unwrap();
    fs::write(root.join("docs/PRODUCT.md"), "# Product\n").unwrap();
    fs::write(root.join("LICENSE"), "Apache-2.0\n").unwrap();
    let readme = root.join("README.md");
    let original = concat!(
        "# Doc Gardening Linter\n",
        "\n",
        "[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/dglint/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/dglint/actions/workflows/ci.yml)\n",
        "[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)\n",
        "\n",
        "\n",
        "Text with a trailing space. \n",
        "\n",
        "For more, see [docs/PRODUCT.md](docs/PRODUCT.md) and [LICENSE](LICENSE).\n",
    );
    let expected = concat!(
        "# Doc Gardening Linter\n",
        "\n",
        "[![CI](https://img.shields.io/github/actions/workflow/status/jesse-black/dglint/ci.yml?branch=main&label=CI)](https://github.com/jesse-black/dglint/actions/workflows/ci.yml)\n",
        "[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)\n",
        "\n",
        "\n",
        "Text with a trailing space. \n",
        "\n",
        "For more, see `docs/PRODUCT.md` and `LICENSE`.\n",
    );
    fs::write(&readme, original).unwrap();

    Command::new(env!("CARGO_BIN_EXE_dglint"))
        .current_dir(root)
        .args(["README.md", "--fix"])
        .assert()
        .success();

    let rewritten = fs::read_to_string(readme).unwrap();
    assert_eq!(rewritten, expected);
}
