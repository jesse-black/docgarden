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
    let (_temp, root) = fixture_repo("test-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-links-for-local-paths"))
        .stdout(predicate::str::contains("fixable"))
        .stdout(predicate::str::contains("Run `docgarden fix "))
        .stdout(predicate::str::contains("--config").not());
}

#[test]
fn explicit_file_target_reports_fixable_diagnostics() {
    let (_temp, root) = fixture_repo("test-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-links-for-local-paths"))
        .stdout(predicate::str::contains(
            "Run `docgarden fix docs/guide.md`",
        ));
}

#[test]
fn fix_subcommand_rewrites_files_and_second_lint_passes() {
    let (_temp, root) = fixture_repo("test-repo");

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
fn explicit_file_fix_rewrites_and_second_passes() {
    let (_temp, root) = fixture_repo("test-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["fix", "docs/guide.md"])
        .assert()
        .success();

    let doc = fs::read_to_string(root.join("docs/guide.md")).unwrap();
    assert!(doc.contains("[./real.md](real.md)"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md"])
        .assert()
        .success();
}

#[test]
fn explicit_file_list_is_supported() {
    let (_temp, root) = fixture_repo("test-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["lint", "docs/guide.md", "docs/real.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("prefer-links-for-local-paths"))
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
        "[Missing](missing.md)\n",
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
        .stdout(predicate::str::contains("unresolved-link-path"));
}

#[test]
fn config_can_opt_out_of_gitignore_support() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/package/docgarden-0.1.0/docs")).unwrap();
    fs::write(root.join("docgarden.toml"), "respect_gitignore = false\n").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join("target/package/docgarden-0.1.0/docs/stale.md"),
        "[Missing](missing.md)\n",
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
        .stdout(predicate::str::contains("unresolved-link-path"));
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
fn fix_rewrites_backticks_to_links_in_fixture_repo() {
    let (_temp, root) = fixture_repo("test-repo");

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
        "[[rules]]\npath = \"README.md\"\nenable = [\"prefer-links-for-local-paths\"]\n",
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
        "For more, see `docs/PRODUCT.md` and `LICENSE`.\n",
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
        "For more, see [docs/PRODUCT.md](docs/PRODUCT.md) and [LICENSE](LICENSE).\n",
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

#[test]
fn fix_respects_rule_disable_for_readme_style_rules() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        concat!(
            "[[rules]]\n",
            "path = \"**\"\n",
            "enable = [\"prefer-links-for-local-paths\"]\n",
            "\n",
            "[[rules]]\n",
            "path = \"README.md\"\n",
            "disable = [\"prefer-links-for-local-paths\"]\n",
        ),
    )
    .unwrap();
    fs::write(root.join("docs/PRODUCT.md"), "# Product\n").unwrap();
    fs::write(root.join("LICENSE"), "Apache-2.0\n").unwrap();
    let readme = root.join("README.md");
    let original = concat!(
        "# Doc Garden\n",
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
    assert_eq!(rewritten, original);
}

#[test]
fn fix_handles_multibyte_text_before_rewrites_without_corruption() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        "[[rules]]\npath = \"README.md\"\nenable = [\"prefer-links-for-local-paths\"]\n",
    )
    .unwrap();
    fs::write(root.join("docs/PRODUCT.md"), "# Product\n").unwrap();
    fs::write(root.join("LICENSE"), "Apache-2.0\n").unwrap();
    let readme = root.join("README.md");
    let original = concat!(
        "Préface\n",
        "\n",
        "For more, see `docs/PRODUCT.md` and `LICENSE`.\n",
    );
    let expected = concat!(
        "Préface\n",
        "\n",
        "For more, see [docs/PRODUCT.md](docs/PRODUCT.md) and [LICENSE](LICENSE).\n",
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

#[test]
fn context_budget_reports_max_tokens_as_error_by_default() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_tokens = 1
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error  max_tokens"))
        .stdout(predicate::str::contains("File has"))
        .stdout(predicate::str::contains(
            "which exceeds configured max_tokens = 1.",
        ))
        .stdout(predicate::str::contains("fixable").not());
}

#[test]
fn context_budget_warn_severity_does_not_fail_lint() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_lines = 1
severity = "warn"
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "first\nsecond\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("warning  max_lines"))
        .stdout(predicate::str::contains(
            "File has 2 lines, which exceeds configured max_lines = 1.",
        ));
}

#[test]
fn context_budget_disable_suppresses_one_budget_rule() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_tokens = 1
max_lines = 1

[[rules]]
path = "README.md"
disable = ["max_tokens"]
reason = "Only line length is enforced here."
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\nsecond line\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("max_lines"))
        .stdout(predicate::str::contains("max_tokens").not());
}

