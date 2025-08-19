use std::io::{self};
use wok::lib::list_writer::get_writer_fn;
use wok::lib::projects::{list_project_items, ProjectItem};

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
    fn test_handle_integration() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path().to_str().unwrap();

        // Capture stdout
        let mut output = Vec::new();
        {
            // We can't easily test handle() directly because it writes to stdout,
            // but we can test the underlying functions it uses
            let list = list_project_items(workspace, vec![]).unwrap();
            let writer = get_writer_fn("tree").unwrap();
            writer(list, &mut output).unwrap();
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
        let writer = get_writer_fn("tree").unwrap();
        writer(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        // Should be alphabetically sorted by organization
        assert_eq!(lines[0], "aardvark");
        assert!(lines[2].contains("middle"));
        assert!(lines[4].contains("zoo"));
    }
}
