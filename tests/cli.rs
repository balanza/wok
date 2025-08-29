use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: wok [OPTIONS] [PROJECT]"));
}

#[test]
fn test_cli_add_existing_repo_ssh() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir.child("workspace").create_dir_all().unwrap();
    let workspace = temp_dir.path().join("workspace");

    let mut cmd = Command::cargo_bin("wok").unwrap();

    // Forward SSH agent
    if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
        cmd.env("SSH_AUTH_SOCK", sock);
    }

    cmd.env("WOK_SPACE", workspace)
        .arg("git@github.com:dottorblaster/stocazzo.git")
        .assert()
        .success();

    let repo_path = temp_dir
        .path()
        .join("workspace")
        .join("dottorblaster")
        .join("stocazzo");
    assert!(temp_dir.child(repo_path).exists());
}

#[test]
fn test_cli_add_existing_repo_https() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir.child("workspace").create_dir_all().unwrap();
    let workspace = temp_dir.path().join("workspace");

    let mut cmd = Command::cargo_bin("wok").unwrap();

    // Forward SSH agent
    if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
        cmd.env("SSH_AUTH_SOCK", sock);
    }

    cmd.env("WOK_SPACE", workspace)
        .arg("https://github.com/dottorblaster/stocazzo.git")
        .assert()
        .success();

    let repo_path = temp_dir
        .path()
        .join("workspace")
        .join("dottorblaster")
        .join("stocazzo");
    assert!(temp_dir.child(repo_path).exists());
}

#[test]
fn test_cli_add_non_existing_repo() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir.child("workspace").create_dir_all().unwrap();
    let workspace = temp_dir.path().join("workspace");

    let mut cmd = Command::cargo_bin("wok").unwrap();

    // Forward SSH agent
    if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
        cmd.env("SSH_AUTH_SOCK", sock);
    }

    cmd.env("WOK_SPACE", workspace)
        .arg("git@github.com:balanza/unknown.git")
        .assert()
        .failure();
}
