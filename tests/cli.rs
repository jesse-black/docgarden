use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod common;

use common::fixture_repo;

// ---------------------------------------------------------------------------
// match subcommand integration tests
// ---------------------------------------------------------------------------

#[test]
fn match_help_documents_output_columns_flags_and_alias() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["match", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("score"))
        .stdout(predicate::str::contains("name"))
        .stdout(predicate::str::contains("description"))
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--path-only"))
        .stdout(predicate::str::contains("-n"))
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("Alias: `m`"))
        .stdout(predicate::str::contains("1-24 is low"))
        .stdout(predicate::str::contains("25-59 is medium"))
        .stdout(predicate::str::contains("60+ is high"));
}

#[test]
fn root_help_lists_match_subcommand() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("match"));
}

#[test]
fn match_alias_m_works_identically_to_match() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let full = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring"])
        .output()
        .unwrap();

    let alias = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["m", "scoring"])
        .output()
        .unwrap();

    assert_eq!(full.stdout, alias.stdout);
    assert!(full.status.success());
}

#[test]
fn match_multi_token_query_accepted_without_quoting() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // "scoring guide" as two separate args should find the scoring-guide doc
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring", "guide"])
        .assert()
        .success()
        .stdout(predicate::str::contains("scoring-guide"));
}

#[test]
fn match_name_hit_ranks_above_description_only_hit() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // scoring-guide has "scoring" in its `name` field.
    // discovery-overview has "scoring" only in its `description` field.
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let paths: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.split(" | ").nth(1))
        .collect();

    let scoring_guide_pos = paths.iter().position(|p| p.contains("scoring-guide"));
    let discovery_pos = paths.iter().position(|p| p.contains("discovery-overview"));

    let (sg, do_) = (scoring_guide_pos.unwrap(), discovery_pos.unwrap());
    assert!(
        sg < do_,
        "scoring-guide (pos {sg}) should rank above discovery-overview (pos {do_})"
    );
}

#[test]
fn match_rare_term_returns_only_matching_doc() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // "radium" appears only in common-word.md; all other docs should score 0 and be dropped.
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "radium"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "expected exactly 1 result, got: {stdout}");
    assert!(
        lines[0].contains("common-word"),
        "expected common-word.md, got: {}",
        lines[0]
    );
}

#[test]
fn match_limit_truncates_to_n_results() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // "scoring" matches multiple docs; limit to 2
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--limit", "2", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let line_count = stdout.lines().count();
    assert_eq!(
        line_count, 2,
        "expected 2 results with --limit 2, got {line_count}"
    );
}

#[test]
fn match_path_only_emits_one_path_per_line() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--path-only", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines() {
        // Each line must be a plain path (no " | " separator)
        assert!(
            !line.contains(" | "),
            "--path-only output should not contain ` | `, got: {line}"
        );
        assert!(
            line.ends_with(".md"),
            "--path-only output should be a .md path, got: {line}"
        );
    }
}

#[test]
fn match_path_only_with_limit() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--path-only", "-n", "1", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "expected 1 path with -n 1, got: {stdout}");
}

#[test]
fn match_color_always_and_never_control_only_score_column() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    fs::write(
        root.join("docs/low.md"),
        "---\ndescription: zetasoup\n---\n# Low\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/medium.md"),
        "---\nname: Uniquealpha\n---\n# Medium\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/high.md"),
        "---\nname: Alpha Beta\n---\n# High\n",
    )
    .unwrap();

    let low = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "always", "etas"])
        .output()
        .unwrap();
    assert!(low.status.success());
    let low_stdout = String::from_utf8(low.stdout).unwrap();
    assert!(low_stdout.contains("\u{1b}[31m1\u{1b}[0m | docs/low.md"));

    let medium = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "always", "uniquealpha"])
        .output()
        .unwrap();
    assert!(medium.status.success());
    let medium_stdout = String::from_utf8(medium.stdout).unwrap();
    assert!(medium_stdout.contains("\u{1b}[33m51\u{1b}[0m | docs/medium.md"));

    let high = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "always", "alpha", "beta"])
        .output()
        .unwrap();
    assert!(high.status.success());
    let high_stdout = String::from_utf8(high.stdout).unwrap();
    assert!(high_stdout.contains("\u{1b}[32m127\u{1b}[0m | docs/high.md"));

    let never = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "never", "alpha", "beta"])
        .output()
        .unwrap();
    assert!(never.status.success());
    let never_stdout = String::from_utf8(never.stdout).unwrap();
    assert!(!never_stdout.contains("\u{1b}["));
    assert!(never_stdout.contains("127 | docs/high.md"));
}

#[test]
fn match_path_only_never_emits_color_even_when_forced() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--path-only", "--color", "always", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn match_no_gitignore_exposes_hidden_doc() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // Without --no-gitignore: hidden/secret-scoring.md is excluded by .gitignore
    let without = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring"])
        .output()
        .unwrap();
    let without_stdout = String::from_utf8(without.stdout).unwrap();
    assert!(
        !without_stdout.contains("secret-scoring"),
        "expected hidden doc to be absent without --no-gitignore: {without_stdout}"
    );

    // With --no-gitignore: hidden/secret-scoring.md should appear
    let with_flag = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--no-gitignore", "scoring"])
        .output()
        .unwrap();
    let with_stdout = String::from_utf8(with_flag.stdout).unwrap();
    assert!(
        with_stdout.contains("secret-scoring"),
        "expected hidden doc to appear with --no-gitignore: {with_stdout}"
    );
}

#[test]
fn match_no_results_exits_zero_and_emits_nothing() {
    let (_temp, root) = fixture_repo("discovery-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "qxjzv987unmatched"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[cfg(unix)]
#[test]
fn match_reports_read_error_for_unreadable_discovered_file() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    let unreadable = root.join("docs/unreadable.md");
    fs::write(&unreadable, "---\nname: Hidden\n---\n# Hidden\n").unwrap();

    let mut perms = fs::metadata(&unreadable).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&unreadable, perms).unwrap();

    let assert = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "hidden"])
        .assert();

    let mut restore = fs::metadata(&unreadable).unwrap().permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&unreadable, restore).unwrap();

    assert
        .failure()
        .stderr(predicate::str::contains("failed to read"))
        .stderr(predicate::str::contains("docs/unreadable.md"));
}

#[test]
fn match_path_only_doc_still_scores_via_path() {
    let (_temp, root) = fixture_repo("discovery-repo");

    // no-frontmatter.md has no frontmatter but its path contains "no-frontmatter";
    // querying "frontmatter" should find it via path tokenization.
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "frontmatter"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no-frontmatter"));
}

#[test]
fn match_output_format_has_four_pipe_separated_columns() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring", "--limit", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let first_line = stdout
        .lines()
        .next()
        .expect("expected at least one result line");
    let cols: Vec<&str> = first_line.split(" | ").collect();
    assert_eq!(cols.len(), 4, "expected 4 columns in output: {first_line}");

    // First column must parse as a non-negative integer score.
    let score: i32 = cols[0]
        .trim()
        .parse()
        .expect("first column should be integer score");
    assert!(score > 0, "score should be positive: {score}");
}

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
fn explicit_non_md_target_fails_with_error() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("notes.txt"), "Plain text file.\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "notes.txt", "--color", "never"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a Markdown file"));
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
