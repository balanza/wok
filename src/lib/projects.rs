use serde::Serialize;
use std::thread;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub organization: String,
    pub remote: Option<String>,
    pub is_current: bool,
}

// Generates a unique project ID in the form "organization::project" to preserve alphabetical order.
pub fn project_id(org: &str, project: &str) -> String {
    format!("{}::{}", org, project)
}

// Lists all projects in the workspace, fetching their details concurrently.
// Returns a vector of ProjectItem, which includes the project name, organization, remote URL,
// and whether it is the current project.
// Uses threads to fetch project details concurrently for better performance.
pub fn list_project_items(
    workspace: &str,
    filter_orgs: Vec<String>,
) -> Result<Vec<ProjectItem>, Box<dyn std::error::Error>> {
    let projects = list_projects(workspace, filter_orgs)?;
    let mut project_items = vec![None::<ProjectItem>; projects.len()];

    thread::scope(|scope| {
        // mantain a single mutable reference to the project items
        let mut remaining = &mut project_items[..];

        for (org, project) in projects {
            let (item_slot, rest) = remaining.split_first_mut().unwrap();
            remaining = rest;

            scope.spawn(move || {
                let item = get_project_item(workspace, &org, &project);
                match item {
                    Ok(item) => *item_slot = item,
                    Err(e) => eprintln!("Error getting project item for {}: {}", project, e),
                }
            });
        }
    });

    let result: Vec<ProjectItem> = project_items.into_iter().flatten().collect();

    Ok(result)
}

// Lists all projects in the given workspace, grouped by organization.
// Organizations are directories within the workspace, and projects are subdirectories within each organization.
// Returns a list of (organization, project name) pairs.
// The `filter_orgs` parameter allows filtering by specific organizations.
fn list_projects(
    workspace: &str,
    filter_orgs: Vec<String>,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    // list all orgs as directories in the workspace
    let orgs = std::fs::read_dir(workspace)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
        .filter(|entry| {
            let org_name = entry.file_name().to_string_lossy().to_string();
            filter_orgs.is_empty() || filter_orgs.contains(&org_name)
        })
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect::<Vec<String>>();

    let mut projects = Vec::new();

    for org in orgs {
        let org_path = format!("{}/{}", workspace, org);
        std::fs::read_dir(org_path)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
            .map(|entry| entry.file_name().into_string().unwrap())
            .for_each(|entry| {
                projects.push((org.clone(), entry));
            });
    }

    Ok(projects)
}

