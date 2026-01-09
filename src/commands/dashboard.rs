use std::collections::HashSet;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use wok::lib::projects::list_project_items;
use wok::lib::shell::{detect_shell, prepare_shell, ShellSetupItemStatus};

pub fn handle(workspace: &str, source: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Display dashboard header
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("WOK v{}", version);
    let box_width = 37; // Inner width of the box
    let padding = (box_width - title.len()) / 2;
    let padded_title = format!("{:^width$}", title, width = box_width);

    println!("╭─────────────────────────────────────╮");
    println!("│{}│", padded_title);
    println!("╰─────────────────────────────────────╯");
    println!();
    println!("  {}", render_workspace(workspace, source));
    println!();
    println!("  {}", render_projects_count(workspace));
    println!();

    // Check setup status
    check_setup_status()?;

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

fn check_setup_status() -> Result<(), Box<dyn std::error::Error>> {
    // Try to detect shell
    let shell_type = match detect_shell() {
        Ok(shell) => shell,
        Err(_) => {
            // If shell detection fails, skip setup check
            return Ok(());
        }
    };

    let binary_path = match env::current_exe() {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => return Ok(()), // Skip if can't determine binary path
    };

    let home_dir = match dirs::home_dir() {
        Some(dir) => dir.to_string_lossy().to_string(),
        None => return Ok(()), // Skip if can't determine home directory
    };

    let items = match prepare_shell(&shell_type, binary_path, &home_dir) {
        Ok(items) => items,
        Err(_) => return Ok(()), // Skip if shell preparation fails
    };

    // Check if all items are Done
    let all_done = items
        .iter()
        .all(|item| item.status == ShellSetupItemStatus::Done);

    // Display setup checklist
    println!("Setup Status:");
    for item in &items {
        let status_icon = match item.status {
            ShellSetupItemStatus::Done => "✓",
            ShellSetupItemStatus::Todo => "✗",
            ShellSetupItemStatus::OutOfDate => "⚠",
        };
        let status_text = match item.status {
            ShellSetupItemStatus::Done => "Done",
            ShellSetupItemStatus::Todo => "Todo",
            ShellSetupItemStatus::OutOfDate => "Out of Date",
        };
        println!("  {} {} - {}", status_icon, item.name, status_text);
    }
    println!();

    if all_done {
        println!("Wok setup correctly");
        println!();
    } else {
        // Prompt user to update setup
        println!("⚠ Wok needs to be setup to work correctly.");
        print!("Do you want to update the setup now? (y/n): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "y" || input == "yes" {
            // Run setup
            println!();
            crate::commands::setup::handle(&None, false)?;
        } else {
            println!();
            println!("You can always redo the setup by running: wok --setup");
            println!();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_workspace_env() {
        let result = render_workspace("/test/workspace", "env");
        assert_eq!(result, "Using workspace '/test/workspace' from environment");
    }

    #[test]
    fn test_render_workspace_default() {
        let result = render_workspace("/test/workspace", "default");
        assert_eq!(result, "Using default workspace '/test/workspace'");
    }

    #[test]
    fn test_render_workspace_other() {
        let result = render_workspace("/test/workspace", "other");
        assert_eq!(result, "/test/workspace");
    }

    #[test]
    fn test_render_projects_count_no_dir() {
        let result = render_projects_count("/nonexistent/path");
        assert_eq!(result, "Workspace directory does not exist");
    }
}
