use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod common;

use common::fixture_repo;

const PACKAGED_DOCS_DIR: &str = concat!(
    "target/package/docgarden-",
    env!("CARGO_PKG_VERSION"),
    "/docs"
);
const PACKAGED_STALE_DOC: &str = concat!(
    "target/package/docgarden-",
    env!("CARGO_PKG_VERSION"),
    "/docs/stale.md"
);

#[derive(Debug)]
struct MatchRow {
    path: String,
    name: String,
    description: String,
}

fn parse_match_rows(stdout: &str) -> Vec<MatchRow> {
    stdout
        .lines()
        .map(|line| {
            let mut cols = line.split(" | ");
            let path = cols.next().expect("match output should include path");
            let name = cols.next().expect("match output should include name");
            let description = cols
                .next()
                .expect("match output should include description");
            assert!(
                cols.next().is_none(),
                "expected 3 columns in output: {line}"
            );
            MatchRow {
                path: path.to_string(),
                name: name.to_string(),
                description: description.to_string(),
            }
        })
        .collect()
}

#[derive(Debug)]
struct ExplainRow {
    score: f32,
    relative: String,
    coverage: String,
    path: String,
    name: String,
    description: String,
}

fn parse_explain_rows(stdout: &str) -> Vec<ExplainRow> {
    let mut lines = stdout.lines();
    let header = lines.next().expect("expected explain header row");
    assert_eq!(
        header,
        "score | relative | coverage | path | name | description"
    );

    lines
        .map(|line| {
            let mut cols = line.split(" | ");
            let score = cols.next().expect("explain output should include score");
            let relative = cols.next().expect("explain output should include relative");
            let coverage = cols.next().expect("explain output should include coverage");
            let path = cols.next().expect("explain output should include path");
            let name = cols.next().expect("explain output should include name");
            let description = cols
                .next()
                .expect("explain output should include description");
            assert!(
                cols.next().is_none(),
                "expected 6 columns in explain output: {line}"
            );
            ExplainRow {
                score: score.trim().parse().expect("score should be float"),
                relative: relative.to_string(),
                coverage: coverage.to_string(),
                path: path.to_string(),
                name: name.to_string(),
                description: description.to_string(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// match subcommand integration tests
// ---------------------------------------------------------------------------

#[test]
fn match_help_documents_output_columns_and_flags() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["match", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: docgarden match [OPTIONS] <QUERY>...",
        ))
        .stdout(predicate::str::contains(
            "Find repository Markdown documents by description, name, or path.",
        ))
        .stdout(predicate::str::contains("metadata").not())
        .stdout(predicate::str::contains("frontmatter").not())
        .stdout(predicate::str::contains("--limit"))
        .stdout(predicate::str::contains("--path-only"))
        .stdout(predicate::str::contains("--explain"))
        .stdout(predicate::str::contains("--color <COLOR>"))
        .stdout(predicate::str::contains("-n"))
        .stdout(predicate::str::contains("-p"));
}

#[test]
fn root_help_lists_match_subcommand() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("match"))
        .stdout(predicate::str::contains(
            "Find repository documents by description, name, or path",
        ));
}

#[test]
fn dg_binary_help_uses_alias_name() {
    Command::new(env!("CARGO_BIN_EXE_dg"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: dg <COMMAND>"))
        .stdout(predicate::str::contains("match"));
}

#[test]
fn list_help_documents_output_columns_and_flags() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: docgarden list [OPTIONS] [TARGETS]...",
        ))
        .stdout(predicate::str::contains(
            "List repository Markdown documents with their descriptions.",
        ))
        .stdout(predicate::str::contains("path | name | description"))
        .stdout(predicate::str::contains("metadata").not())
        .stdout(predicate::str::contains("frontmatter").not())
        .stdout(predicate::str::contains("--recurse"))
        .stdout(predicate::str::contains("--skills"))
        .stdout(predicate::str::contains("--plans"))
        .stdout(predicate::str::contains("--active-plans"))
        .stdout(predicate::str::contains("--completed-plans"))
        .stdout(predicate::str::contains("--color <COLOR>").not());
}

#[test]
fn root_help_lists_list_subcommand() {
    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains(
            "List repository documents and descriptions",
        ));
}

