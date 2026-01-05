use std::collections::HashSet;
use std::path::Path;
use wok::lib::projects::list_project_items;

pub fn handle(workspace: &str, source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Display dashboard header
    println!("╭─────────────────────────────────────╮");
    println!("│       Wok Workspace Dashboard       │");
    println!("╰─────────────────────────────────────╯");
    println!();
    println!("  {}", render_workspace(workspace, source));
    println!();
    println!("  {}", render_projects_count(workspace));
    println!();

    Ok(())
}

fn render_workspace(workspace: &str, source: &str) -> String {
    match source {
        "env" => format!("Using workspace '{}' from environment", workspace),
        "default" => format!("Using default workspace '{}'", workspace),
        _ => workspace.to_string(),
    }
}

fn render_projects_count(workspace: &str) -> String {
    match list_project_items(workspace, vec![]) {
        Ok(projects) => {
            // Count organizations
            let orgs: HashSet<String> = projects.iter().map(|p| p.organization.clone()).collect();

            format!(
                "{} projects in {} organizations",
                projects.len(),
                orgs.len()
            )
        }
        Err(_) => {
            // Check if workspace directory exists
            if !Path::new(workspace).exists() {
                "Workspace directory does not exist".to_string()
            } else {
                "No projects found".to_string()
            }
        }
    }
}
