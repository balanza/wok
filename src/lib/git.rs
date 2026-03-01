use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Represents a discovered git repository
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredRepo {
    pub path: PathBuf,
    pub org: String,
    pub name: String,
    pub remote: String,
}

/// Recursively discover git repositories in a directory
pub fn discover_repositories(root_path: &Path) -> Result<Vec<DiscoveredRepo>, Box<dyn std::error::Error>> {
    let mut repos = Vec::new();
    discover_repositories_recursive(root_path, &mut repos)?;
    Ok(repos)
}

fn discover_repositories_recursive(
    path: &Path,
    repos: &mut Vec<DiscoveredRepo>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if this is a git repository
    let git_dir = path.join(".git");
    if git_dir.exists() && git_dir.is_dir() {
        // This is a git repository, try to get the remote
        if let Some(repo) = extract_repo_info(path)? {
            repos.push(repo);
        }
        // Don't recurse into subdirectories of a git repo
        return Ok(());
    }

    // Not a git repo, continue searching subdirectories
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    // Skip hidden directories (except .git which we already handled)
                    if let Some(name) = entry_path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with('.') {
                                continue;
                            }
                        }
                    }
                    // Recursively search this directory
                    discover_repositories_recursive(&entry_path, repos)?;
                }
            }
        }
    }

    Ok(())
}

/// Extract repository information from a git directory
fn extract_repo_info(repo_path: &Path) -> Result<Option<DiscoveredRepo>, Box<dyn std::error::Error>> {
    // Get the remote URL
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output();

    let remote_url = match output {
        Ok(out) if out.status.success() => {
            String::from_utf8(out.stdout)?.trim().to_string()
        }
        _ => {
            // No remote or error getting remote
            return Ok(None);
        }
    };

    // Skip if remote is empty
    if remote_url.is_empty() {
        return Ok(None);
    }

    // Parse the remote URL to extract org/name
    let (org, name) = parse_git_remote(&remote_url)?;

    Ok(Some(DiscoveredRepo {
        path: repo_path.to_path_buf(),
        org,
        name,
        remote: remote_url,
    }))
}

/// Parse a git remote URL to extract organization and repository name
/// Supports:
/// - https://github.com/org/repo.git
/// - git@github.com:org/repo.git
/// - ssh://git@github.com/org/repo.git
/// Represents a git worktree
#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeItem {
    pub path: PathBuf,
    pub name: String,
}

/// Parse the output of `git worktree list --porcelain` into WorktreeItem entries.
/// Skips entries whose directory does not exist on disk (prunable/stale).
pub fn parse_worktree_list(output: &str) -> Vec<WorktreeItem> {
    let mut items = Vec::new();
    let mut current_path: Option<PathBuf> = None;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            current_path = Some(PathBuf::from(&line["worktree ".len()..]));
        } else if line.is_empty() {
            if let Some(path) = current_path.take() {
                if path.exists() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    items.push(WorktreeItem { path, name });
                }
            }
        }
    }

    // Handle last entry (output may not end with a blank line)
    if let Some(path) = current_path.take() {
        if path.exists() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            items.push(WorktreeItem { path, name });
        }
    }

    items
}

/// List all worktrees for a git project
pub fn list_worktrees(project_path: &Path) -> Result<Vec<WorktreeItem>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(&["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Box::from(format!("git worktree list failed: {}", stderr)));
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(parse_worktree_list(&stdout))
}

/// Create a new worktree for a git project.
/// The worktree is created at `{org_path}/.worktrees/{project_name}/{worktree_name}`,
/// where `org_path` is the parent of `project_path`.
/// Extra args are passed directly to `git worktree add`.
/// Returns the path to the newly created worktree directory.
pub fn create_worktree(
    project_path: &Path,
    worktree_name: &str,
    extra_args: &[String],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let org_path = project_path
        .parent()
        .ok_or("Could not determine organization directory")?;
    let project_name = project_path
        .file_name()
        .ok_or("Could not determine project name")?;
    let target_dir = org_path.join(".worktrees").join(project_name).join(worktree_name);

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(project_path)
        .args(&["worktree", "add"])
        .arg(&target_dir);

    for arg in extra_args {
        cmd.arg(arg);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Box::from(format!("git worktree add failed: {}", stderr)));
    }

    Ok(target_dir)
}

fn parse_git_remote(remote_url: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    use git_url_parse::GitUrl;

    let parsed = GitUrl::parse(remote_url)?;

    let org = parsed
        .owner
        .ok_or("No organization found in remote URL")?;

    let name = parsed.name;

    Ok((org, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_remote_https() {
        let (org, name) = parse_git_remote("https://github.com/myorg/myrepo.git").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(name, "myrepo");
    }

    #[test]
    fn test_parse_git_remote_ssh() {
        let (org, name) = parse_git_remote("git@github.com:myorg/myrepo.git").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(name, "myrepo");
    }

    #[test]
    fn test_parse_git_remote_ssh_url() {
        let (org, name) = parse_git_remote("ssh://git@github.com/myorg/myrepo.git").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(name, "myrepo");
    }

    #[test]
    fn test_parse_git_remote_no_git_extension() {
        let (org, name) = parse_git_remote("https://github.com/myorg/myrepo").unwrap();
        assert_eq!(org, "myorg");
        assert_eq!(name, "myrepo");
    }

    #[test]
    fn test_parse_worktree_list_empty() {
        let items = parse_worktree_list("");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_worktree_list_with_existing_paths() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let main_path = temp_dir.path().join("main");
        let feature_path = temp_dir.path().join("feature");
        std::fs::create_dir(&main_path).unwrap();
        std::fs::create_dir(&feature_path).unwrap();

        let output = format!(
            "worktree {}\nHEAD abc123\nbranch refs/heads/main\n\nworktree {}\nHEAD def456\nbranch refs/heads/feature\n\n",
            main_path.display(),
            feature_path.display()
        );

        let items = parse_worktree_list(&output);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "main");
        assert_eq!(items[0].path, main_path);
        assert_eq!(items[1].name, "feature");
        assert_eq!(items[1].path, feature_path);
    }

    #[test]
    fn test_parse_worktree_list_skips_nonexistent_paths() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let existing_path = temp_dir.path().join("existing");
        std::fs::create_dir(&existing_path).unwrap();
        let nonexistent_path = temp_dir.path().join("gone");

        let output = format!(
            "worktree {}\nHEAD abc123\nbranch refs/heads/main\n\nworktree {}\nHEAD def456\nbranch refs/heads/gone\nprunable gitdir file points to non-existent location\n\n",
            existing_path.display(),
            nonexistent_path.display()
        );

        let items = parse_worktree_list(&output);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "existing");
    }

    #[test]
    fn test_parse_worktree_list_bare() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let bare_path = temp_dir.path().join("bare-repo");
        std::fs::create_dir(&bare_path).unwrap();

        let output = format!(
            "worktree {}\nHEAD abc123\nbare\n\n",
            bare_path.display()
        );

        let items = parse_worktree_list(&output);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "bare-repo");
    }

    #[test]
    fn test_parse_worktree_list_no_trailing_newline() {
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let path = temp_dir.path().join("wt");
        std::fs::create_dir(&path).unwrap();

        let output = format!(
            "worktree {}\nHEAD abc123\nbranch refs/heads/main",
            path.display()
        );

        let items = parse_worktree_list(&output);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "wt");
    }
}
