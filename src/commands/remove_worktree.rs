use std::io::{self, Write};
use wok::lib::git::{is_worktree_dirty, list_worktrees, remove_worktree};

use crate::commands::go_worktree::resolve_project_path;

pub fn handle(
    workspace: &str,
    project: Option<&str>,
    name: &str,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, project)?;
    let canonical_project = project_path.canonicalize().unwrap_or(project_path.clone());

    let worktrees = list_worktrees(&project_path)?;
    let target = worktrees
        .iter()
        .find(|wt| {
            wt.name == name
                && wt.path.canonicalize().unwrap_or(wt.path.clone()) != canonical_project
        })
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            Box::from(format!("Worktree '{}' not found.", name))
        })?;

    let dirty = is_worktree_dirty(&target.path).unwrap_or(false);
    let force = if dirty {
        if !yes && !confirm_dirty_removal(name)? {
            return Ok(());
        }
        true
    } else {
        false
    };

    remove_worktree(&project_path, &target.path, force)?;
    println!("Removed worktree '{}'.", name);
    Ok(())
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
