use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;

fn init_git_project(path: &Path) {
    fs::create_dir_all(path).unwrap();
    Command::new("git").arg("init").current_dir(path).output().unwrap();
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(&["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    fs::write(path.join("README"), "x").unwrap();
    Command::new("git").args(&["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(&["commit", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn add_worktree(project: &Path, branch: &str, target: &Path) {
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(&["worktree", "add", "-b", branch])
        .arg(target)
        .output()
        .expect("Failed to add worktree");
}

fn current_branch(path: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(&["symbolic-ref", "--short", "HEAD"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn test_worktree_list_marks_root_with_branch_and_asterisk() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());
    let branch = current_branch(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("* {}", branch)))
        .stdout(predicate::str::contains("feature-x"))
        .stdout(predicate::str::contains("__root").not());
}

#[test]
fn test_worktree_search_by_root_branch_does_not_prompt() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());
    let branch = current_branch(project.path());

    // Search for the root branch — should resolve to root without prompting.
    // The "create worktree?" prompt goes to stderr, so empty stderr proves the
    // root-branch shortcut fired before the create-prompt path.
    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg(&branch)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // No worktree should have been created
    let worktrees_dir = workspace.child("org/.worktrees");
    if worktrees_dir.exists() {
        let count = fs::read_dir(worktrees_dir.path()).unwrap().count();
        assert_eq!(count, 0, "no worktree should have been created");
    }
}

#[test]
fn test_worktree_root_with_project_arg() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-W")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_worktree_root_resolves_from_cwd() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());

    // Run from inside the worktree — -W should resolve back to the project root
    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .current_dir(wt_path.path())
        .arg("-W")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_worktree_root_outside_workspace_errors() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    // Outside the workspace and no project arg → cannot determine project
    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .current_dir(temp_dir.path())
        .arg("-W")
        .assert()
        .failure();
}

#[test]
fn test_remove_clean_worktree() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());
    assert!(wt_path.exists());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg("feature-x")
        .arg("--rm")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed worktree"));

    assert!(!wt_path.exists());
}

#[test]
fn test_remove_worktree_via_rm_value() {
    // Same removal but with the name on --rm and -w bare
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("--rm")
        .arg("feature-x")
        .arg("-w")
        .assert()
        .success();

    assert!(!wt_path.exists());
}

#[test]
fn test_remove_dirty_worktree_without_yes_declines() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());
    fs::write(wt_path.path().join("untracked.txt"), "x").unwrap();

    // assert_cmd closes stdin → read_line returns EOF → empty answer → declined
    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg("feature-x")
        .arg("--rm")
        .assert()
        .success()
        .stderr(predicate::str::contains("uncommitted"));

    assert!(wt_path.exists(), "dirty worktree should not be removed without confirmation");
}

#[test]
fn test_remove_dirty_worktree_with_yes_removes() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());
    fs::write(wt_path.path().join("untracked.txt"), "x").unwrap();

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg("feature-x")
        .arg("--rm")
        .arg("-y")
        .assert()
        .success();

    assert!(!wt_path.exists());
}

#[test]
fn test_remove_nonexistent_worktree_errors() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg("nope")
        .arg("--rm")
        .assert()
        .failure();
}

#[test]
fn test_worktree_search_existing_worktree_succeeds() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    let project = workspace.child("org/proj");
    init_git_project(project.path());

    let wt_path = workspace.child("org/.worktrees/proj/feature-x");
    add_worktree(project.path(), "feature-x", wt_path.path());

    Command::cargo_bin("wok")
        .unwrap()
        .env("WOK_SPACE", workspace.path())
        .arg("org/proj")
        .arg("-w")
        .arg("feature-x")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