#[test]
fn list_alias_ls_works_identically_to_list() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let full = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "docs"])
        .output()
        .unwrap();

    let alias = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["ls", "docs"])
        .output()
        .unwrap();

    assert_eq!(full.stdout, alias.stdout);
    assert!(full.status.success());
}

#[test]
fn list_default_output_has_three_pipe_separated_columns() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "docs/scoring-guide.md"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let rows = parse_match_rows(&String::from_utf8(output.stdout).unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "docs/scoring-guide.md");
    assert_eq!(rows[0].name, "Scoring Guide");
    assert!(rows[0].description.contains("scoring"));
}

#[test]
fn list_directory_targets_are_shallow_without_recurse() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs/nested")).unwrap();
    fs::write(
        root.join("docs/top.md"),
        "---\nname: Top\ndescription: Top-level doc.\n---\n# Top\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/nested/deep.md"),
        "---\nname: Deep\ndescription: Nested doc.\n---\n# Deep\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "docs"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("docs/top.md"));
    assert!(!stdout.contains("docs/nested/deep.md"));
}

#[test]
fn list_recurse_descends_into_nested_directories() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs/nested")).unwrap();
    fs::write(
        root.join("docs/top.md"),
        "---\nname: Top\ndescription: Top-level doc.\n---\n# Top\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/nested/deep.md"),
        "---\nname: Deep\ndescription: Nested doc.\n---\n# Deep\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--recurse", "docs"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("docs/top.md"));
    assert!(stdout.contains("docs/nested/deep.md"));
}

#[test]
fn list_directory_targets_omit_docs_without_descriptions() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/cataloged.md"),
        "---\ndescription: Cataloged doc.\n---\n# Cataloged\n",
    )
    .unwrap();
    fs::write(root.join("docs/readme.md"), "# Readme\n").unwrap();
    fs::write(root.join("README.md"), "# Root Readme\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--recurse", "."])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("docs/cataloged.md"));
    assert!(!stdout.contains("docs/readme.md"));
    assert!(!stdout.contains("README.md"));
}

#[test]
fn list_explicit_file_target_renders_without_description() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("README.md"), "# Root Readme\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "README.md"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let rows = parse_match_rows(&String::from_utf8(output.stdout).unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "README.md");
    assert_eq!(rows[0].name, "README");
    assert_eq!(rows[0].description, "");
}

#[test]
fn list_scope_targets_omit_docs_without_descriptions() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs/exec-plans/active")).unwrap();
    fs::write(
        root.join("docs/exec-plans/active/cataloged.md"),
        "---\ndescription: Cataloged plan.\n---\n# Cataloged\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/exec-plans/active/undescribed.md"),
        "# Undescribed\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--active-plans"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("docs/exec-plans/active/cataloged.md"));
    assert!(!stdout.contains("docs/exec-plans/active/undescribed.md"));
}

#[test]
fn list_scope_switches_select_configured_sets() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let skills = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--skills"])
        .output()
        .unwrap();
    assert!(skills.status.success());
    let skills_stdout = String::from_utf8(skills.stdout).unwrap();
    assert!(skills_stdout.contains(".agents/skills/skillbeacon/SKILL.md"));
    assert!(!skills_stdout.contains("docs/exec-plans/active"));

    let plans = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--plans"])
        .output()
        .unwrap();
    assert!(plans.status.success());
    let plans_stdout = String::from_utf8(plans.stdout).unwrap();
    assert!(plans_stdout.contains("docs/exec-plans/active/current.md"));
    assert!(plans_stdout.contains("docs/exec-plans/completed/done.md"));

    let active = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--active-plans"])
        .output()
        .unwrap();
    assert!(active.status.success());
    let active_stdout = String::from_utf8(active.stdout).unwrap();
    assert!(active_stdout.contains("docs/exec-plans/active/current.md"));
    assert!(!active_stdout.contains("docs/exec-plans/completed/done.md"));

    let completed = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--completed-plans"])
        .output()
        .unwrap();
    assert!(completed.status.success());
    let completed_stdout = String::from_utf8(completed.stdout).unwrap();
    assert!(completed_stdout.contains("docs/exec-plans/completed/done.md"));
    assert!(!completed_stdout.contains("docs/exec-plans/active/current.md"));
}

