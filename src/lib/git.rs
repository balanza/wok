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
}