// Given a tuple (organization, project name), returns an Option<ProjectItem>.
// Returns None if the project is not a git repository.
// Fields:
// - name: project name
// - remote: remote URL if a git repository, otherwise None
// - is_current: true if this is the current git repository, false otherwise
fn get_project_item(
    workspace: &str,
    org: &str,
    project: &str,
) -> Result<Option<ProjectItem>, Box<dyn std::error::Error>> {
    let project_path = format!("{}/{}/{}", workspace, org, project);
    let git_path = format!("{}/.git", project_path);
    if !std::path::Path::new(&git_path).exists() {
        return Ok(None);
    }

    let remote = std::process::Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(&project_path)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .map(Some)
        .unwrap_or_else(|_| None);

    // check if the current directory is same os a sub directory of the project path
    let is_current = std::env::current_dir()
        .and_then(|current| {
            let current = current.canonicalize()?;
            let project = std::path::Path::new(&project_path).canonicalize()?;
            Ok(current == project || current.starts_with(&project))
        })
        .unwrap_or(false);

    Ok(Some(ProjectItem {
        id: project_id(org, project),
        organization: org.to_string(),
        name: project.to_string(),
        remote,
        is_current,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    // Helper function to create a test workspace with projects
    fn setup_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create org1 with two projects
        let org1 = workspace.join("org1");
        fs::create_dir(&org1).unwrap();

        let project1 = org1.join("project1");
        fs::create_dir(&project1).unwrap();
        init_git_repo(&project1, "https://github.com/org1/project1.git");

        let project2 = org1.join("project2");
        fs::create_dir(&project2).unwrap();
        init_git_repo(&project2, "https://github.com/org1/project2.git");

        // Create org2 with one project
        let org2 = workspace.join("org2");
        fs::create_dir(&org2).unwrap();

        let project3 = org2.join("project3");
        fs::create_dir(&project3).unwrap();
        init_git_repo(&project3, "https://github.com/org2/project3.git");

        // Create a non-git directory (should be ignored)
        let project4 = org2.join("not-a-repo");
        fs::create_dir(&project4).unwrap();

        temp_dir
    }

    // Helper to initialize a git repository with a remote
    fn init_git_repo(path: &PathBuf, remote_url: &str) {
        Command::new("git")
            .arg("init")
            .current_dir(path)
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(&["remote", "add", "origin", remote_url])
            .current_dir(path)
            .output()
            .expect("Failed to add remote");
    }

    #[test]
    fn test_project_id() {
        assert_eq!(project_id("org1", "project1"), "org1::project1");
        assert_eq!(project_id("my-org", "my-project"), "my-org::my-project");
        assert_eq!(project_id("", "project"), "::project");
    }

    #[test]
    fn test_list_projects() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Test listing all projects
        let projects = list_projects(workspace, vec![]).unwrap();
        assert_eq!(projects.len(), 4); // 2 from org1, 2 from org2 (including non-git)

        // Check that we have the expected projects
        assert!(projects.contains(&("org1".to_string(), "project1".to_string())));
        assert!(projects.contains(&("org1".to_string(), "project2".to_string())));
        assert!(projects.contains(&("org2".to_string(), "project3".to_string())));
        assert!(projects.contains(&("org2".to_string(), "not-a-repo".to_string())));
    }

    #[test]
    fn test_list_projects_with_filter() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Test filtering by organization
        let projects = list_projects(workspace, vec!["org1".to_string()]).unwrap();
        assert_eq!(projects.len(), 2);
        assert!(projects.iter().all(|(org, _)| org == "org1"));

        // Test filtering by multiple organizations
        let projects =
            list_projects(workspace, vec!["org1".to_string(), "org2".to_string()]).unwrap();
        assert_eq!(projects.len(), 4);

        // Test filtering by non-existent organization
        let projects = list_projects(workspace, vec!["org3".to_string()]).unwrap();
        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_get_project_item() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Test getting a valid git project
        let item = get_project_item(workspace, "org1", "project1").unwrap();
        assert!(item.is_some());

        let item = item.unwrap();
        assert_eq!(item.id, "org1::project1");
        assert_eq!(item.organization, "org1");
        assert_eq!(item.name, "project1");
        assert_eq!(
            item.remote,
            Some("https://github.com/org1/project1.git".to_string())
        );
        assert!(!item.is_current); // Not in the current directory

        // Test getting a non-git directory
        let item = get_project_item(workspace, "org2", "not-a-repo").unwrap();
        assert!(item.is_none());

        // Test getting a non-existent project
        let result = get_project_item(workspace, "org1", "nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_project_item_current_directory() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();
        let project_path = temp_dir.path().join("org1").join("project1");

        // Change to the project directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_path).unwrap();

        let item = get_project_item(workspace, "org1", "project1")
            .unwrap()
            .unwrap();
        assert!(item.is_current);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_list_project_items() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        let items = list_project_items(workspace, vec![]).unwrap();

        // Should only include git repositories (not the "not-a-repo" directory)
        assert_eq!(items.len(), 3);

        // Check that all items have the expected structure
        for item in &items {
            assert!(!item.id.is_empty());
            assert!(!item.name.is_empty());
            assert!(!item.organization.is_empty());
            assert!(item.remote.is_some());
        }

        // Check specific items exist
        let project1 = items.iter().find(|i| i.name == "project1").unwrap();
        assert_eq!(project1.organization, "org1");
        assert_eq!(
            project1.remote,
            Some("https://github.com/org1/project1.git".to_string())
        );
    }
}