#[test]
fn list_scope_switches_use_configured_directories() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        "skills-dir = \"custom/skills\"\nplans-dir = \"custom/plans\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("custom/skills/beacon")).unwrap();
    fs::create_dir_all(root.join("custom/plans/active")).unwrap();
    fs::write(
        root.join("custom/skills/beacon/SKILL.md"),
        "---\nname: Custom Skill\ndescription: Custom skillbeacon metadata.\n---\n# Custom\n",
    )
    .unwrap();
    fs::write(
        root.join("custom/plans/active/current.md"),
        "---\ndescription: Custom planbeacon metadata.\n---\n# Current\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--skills"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom/skills/beacon/SKILL.md"))
        .stdout(predicate::str::contains(".agents/skills").not());

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--active-plans"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom/plans/active/current.md"))
        .stdout(predicate::str::contains("docs/exec-plans").not());
}

#[test]
fn list_scope_switches_are_mutually_exclusive_and_reject_targets() {
    let (_temp, root) = fixture_repo("discovery-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--skills", "--plans"])
        .assert()
        .failure();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["list", "--skills", "docs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"))
        .stderr(predicate::str::contains("Usage: docgarden list"));
}

#[test]
fn list_scope_reports_configured_root_that_is_not_a_directory() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("custom")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        "skills-dir = \"custom/skills.md\"\n",
    )
    .unwrap();
    fs::write(root.join("custom/skills.md"), "# Not a directory\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["list", "--skills"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("configured skills-dir path"))
        .stderr(predicate::str::contains("is not a directory"));
}

#[test]
fn list_status_plan_scopes_report_configured_plans_root_that_is_not_a_directory() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("custom")).unwrap();
    fs::write(
        root.join("docgarden.toml"),
        "plans-dir = \"custom/plans\"\n",
    )
    .unwrap();
    fs::write(root.join("custom/plans"), "# Not a directory\n").unwrap();

    for scope in ["--active-plans", "--completed-plans"] {
        Command::new(env!("CARGO_BIN_EXE_docgarden"))
            .current_dir(root)
            .args(["list", scope])
            .assert()
            .failure()
            .stderr(predicate::str::contains("configured plans-dir path"))
            .stderr(predicate::str::contains("is not a directory"));
    }
}

#[test]
fn match_scope_switches_restrict_ranked_corpus() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let skills = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--skills", "skillbeacon"])
        .output()
        .unwrap();
    assert!(skills.status.success());
    let skill_rows = parse_match_rows(&String::from_utf8(skills.stdout).unwrap());
    assert_eq!(skill_rows.len(), 1);
    assert!(
        skill_rows[0]
            .path
            .contains(".agents/skills/skillbeacon/SKILL.md")
    );

    let plans = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args([
            "match",
            "--plans",
            "--path-only",
            "--limit",
            "1",
            "planbeacon",
        ])
        .output()
        .unwrap();
    assert!(plans.status.success());
    let plans_stdout = String::from_utf8(plans.stdout).unwrap();
    assert_eq!(plans_stdout.lines().count(), 1);
    assert!(plans_stdout.contains("docs/exec-plans/active/current.md"));

    let explain = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--plans", "--explain", "planbeacon"])
        .output()
        .unwrap();
    assert!(explain.status.success());
    let explain_stdout = String::from_utf8(explain.stdout).unwrap();
    assert!(explain_stdout.starts_with("score | relative | coverage | path | name | description"));
}

