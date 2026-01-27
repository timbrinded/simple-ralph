//! CLI integration tests using assert_cmd

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn ralph_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("ralph").unwrap()
}

#[test]
fn cli_no_args_shows_help() {
    ralph_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"))
        .stderr(predicate::str::contains("ralph"));
}

#[test]
fn cli_help_flag() {
    ralph_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI-powered PRD execution"))
        .stdout(predicate::str::contains("build"))
        .stdout(predicate::str::contains("plan"));
}

#[test]
fn cli_version_flag() {
    ralph_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ralph"))
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn cli_build_help() {
    ralph_cmd()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Execute tasks from an existing PRD",
        ))
        .stdout(predicate::str::contains("--prd-path"));
}

#[test]
fn cli_plan_help() {
    ralph_cmd()
        .args(["plan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate a new PRD"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--resume"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn cli_build_nonexistent_prd_fails() {
    ralph_cmd()
        .args(["build", "--prd-path", "/nonexistent/path/prd.json"])
        .assert()
        .failure();
}

#[test]
fn cli_build_invalid_prd_fails() {
    let temp_dir = TempDir::new().unwrap();
    let prd_path = temp_dir.path().join("invalid.json");
    std::fs::write(&prd_path, "not valid json {{{").unwrap();

    ralph_cmd()
        .args(["build", "--prd-path", prd_path.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn cli_plan_output_flag_accepted() {
    // Just test that the flag is recognized - actual execution would require Claude
    ralph_cmd()
        .args(["plan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-o, --output"));
}

#[test]
fn cli_plan_resume_flag_accepted() {
    ralph_cmd()
        .args(["plan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-r, --resume"));
}

#[test]
fn cli_plan_force_flag_accepted() {
    ralph_cmd()
        .args(["plan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-f, --force"));
}

#[test]
fn cli_plan_description_flag_accepted() {
    ralph_cmd()
        .args(["plan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-d, --description"));
}

#[test]
fn cli_invalid_subcommand_fails() {
    ralph_cmd()
        .arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn cli_build_max_loops_flag() {
    ralph_cmd()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-l, --max-loops"));
}
