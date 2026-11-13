use std::io::{self, Write};
use std::path::Path;
use wok::lib::constants::GOTO_MARKER;
use wok::lib::fd::write_to_fd;
use wok::lib::git::{
    delete_branch, is_worktree_dirty, list_worktrees, remove_worktree, WorktreeItem,
};

use crate::commands::go_worktree::resolve_project_path;

/// `wok -w <name> --rm`: remove a named worktree of the given (or current)
/// project.
pub fn handle(
    workspace: &str,
    project: Option<&str>,
    name: &str,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, project)?;
    let target = linked_worktrees(&project_path)?
        .into_iter()
        .find(|wt| wt.name == name)
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            Box::from(format!("Worktree '{}' not found.", name))
        })?;

    remove_one(&project_path, &target, yes)?;
    Ok(())
}

/// `wok -w --rm`: remove the worktree containing the current directory, then
/// send the shell back to the project root (the cwd no longer exists).
pub fn handle_current(workspace: &str, yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, None)?;
    let cwd = std::env::current_dir()?.canonicalize()?;

    let target = linked_worktrees(&project_path)?
        .into_iter()
        .find(|wt| cwd.starts_with(canonical(&wt.path)))
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            Box::from(
                "Not inside a worktree: run from a worktree, or name it (wok -w <name> --rm).",
            )
        })?;

    if remove_one(&project_path, &target, yes)? {
        let goto_dest = format!("{}{}", GOTO_MARKER, project_path.to_string_lossy());
        write_to_fd(3, &goto_dest)?;
    }
    Ok(())
}

/// All worktrees of the project except the main checkout.
fn linked_worktrees(project_path: &Path) -> Result<Vec<WorktreeItem>, Box<dyn std::error::Error>> {
    let canonical_project = canonical(project_path);
    Ok(list_worktrees(project_path)?
        .into_iter()
        .filter(|wt| canonical(&wt.path) != canonical_project)
        .collect())
}

/// Remove one worktree, prompting first when it is dirty (unless `yes`).
/// Returns whether it was actually removed.
fn remove_one(
    project_path: &Path,
    target: &WorktreeItem,
    yes: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let name = &target.name;
    let dirty = is_worktree_dirty(&target.path).unwrap_or(false);
    let force = if dirty {
        if !yes && !confirm_dirty_removal(name)? {
            return Ok(false);
        }
        true
    } else {
        false
    };

    remove_worktree(project_path, &target.path, force)?;

    // PR worktrees carry their own local branch (pr-<n>); clean it up too.
    // Regular worktrees keep their branch, as before.
    if let Some(num) = name.strip_prefix("pr-") {
        if num.parse::<u32>().is_ok() {
            delete_branch(project_path, name)?;
        }
    }

    println!("Removed worktree '{}'.", name);
    Ok(true)
}

fn canonical(path: &Path) -> std::path::PathBuf {
    path.canonicalize().unwrap_or(path.to_path_buf())
}

fn confirm_dirty_removal(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    eprint!(
        "Worktree '{}' has uncommitted or untracked changes. Remove anyway? [y/N] ",
        name
    );
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}