#[test]
fn match_scope_switches_are_mutually_exclusive() {
    let (_temp, root) = fixture_repo("discovery-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--skills", "--plans", "review"])
        .assert()
        .failure();
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
    let rows = parse_match_rows(&stdout);
    let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();

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
fn match_default_limit_returns_top_five_results() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    for n in 1..=6 {
        fs::write(
            root.join(format!("docs/alpha-{n}.md")),
            format!("---\nname: Alpha {n}\ndescription: Alpha result {n}.\n---\n# Alpha {n}\n"),
        )
        .unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "alpha"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let line_count = stdout.lines().count();
    assert_eq!(
        line_count, 5,
        "expected default limit of 5 results, got {line_count}: {stdout}"
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
fn match_color_always_and_never_control_match_styling() {
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
        .args(["match", "--color", "always", "zetasoup"])
        .output()
        .unwrap();
    assert!(low.status.success());
    let low_stdout = String::from_utf8(low.stdout).unwrap();
    assert!(low_stdout.contains("docs/low.md | low | \u{1b}[1mzetasoup\u{1b}[0m"));
    assert!(!low_stdout.contains("\u{1b}[1;31m"));
    assert!(!low_stdout.contains("1.01 |"));

    let medium = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "always", "uniquealpha"])
        .output()
        .unwrap();
    assert!(medium.status.success());
    let medium_stdout = String::from_utf8(medium.stdout).unwrap();
    assert!(medium_stdout.contains("docs/medium.md | \u{1b}[1mUniquealpha\u{1b}[0m | "));
    assert!(!medium_stdout.contains("\u{1b}[1;33m"));

    let high = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "always", "alpha", "beta"])
        .output()
        .unwrap();
    assert!(high.status.success());
    let high_stdout = String::from_utf8(high.stdout).unwrap();
    assert!(high_stdout.contains("\u{1b}[1mAlpha\u{1b}[0m \u{1b}[1mBeta\u{1b}[0m"));
    assert!(!high_stdout.contains("\u{1b}[1;32m"));

    let never = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--color", "never", "alpha", "beta"])
        .output()
        .unwrap();
    assert!(never.status.success());
    let never_stdout = String::from_utf8(never.stdout).unwrap();
    assert!(!never_stdout.contains("\u{1b}["));
    assert!(never_stdout.contains("docs/high.md"));
}

#[test]
fn match_explain_colorizes_scores_by_relative_and_coverage_bands() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    fs::write(
        root.join("docs/top.md"),
        "---\nname: Alpha Beta Gamma Delta\n---\n# Top\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/mid.md"),
        "---\nname: Alpha Beta\n---\n# Mid\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/low.md"),
        "---\ndescription: Alpha\n---\n# Low\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args([
            "match",
            "--explain",
            "--color",
            "always",
            "alpha",
            "beta",
            "gamma",
            "delta",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("score | relative | coverage | path | name | description"));
    assert!(stdout.contains("\u{1b}[1;32m"));
    assert!(stdout.contains("\u{1b}[1;33m"));
    assert!(stdout.contains("\u{1b}[1;31m"));
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
    let hidden = root.join("hidden");
    fs::create_dir_all(&hidden).unwrap();
    fs::write(
        hidden.join("secret-scoring.md"),
        concat!(
            "---\n",
            "name: Hidden Scoring Doc\n",
            "description: This document is inside a gitignored directory and should only appear when --no-gitignore is passed.\n",
            "---\n\n",
            "# Hidden Scoring Doc\n\n",
            "Only visible when gitignore exclusions are disabled.\n",
        ),
    )
    .unwrap();

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

    if fs::read_to_string(&unreadable).is_ok() {
        let mut restore = fs::metadata(&unreadable).unwrap().permissions();
        restore.set_mode(0o644);
        fs::set_permissions(&unreadable, restore).unwrap();
        return;
    }

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
fn match_name_column_falls_back_to_file_stem_when_frontmatter_name_is_missing() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "frontmatter"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout
        .lines()
        .find(|line| line.contains("docs/no-frontmatter.md"))
        .expect("expected no-frontmatter doc in results");
    let rows = parse_match_rows(line);
    assert_eq!(rows[0].name, "no-frontmatter");
}

#[test]
fn match_output_format_has_three_pipe_separated_columns() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "scoring", "--limit", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows = parse_match_rows(&stdout);
    assert!(!rows.is_empty(), "expected at least one result row");
    assert!(!rows[0].path.is_empty());
    assert!(!rows[0].name.is_empty());
    assert!(!rows[0].description.is_empty());
}

#[test]
fn match_explain_output_includes_header_and_diagnostics() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--explain", "scoring", "--limit", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let rows = parse_explain_rows(&stdout);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].score > 0.0);
    assert!(rows[0].relative.ends_with("% of top"));
    assert!(rows[0].coverage.ends_with("terms"));
    assert!(!rows[0].path.is_empty());
    assert!(!rows[0].name.is_empty());
}

#[test]
fn match_default_output_highlights_matching_terms_when_styled() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--color", "always", "review"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\u{1b}[1mReview\u{1b}[0m"));
    assert!(!stdout.contains("\u{1b}[1;"));
}

