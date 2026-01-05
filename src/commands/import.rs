use serde::Deserialize;
use std::fs;
use std::io::{self, Read};
use std::process::{Command, Stdio};
use wok::lib::spinner;

#[derive(Deserialize)]
struct ImportProject {
    name: String,
    organization: String,
    remote: Option<String>,
}

#[derive(Deserialize)]
struct WorkspaceImport {
    #[allow(dead_code)]
    timestamp: String,
    items: Vec<ImportProject>,
}

pub fn handle(workspace: &str, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Read input from file or stdin
    let json_content = if input == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        fs::read_to_string(input)?
    };

    // Parse JSON
    let import_data: WorkspaceImport = serde_json::from_str(&json_content)?;

    println!("Importing {} projects...", import_data.items.len());

    let mut cloned = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for project in import_data.items {
        let destination = format!("{}/{}/{}", workspace, project.organization, project.name);

        // Check if project already exists
        if std::path::Path::new(&destination).exists() {
            println!("Skipping {} (already exists)", destination);
            skipped += 1;
            continue;
        }

        // Skip projects without a remote URL
        let Some(remote_url) = project.remote else {
            println!(
                "Skipping {}/{} (no remote URL)",
                project.organization, project.name
            );
            skipped += 1;
            continue;
        };

        // Skip projects with empty remote URL
        if remote_url.is_empty() {
            println!(
                "Skipping {}/{} (empty remote URL)",
                project.organization, project.name
            );
            skipped += 1;
            continue;
        }

        // Create organization directory if it doesn't exist
        let org_dir = format!("{}/{}", workspace, project.organization);
        fs::create_dir_all(&org_dir)?;

        // Clone the repository
        let spinner = spinner::Spinner::new(format!("Cloning {}/{}", project.organization, project.name));

        match Command::new("git")
            .arg("clone")
            .arg(&remote_url)
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                spinner.success(format!("✓ {}/{}", project.organization, project.name));
                cloned += 1;
            }
            Ok(_) => {
                spinner.error(format!("✗ {}/{}", project.organization, project.name));
                failed += 1;
            }
            Err(e) => {
                spinner.error(format!("✗ {}/{} ({})", project.organization, project.name, e));
                failed += 1;
            }
        }
    }

    println!("\nImport complete:");
    println!("  Cloned:  {}", cloned);
    println!("  Skipped: {}", skipped);
    println!("  Failed:  {}", failed);

    Ok(())
}
