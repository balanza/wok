use serde::Serialize;
use std::process::Command;
use wok::lib::projects::list_project_items;

#[derive(Serialize)]
struct ExportProject {
    name: String,
    organization: String,
    remote: Option<String>,
}

#[derive(Serialize)]
struct WorkspaceExport {
    timestamp: String,
    items: Vec<ExportProject>,
}

pub fn handle(workspace: &str) -> Result<(), Box<dyn std::error::Error>> {
    let projects = list_project_items(workspace, vec![])?;

    // Convert to export format (without runtime state like is_current)
    let export_projects: Vec<ExportProject> = projects
        .into_iter()
        .map(|p| ExportProject {
            name: p.name,
            organization: p.organization,
            remote: p.remote,
        })
        .collect();

    // Get current timestamp in ISO 8601 format using date command
    let timestamp_output = Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output()?;

    let timestamp = String::from_utf8(timestamp_output.stdout)?
        .trim()
        .to_string();

    let export = WorkspaceExport {
        timestamp,
        items: export_projects,
    };

    let json = serde_json::to_string_pretty(&export)?;
    println!("{}", json);
    Ok(())
}
