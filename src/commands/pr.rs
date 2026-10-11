use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use wok::lib::constants::GOTO_MARKER;
use wok::lib::fd::write_to_fd;
use wok::lib::git::{
    create_worktree, fetch_pr_branch, fetch_pr_head, is_worktree_dirty, list_worktrees,
    merge_fetch_head, reset_hard,
};

use crate::commands::add;
use crate::commands::go_worktree::resolve_project_path;

/// `wok --pr <number> [project]`: review a GitHub pull request of an
/// existing project (positional project name, or the current directory's
/// project) in a dedicated worktree.
pub fn handle(
    workspace: &str,
    project: Option<&str>,
    number: u32,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, project)?;
    review_pr(&project_path, number, force)
}

/// `wok <pr-url>`: the same, but the URL tells us org, project and PR
/// number; the project is cloned into the workspace first when missing.
pub fn handle_url(
    workspace: &str,
    org: &str,
    repo: &str,
    number: u32,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = PathBuf::from(format!("{}/{}/{}", workspace, org, repo));
    if !project_path.exists() {
        let repo_url = format!("https://github.com/{}/{}.git", org, repo);
        add::handle(workspace, &repo_url)?;
    }
    review_pr(&project_path, number, force)
}

/// Ensure a `pr-<number>` worktree exists for the project, on the PR head,
/// and jump into it (FD3 goto).
///
/// - New: fetch `pull/<n>/head` into local branch `pr-<n>` and check it out
///   in a worktree (no `gh` dependency; `refs/pull` works for fork PRs too,
///   since the refs live on the base repository). GitHub-only ref namespace.
/// - Existing: fetch the PR head and merge it (fast-forward when possible);
///   a conflict aborts the merge and returns an error. A dirty worktree
///   prompts for a hard reset; `--force` resets hard without asking.
fn review_pr(
    project_path: &Path,
    number: u32,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let worktree_name = format!("pr-{}", number);
    let canonical_project = project_path
        .canonicalize()
        .unwrap_or(project_path.to_path_buf());

    let worktrees: Vec<_> = list_worktrees(project_path)?
        .into_iter()
        .filter(|wt| wt.path.canonicalize().unwrap_or(wt.path.clone()) != canonical_project)
        .collect();

    if let Some(wt) = worktrees.iter().find(|wt| wt.name == worktree_name) {
        // Existing PR worktree: pull the PR head in. The fetch runs from
        // inside the worktree so FETCH_HEAD lands in the worktree's own
        // gitdir, where the merge/reset below reads it.
        fetch_pr_head(&wt.path, number)?;

        let dirty = is_worktree_dirty(&wt.path).unwrap_or(false);
        if dirty || force {
            if dirty && !force && !confirm_discard_and_reset(&worktree_name)? {
                return Ok(());
            }
            reset_hard(&wt.path, "FETCH_HEAD")?;
        } else {
            merge_fetch_head(&wt.path)?;
        }
        goto(&wt.path)
    } else {
        // New PR worktree: fetch the PR head into a local branch and check
        // it out at the usual opaque path
        fetch_pr_branch(project_path, number, &worktree_name)?;
        let path = create_worktree(project_path, &worktree_name, &[worktree_name.clone()])?;
        goto(&path)
    }
}

fn confirm_discard_and_reset(worktree_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    eprint!(
        "Worktree '{}' has uncommitted or untracked changes. Discard them and update to the PR head? [y/N] ",
        worktree_name
    );
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn goto(dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let goto_dest = format!("{}{}", GOTO_MARKER, dest.to_string_lossy());
    write_to_fd(3, &goto_dest)?;
    if is_tty() {
        println!("{}", dest.to_string_lossy());
    }
    Ok(())
}

fn is_tty() -> bool {
    unsafe { libc::isatty(std::io::stdout().as_raw_fd()) != 0 }
}