#[test]
fn match_singular_query_matches_and_highlights_plural_surface_form() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/morphology.md"),
        "---\nname: Release Plans\ndescription: Operational plans archive.\n---\n# Release Plans\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/other.md"),
        "---\nname: Other\ndescription: Unrelated archive.\n---\n# Other\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "plan"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let rows = parse_match_rows(&String::from_utf8(output.stdout).unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "docs/morphology.md");

    let explain = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--explain", "--color", "always", "plan"])
        .output()
        .unwrap();
    assert!(explain.status.success());
    let stdout = String::from_utf8(explain.stdout).unwrap();
    assert!(stdout.contains("docs/morphology.md"));
    assert!(stdout.contains("Release \u{1b}[1mPlans\u{1b}[0m"));
    assert!(stdout.contains("Operational \u{1b}[1mplans\u{1b}[0m archive."));
}

#[test]
fn match_camel_case_name_signal_matches_plan_query() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "plan"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let rows = parse_match_rows(&String::from_utf8(output.stdout).unwrap());
    assert!(
        rows.iter()
            .any(|row| { row.path == "docs/camelcase-only.md" && row.name == "ExecPlan" })
    );
}

#[test]
fn match_explain_highlights_matching_camel_case_half_only() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--explain", "--color", "always", "plan"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Exec\u{1b}[1mPlan\u{1b}[0m"));
    assert!(!stdout.contains("\u{1b}[1mExec\u{1b}[0mPlan"));
}

#[test]
fn match_stopword_only_query_is_rejected() {
    let (_temp, root) = fixture_repo("discovery-repo");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "the"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "query must contain at least one non-stopword term",
        ));
}

#[test]
fn match_routes_review_queries_to_expected_execplan_docs() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let review = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "review"])
        .output()
        .unwrap();
    assert!(review.status.success());
    let review_rows = parse_match_rows(&String::from_utf8(review.stdout).unwrap());
    assert!(review_rows[0].path.contains("evaluator-execplan"));
    assert_eq!(review_rows.len(), 1);

    let review_plan = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args([
            "match",
            "--explain",
            "review",
            "against",
            "the",
            "active",
            "plan",
        ])
        .output()
        .unwrap();
    assert!(review_plan.status.success());
    let review_plan_rows = parse_explain_rows(&String::from_utf8(review_plan.stdout).unwrap());
    assert!(review_plan_rows[0].path.contains("evaluator-execplan"));
    assert!(review_plan_rows[0].score >= review_plan_rows[1].score * 1.5);
}

#[test]
fn match_routes_plan_authoring_and_implementation_queries() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let implement = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args([
            "match",
            "--explain",
            "implement",
            "from",
            "the",
            "active",
            "plan",
        ])
        .output()
        .unwrap();
    assert!(implement.status.success());
    let implement_rows = parse_explain_rows(&String::from_utf8(implement.stdout).unwrap());
    assert!(implement_rows[0].path.contains("generator-execplan"));
    assert!(implement_rows[0].score >= implement_rows[1].score * 1.5);

    let revise = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--explain", "revise", "the", "active", "plan"])
        .output()
        .unwrap();
    assert!(revise.status.success());
    let revise_rows = parse_explain_rows(&String::from_utf8(revise.stdout).unwrap());
    assert!(revise_rows[0].path.contains("planner-execplan"));
    assert!(revise_rows[0].score >= revise_rows[1].score * 1.5);
}

