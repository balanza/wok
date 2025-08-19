use serde::Serialize;
use std::io::{self, Write};
use std::thread;

#[derive(Debug, Clone, Serialize)]
struct ProjectItem {
    id: String,
    name: String,
    organization: String,
    remote: Option<String>,
    is_current: bool,
}

type ListWriter = fn(Vec<ProjectItem>, &mut dyn Write) -> io::Result<()>;

pub fn handle(
    workspace: &str,
    orgs: Option<Vec<String>>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let filter_orgs = orgs.clone().unwrap_or_else(Vec::new);
    let list: Vec<ProjectItem> = list_project_items(workspace, filter_orgs)?;

    let writer = get_writer_fn(format)?;
    writer(list, &mut io::stdout())?;

    Ok(())
}

// Generates a unique project ID in the form "organization::project" to preserve alphabetical order.
fn project_id(org: &str, project: &str) -> String {
    format!("{}::{}", org, project)
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
        .map(|current| current.starts_with(&project_path))
        .unwrap_or(false);

    Ok(Some(ProjectItem {
        id: project_id(org, project),
        organization: org.to_string(),
        name: project.to_string(),
        remote,
        is_current,
    }))
}

// Lists all projects in the workspace, fetching their details concurrently.
// Returns a vector of ProjectItem, which includes the project name, organization, remote URL,
// and whether it is the current project.
// Uses threads to fetch project details concurrently for better performance.
fn list_project_items(
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

fn get_writer_fn(format: &str) -> io::Result<ListWriter> {
    match format {
        "json" => Ok(write_project_tree_json),
        "flat" => Ok(write_project_tree_flat),
        "tree" => Ok(write_project_tree_compact),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported format: {}", format),
        )),
    }
}

fn write_project_tree_compact(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    if list.is_empty() {
        return Ok(());
    }

    // sorting by id ensures that we maintain the order of projects within each organization
    // This is important for the tree structure to be consistent.
    let mut sorted_list = list.clone();
    sorted_list.sort_by(|a, b| a.id.cmp(&b.id));

    let mut current_org: Option<&str> = None;

    for (i, item) in sorted_list.iter().enumerate() {
        // Check if we're starting a new organization
        if current_org != Some(&item.organization) {
            // If we had a previous org, fix up the last project's prefix
            if i > 0 {
                // The last project of previous org needs └── instead of ├──
                // We'll handle this by looking ahead
            }

            // Write the organization name
            writeln!(writer, "{}", item.organization)?;
            current_org = Some(&item.organization);
        }

        // Check if this is the last project in the current org
        let is_last_in_org =
            i == sorted_list.len() - 1 || sorted_list[i + 1].organization != item.organization;

        let prefix = if is_last_in_org {
            "└──"
        } else {
            "├──"
        };

        let marker = if item.is_current { "* " } else { "" };

        writeln!(
            writer,
            "{} {}{} ({})",
            prefix,
            marker,
            item.name,
            item.remote.as_deref().unwrap_or("no remote")
        )?;
    }

    Ok(())
}

fn write_project_tree_json(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    let result = serde_json::to_writer(writer, &list.clone());

    if let Err(e) = result {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to write JSON: {}", e),
        ));
    }
    Ok(())
}

