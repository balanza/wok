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
