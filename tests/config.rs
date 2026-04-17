use assert_cmd::Command;
use predicates::prelude::*;

mod common;

use common::fixture_repo;

#[test]
fn json_output_is_uncolored() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args([
            "lint",
            root.to_str().unwrap(),
            "--json",
            "--color",
            "always",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "\"rule\": \"prefer-links-for-local-paths\"",
        ))
        .stdout(predicate::str::contains("\u{1b}").not());
}

#[test]
fn color_always_forces_ansi_in_human_output() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args(["lint", root.to_str().unwrap(), "--color", "always"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\u{1b}[31merror\u{1b}[0m"));
}

#[test]
fn explicit_config_path_is_echoed_in_fix_hint() {
    let (_temp, root) = fixture_repo("backticks");

    Command::new(env!("CARGO_BIN_EXE_docgarden"))
        .args([
            "lint",
            root.to_str().unwrap(),
            "--config",
            root.join("docgarden.toml").to_str().unwrap(),
            "--color",
            "never",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("--config docgarden.toml"));
}