fn write_project_tree_flat(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    for item in list {
        let marker = if item.is_current { "* " } else { "" };
        writeln!(
            writer,
            "{}{} ({})",
            marker,
            item.name,
            item.remote.as_deref().unwrap_or("no remote")
        )?;
    }
    Ok(())
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

    /// Tests for the different output formats

    #[test]
    fn test_get_writer_function() {
        assert!(get_writer_fn("json").is_ok());
        assert!(get_writer_fn("flat").is_ok());
        assert!(get_writer_fn("tree").is_ok());
        assert!(get_writer_fn("invalid").is_err());
    }

    // tree

    #[test]
    fn test_write_empty_list_tree() {
        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        let result = writer_fn(vec![], &mut output);

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[test]
    fn test_write_single_org_tree() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project2".to_string(),
                name: "project2".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project2.git".to_string()),
                is_current: true,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines[0], "org1");
        assert_eq!(
            lines[1],
            "├── project1 (https://github.com/org1/project1.git)"
        );
        assert_eq!(
            lines[2],
            "└── * project2 (https://github.com/org1/project2.git)"
        );
    }

    #[test]
    fn test_write_multiple_orgs_tree() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org2::project2".to_string(),
                name: "project2".to_string(),
                organization: "org2".to_string(),
                remote: Some("https://github.com/org2/project2.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project3".to_string(),
                name: "project3".to_string(),
                organization: "org1".to_string(),
                remote: None,
                is_current: false,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        // Should be sorted by ID, so org1 projects come first
        assert_eq!(lines[0], "org1");
        assert_eq!(
            lines[1],
            "├── project1 (https://github.com/org1/project1.git)"
        );
        assert_eq!(lines[2], "└── project3 (no remote)");
        assert_eq!(lines[3], "org2");
        assert_eq!(
            lines[4],
            "└── project2 (https://github.com/org2/project2.git)"
        );
    }

    #[test]
    fn test_write_no_remote_tree() {
        let items = vec![ProjectItem {
            id: "org1::project1".to_string(),
            name: "project1".to_string(),
            organization: "org1".to_string(),
            remote: None,
            is_current: false,
        }];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("(no remote)"));
    }

    // json
    #[test]
    fn test_write_empty_list_json() {
        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        let result = writer_fn(vec![], &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.eq("[]"));
    }

    #[test]
    fn test_write_single_org_json() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project2".to_string(),
                name: "project2".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project2.git".to_string()),
                is_current: true,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":"https://github.com/org1/project1.git","is_current":false},{"id":"org1::project2","name":"project2","organization":"org1","remote":"https://github.com/org1/project2.git","is_current":true}]"#;
        assert_eq!(output_str, expected);
    }

    #[test]
    fn test_write_multiple_orgs_json() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org2::project2".to_string(),
                name: "project2".to_string(),
                organization: "org2".to_string(),
                remote: Some("https://github.com/org2/project2.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project3".to_string(),
                name: "project3".to_string(),
                organization: "org1".to_string(),
                remote: None,
                is_current: false,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":"https://github.com/org1/project1.git","is_current":false},{"id":"org2::project2","name":"project2","organization":"org2","remote":"https://github.com/org2/project2.git","is_current":false},{"id":"org1::project3","name":"project3","organization":"org1","remote":null,"is_current":false}]"#;
        assert_eq!(output_str, expected);
    }

    #[test]
    fn test_write_no_remote_json() {
        let items = vec![ProjectItem {
            id: "org1::project1".to_string(),
            name: "project1".to_string(),
            organization: "org1".to_string(),
            remote: None,
            is_current: false,
        }];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":null,"is_current":false}]"#;
        assert_eq!(output_str, expected);
    }

    // flat
    // TBD

    #[test]
    fn test_handle_integration() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Capture stdout
        let mut output = Vec::new();
        {
            // We can't easily test handle() directly because it writes to stdout,
            // but we can test the underlying functions it uses
            let list = list_project_items(workspace, vec![]).unwrap();
            write_project_tree_compact(list, &mut output).unwrap();
        }

        let output_str = String::from_utf8(output).unwrap();

        // Should contain organization names
        assert!(output_str.contains("org1"));
        assert!(output_str.contains("org2"));

        // Should contain project names
        assert!(output_str.contains("project1"));
        assert!(output_str.contains("project2"));
        assert!(output_str.contains("project3"));

        // Should use tree characters
        assert!(output_str.contains("├──"));
        assert!(output_str.contains("└──"));
    }

    #[test]
    fn test_concurrent_list_project_items() {
        // Test that concurrent fetching works correctly
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Run multiple times to check for race conditions
        for _ in 0..5 {
            let items = list_project_items(workspace, vec![]).unwrap();
            assert_eq!(items.len(), 3);

            // Check that all items are unique
            let mut ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), 3);
        }
    }

    #[test]
    fn test_sorting_consistency() {
        // Test that items are consistently sorted
        let items = vec![
            ProjectItem {
                id: "zoo::project1".to_string(),
                name: "project1".to_string(),
                organization: "zoo".to_string(),
                remote: None,
                is_current: false,
            },
            ProjectItem {
                id: "aardvark::project2".to_string(),
                name: "project2".to_string(),
                organization: "aardvark".to_string(),
                remote: None,
                is_current: false,
            },
            ProjectItem {
                id: "middle::project3".to_string(),
                name: "project3".to_string(),
                organization: "middle".to_string(),
                remote: None,
                is_current: false,
            },
        ];

        let mut output = Vec::new();
        write_project_tree_compact(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        // Should be alphabetically sorted by organization
        assert_eq!(lines[0], "aardvark");
        assert!(lines[2].contains("middle"));
        assert!(lines[4].contains("zoo"));
    }
}
