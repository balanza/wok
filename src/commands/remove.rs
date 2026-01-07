use std::env;
use std::fs;
use wok::lib::spinner::Spinner;

pub fn handle(workspace: &str, project_spec: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the project specification (must be in "org/project" format)
    let parts: Vec<&str> = project_spec.split('/').collect();
    if parts.len() != 2 {
        return Err(Box::from(
            "Project must be specified in the format: org/project"
        ));
    }

    let organization = parts[0];
    let project_name = parts[1];

    if organization.is_empty() || project_name.is_empty() {
        return Err(Box::from(
            "Project must be specified in the format: org/project"
        ));
    }

    let project_path = format!("{}/{}/{}", workspace, organization, project_name);

    // Safety checks
    validate_project_path(workspace, &project_path)?;

    // Delete the project directory
    let spinner = Spinner::new(format!("Removing {}/{}", organization, project_name));
    match fs::remove_dir_all(&project_path) {
        Ok(_) => {
            spinner.success(format!("Removed {}/{}", organization, project_name));
        }
        Err(e) => {
            spinner.error("Failed to remove project".to_string());
            return Err(Box::from(format!("Deletion failed: {}", e)));
        }
    }

    // Optional: Clean up empty organization directory
    let org_path = format!("{}/{}", workspace, organization);
    if let Ok(entries) = fs::read_dir(&org_path) {
        if entries.count() == 0 {
            let _ = fs::remove_dir(&org_path);
        }
    }

    Ok(())
}

fn validate_project_path(workspace: &str, project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project_path_obj = std::path::Path::new(project_path);
    let workspace_path = std::path::Path::new(workspace);

    // Check if path is within workspace
    if !project_path_obj.starts_with(workspace_path) {
        return Err(Box::from("Security error: Path is outside workspace"));
    }

    // Check if directory exists
    if !project_path_obj.exists() {
        return Err(Box::from(
            "Project not found. Project must be specified in the format: org/project"
        ));
    }

    // Check if it's actually a directory
    if !project_path_obj.is_dir() {
        return Err(Box::from(format!("Path is not a directory: {}", project_path)));
    }

    // Check if it's a git repository
    let git_dir = project_path_obj.join(".git");
    if !git_dir.exists() {
        return Err(Box::from("Not a valid project (missing .git directory)"));
    }

    // Check if it's the current working directory
    if let Ok(current_dir) = env::current_dir() {
        if current_dir.starts_with(project_path_obj) {
            return Err(Box::from(
                "Cannot remove current directory. Please navigate elsewhere first."
            ));
        }
    }

    Ok(())
}
