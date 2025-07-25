use git_url_parse::GitUrl;
use std::process::{Command, Stdio};
use wok::lib::spinner;

pub fn handle(workspace: &str, repo_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some((org_name, repo_name)) = parse_repo_url(repo_url) {
        let destination = format!("{}/{}/{}", workspace, org_name, repo_name);

        let spinner = spinner::Spinner::new(format!("Cloning into {}", destination));

        match Command::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                spinner.success(format!("Project created successfully at: {}", destination));
                Ok(())
            }
            Ok(status) => {
                spinner.error(format!("Project creation failed"));
                Err(format!(
                    "Failed to clone repository. Command exited with status: {}",
                    status
                )
                .into())
            }

            Err(e) => {
                spinner.error(format!("Project creation failed"));
                Err(format!("Failed to execute git clone command: {}", e).into())
            }
        }
    } else {
        Err(format!("Invalid repository URL: {}", repo_url).into())
    }
}

// parse a repository URL and return the organization name and repository name
// the repository can come in various formats, e.g.:
// - git@github.com:balanza/wok.git
// - https://github.com/balanza/wok.git
// In both cases, we want to extract "balanza" and "wok"
fn parse_repo_url(repo_url: &str) -> Option<(String, String)> {
    let parsed_url = GitUrl::parse(repo_url).ok()?;

    if parsed_url.fullname.is_empty() {
        return None;
    }

    let (org_name, repo_name) = parsed_url.fullname.split_once("/").unwrap_or_default();
    return Some((org_name.to_string(), repo_name.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_url() {
        assert_eq!(
            parse_repo_url("https://github.com/balanza/wok.git"),
            Some(("balanza".to_string(), "wok".to_string()))
        );
        assert_eq!(
            parse_repo_url("git@github.com:balanza/wok.git"),
            Some(("balanza".to_string(), "wok".to_string()))
        );
        assert_eq!(parse_repo_url("invalid_url"), None);
    }

    #[test]
    fn test_git_url_parse() {
        let url = "foo";
        let parsed = GitUrl::parse(url);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.fullname, "");
        assert_eq!(parsed.host, None);
        assert_eq!(parsed.path, "foo/");
        assert_eq!(parsed.port, None);
    }
}