#[test]
fn match_routes_scoring_query_to_scoring_guide() {
    let (_temp, root) = fixture_repo("discovery-repo");

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&root)
        .args(["match", "--explain", "docgarden", "match", "scoring"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let rows = parse_explain_rows(&String::from_utf8(output.stdout).unwrap());
    assert!(rows[0].path.contains("scoring-guide"));
    assert!(rows[0].description.contains("scoring"));
    assert!(rows[0].score >= rows[1].score * 1.5);
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
        .stdout(predicate::str::contains(
            "Fix Markdown lint findings where possible",
        ))
        .stdout(predicate::str::contains("deterministic safe rewrites").not())
        .stdout(predicate::str::contains("init").not())
        .stdout(predicate::str::contains("skill").not())
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
        .stdout(predicate::str::contains("--no-gitignore"))
        .stdout(predicate::str::contains("--color <COLOR>"));

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[TARGETS]..."))
        .stdout(predicate::str::contains("--config <CONFIG>"))
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
    fs::create_dir_all(root.join(PACKAGED_DOCS_DIR)).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join(PACKAGED_STALE_DOC),
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
    fs::create_dir_all(root.join(PACKAGED_DOCS_DIR)).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(
        root.join(PACKAGED_STALE_DOC),
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
    fs::create_dir_all(root.join(PACKAGED_DOCS_DIR)).unwrap();
    fs::write(root.join("docgarden.toml"), "").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(root.join(PACKAGED_STALE_DOC), "[Missing](missing.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", ".", "--no-gitignore", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(PACKAGED_STALE_DOC))
        .stdout(predicate::str::contains("unresolved-link-path"));
}

#[test]
fn config_can_opt_out_of_gitignore_support() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join(PACKAGED_DOCS_DIR)).unwrap();
    fs::write(root.join("docgarden.toml"), "respect-gitignore = false\n").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("docs/guide.md"), "Guide text.\n").unwrap();
    fs::write(root.join(PACKAGED_STALE_DOC), "[Missing](missing.md)\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", ".", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(PACKAGED_STALE_DOC))
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
    fs::write(
        root.join("docs/guide.md"),
        "---\ndescription: A guide.\n---\n\nGuide text.\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(&outside)
        .args(["lint", root.join("docs/guide.md").to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn lint_applies_embedded_default_when_no_config_found() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/no-frontmatter.md"),
        "# Missing description\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", "--color", "never", root.to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicates::str::contains("frontmatter-field-missing"));
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
max-tokens = 1
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error  max-tokens"))
        .stdout(predicate::str::contains("File has"))
        .stdout(predicate::str::contains(
            "which exceeds configured max-tokens = 1.",
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
max-lines = 1
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
        .stdout(predicate::str::contains("warning  max-lines"))
        .stdout(predicate::str::contains(
            "File has 2 lines, which exceeds configured max-lines = 1.",
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
max-tokens = 1
max-lines = 1

[[rules]]
path = "README.md"
disable = ["max-tokens"]
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
        .stdout(predicate::str::contains("max-lines"))
        .stdout(predicate::str::contains("max-tokens").not());
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
max-tokens = 1

[[rules]]
path = "README.md"
max-lines = 1
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
        .stdout(predicate::str::contains("error  max-tokens"))
        .stdout(predicate::str::contains("warning  max-lines"));
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
max-tokens = 1

[[rules]]
path = "README.md"
max-tokens = 1000
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max-tokens").not());
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
max-tokens = 1

[[rules]]
path = "README.md"
disable = ["max-tokens"]
reason = "Temporarily disable token budget."

[[rules]]
path = "README.md"
max-tokens = 1
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "alpha beta gamma\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "README.md", "--color", "never"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error  max-tokens"));
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
max-lines = 1
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
        .stdout(predicate::str::contains("max-lines"))
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
fn folded_strip_frontmatter_description_satisfies_required_field() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("docgarden.toml"),
        r#"
[[rules]]
path = "**/*.md"

[rules.frontmatter]
required = ["description"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("guide.md"),
        "---\ndescription: >-\n  Claude Code generated skill\n  description frontmatter.\n---\n# Guide\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", "guide.md", "--color", "never"])
        .assert()
        .success()
        .stdout(predicate::str::contains("frontmatter-malformed").not())
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
max-chars = 20
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
        .stdout(predicate::str::contains("max-chars = 20"));
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
max-chars = 20
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

#[test]
fn match_path_only_and_explain_conflict() {
    // --path-only and --explain are mutually exclusive output modes.
    // clap must reject the combination at parse time, before any repository
    // walk or match computation occurs.
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("README.md"), "# Repo\n").unwrap();

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--path-only", "--explain", "anything"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ---------------------------------------------------------------------------
// match-only exclusion integration tests
// ---------------------------------------------------------------------------

#[test]
fn match_only_exclusion_filters_source_skills() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("skills/builder/SKILL.md").parent().unwrap()).unwrap();
    fs::create_dir_all(
        root.join(".agents/skills/builder/SKILL.md")
            .parent()
            .unwrap(),
    )
    .unwrap();

    fs::write(
        root.join("docgarden.toml"),
        r#"
[match]
exclude = ["skills/**"]
"#,
    )
    .unwrap();

    fs::write(
        root.join("skills/builder/SKILL.md"),
        "---\nname: skill-builder\ndescription: A skill builder tool.\n---\n# Skill Builder\n",
    )
    .unwrap();
    fs::write(
        root.join(".agents/skills/builder/SKILL.md"),
        "---\nname: agent-builder\ndescription: Agent-local builder skill.\n---\n# Agent Builder\n",
    )
    .unwrap();

    // Broad match should exclude skills/builder/ and include .agents/skills/builder/
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "builder"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.lines().all(|line| !line.starts_with("skills/")),
        "broad match should exclude source skills/: {stdout}"
    );
    assert!(
        stdout.contains(".agents/skills/builder/SKILL.md"),
        "broad match should include live agent skills/: {stdout}"
    );
}