#[test]
fn context_budget_duplicate_path_entries_can_split_severity() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_tokens = 1

[[rules]]
path = "README.md"
max_lines = 1
severity = "warn"
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\nsecond line\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error  max_tokens"))
        .stdout(predicate::str::contains("warning  max_lines"));
}

#[test]
fn context_budget_later_matching_limit_wins() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_tokens = 1

[[rules]]
path = "README.md"
max_tokens = 1000
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_tokens").not());
}

#[test]
fn context_budget_later_matching_limit_can_reenable_after_disable() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_tokens = 1

[[rules]]
path = "README.md"
disable = ["max_tokens"]
reason = "Temporarily disable token budget."

[[rules]]
path = "README.md"
max_tokens = 1
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error  max_tokens"));
}

#[test]
fn context_budget_fix_does_not_rewrite_over_budget_files() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "README.md"
max_lines = 1
"#,
    )
    .unwrap();
    let readme = root.join("README.md");
    let original = "first\nsecond\n";
    fs::write(&readme, original).unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["fix", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("max_lines"))
        .stdout(predicate::str::contains("fixable").not());

    let rewritten = fs::read_to_string(readme).unwrap();
    assert_eq!(rewritten, original);
}

#[test]
fn frontmatter_missing_required_field_is_reported() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    // File with no frontmatter at all.
    fs::write(root.join("README.md"), "# Hello\n\nBody text.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("frontmatter-field-missing"))
        .stdout(predicate::str::contains("`description`"))
        .stdout(predicate::str::contains("fixable").not());
}

#[test]
fn frontmatter_present_but_missing_required_field_is_reported() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    // File with frontmatter but missing the required field.
    fs::write(
        root.join("guide.md"),
        "---\ntitle: My Guide\n---\n\n# Guide\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("frontmatter-field-missing"))
        .stdout(predicate::str::contains("`description`"));
}

#[test]
fn frontmatter_agents_md_excluded_from_required_field() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    // AGENTS.md has no frontmatter – should not trigger missing-field diagnostic.
    fs::write(root.join("AGENTS.md"), "# Agent Instructions\n\nBody.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "AGENTS.md", "--color", "never"])
        .assert()
        .success();
}

#[test]
fn frontmatter_non_md_files_unaffected_by_frontmatter_rules() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    // Frontmatter rules target only **/*.md.  A .txt file should not trigger them.
    fs::write(
        root.join("docgarden.toml"),
        r#"
include = ["*.md", "*.txt"]

[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    fs::write(root.join("notes.txt"), "Plain text file.\n").unwrap();
    fs::write(
        root.join("guide.md"),
        "---\ndescription: A guide.\n---\n# Guide\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "notes.txt", "guide.md", "--color", "never"])
        .assert()
        .success();
}

#[test]
fn frontmatter_malformed_block_reported_distinctly_from_missing_field() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"
exclude = ["AGENTS.md"]

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    // File with a leading --- but no closing ---.
    fs::write(
        root.join("guide.md"),
        "---\ndescription: A guide.\n\n# Body starts without closing delimiter\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("frontmatter-malformed"))
        .stdout(predicate::str::contains("frontmatter-field-missing").not());
}

#[test]
fn frontmatter_max_chars_enforced_for_present_field() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
max_chars = 20
"#,
    )
    .unwrap();
    fs::write(
        root.join("guide.md"),
        "---\ndescription: This description is definitely longer than twenty characters.\n---\n# Guide\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "guide.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("frontmatter-field-max-chars"))
        .stdout(predicate::str::contains("max_chars = 20"));
}

#[test]
fn frontmatter_max_chars_not_triggered_when_field_absent() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter.fields.description]
max_chars = 20
"#,
    )
    .unwrap();
    // File has no frontmatter at all – max_chars should not fire.
    fs::write(root.join("guide.md"), "# Guide\n\nBody.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "guide.md", "--color", "never"])
        .assert()
        .success();
}