#[test]
fn match_skills_scope_uses_live_agent_skills_dir() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("skills/builder/SKILL.md").parent().unwrap()).unwrap();
    fs::create_dir_all(
        root.join(".agents/skills/builder/SKILL.md")
            .parent()
            .unwrap(),
    )
    .unwrap();

    fs::write(
        root.join("docgarden.toml"),
        r#"
[match]
exclude = ["skills/**"]
"#,
    )
    .unwrap();

    fs::write(
        root.join("skills/builder/SKILL.md"),
        "---\nname: skill-builder\ndescription: A skill builder tool.\n---\n# Skill Builder\n",
    )
    .unwrap();
    fs::write(
        root.join(".agents/skills/builder/SKILL.md"),
        "---\nname: agent-builder\ndescription: Agent-local builder skill.\n---\n# Agent Builder\n",
    )
    .unwrap();

    // --skills scope uses configured skills-dir (default: .agents/skills)
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--skills", "builder"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("agent-builder"),
        "--skills should find live agent skills: {stdout}"
    );
    assert!(
        !stdout.contains("skill-builder"),
        "--skills should exclude source skills: {stdout}"
    );
}

#[test]
fn match_scopes_ignore_match_only_exclusion() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("skills/builder/SKILL.md").parent().unwrap()).unwrap();
    fs::create_dir_all(root.join("docs/exec-plans/active")).unwrap();

    fs::write(
        root.join("docgarden.toml"),
        r#"
[match]
exclude = ["skills/**", "docs/**"]
"#,
    )
    .unwrap();

    fs::write(
        root.join("skills/builder/SKILL.md"),
        "---\nname: skill-builder\ndescription: A skill builder tool.\n---\n# Skill Builder\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/exec-plans/active/plan-alpha.md"),
        "---\nname: Plan Alpha\ndescription: An active plan.\n---\n# Plan Alpha\n",
    )
    .unwrap();

    // --plans scope should ignore [match].exclude entirely
    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["match", "--plans", "plan"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("docs/exec-plans/active/plan-alpha.md"),
        "--plans scope should bypass match.exclude for docs: {stdout}"
    );
}

#[test]
fn lint_still_scans_source_and_generated_skills() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("skills/builder/SKILL.md").parent().unwrap()).unwrap();
    fs::create_dir_all(
        root.join(".agents/skills/builder/SKILL.md")
            .parent()
            .unwrap(),
    )
    .unwrap();

    fs::write(
        root.join("docgarden.toml"),
        r#"
[match]
exclude = ["skills/**"]

[[rules]]
path = "*.md"

[rules.frontmatter.fields.description]
max-chars = 20
"#,
    )
    .unwrap();

    // Both files have descriptions exceeding max-chars = 20
    let long_desc = "This description is way longer than twenty characters.";
    fs::write(
        root.join("skills/builder/SKILL.md"),
        format!("---\nname: builder\ndescription: {long_desc}\n---\n# Skill Builder\n"),
    )
    .unwrap();
    fs::write(
        root.join(".agents/skills/builder/SKILL.md"),
        format!("---\nname: agent-builder\ndescription: {long_desc}\n---\n# Agent Builder\n"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .current_dir(root)
        .args(["lint", ".", "--color", "never"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("skills/builder/SKILL.md"),
        "lint should scan source skills/: {stdout}"
    );
    assert!(
        stdout.contains(".agents/skills/builder/SKILL.md"),
        "lint should scan generated skills/: {stdout}"
    );
}
