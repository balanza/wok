use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use wok::lib::shell::{detect_shell, prepare_shell, ShellSetupItemStatus, SupportedShells};

pub fn handle(shell: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let shell_type = match shell {
        Some(s) => validate_shell(s)?,
        None => detect_shell()?,
    };

    let binary_path = env::current_exe()?.to_string_lossy().to_string();
    let home_dir = dirs::home_dir()
        .ok_or("Unable to find home directory")?
        .to_string_lossy()
        .to_string();

    println!("Cleaning up wok setup for {}...", shell_type.to_string());

    // Get the setup items to understand what needs to be cleaned
    let items = match prepare_shell(&shell_type, binary_path, &home_dir) {
        Ok(items) => items,
        Err(errors) => {
            return Err(Box::from(format!(
                "Failed to prepare shell for {}: {}",
                shell_type.to_string(),
                errors.len()
            )));
        }
    };

    // Iterate through items and undo them
    for item in items {
        // Skip items that were never created (status is Todo)
        if item.status == ShellSetupItemStatus::Todo {
            println!("Skipping {} (was never installed)", item.name);
            continue;
        }

        if item.entire_file {
            // Remove the entire file
            remove_file(&item.target, &item.name)?;
        } else {
            // Remove the content line from the file
            remove_line_from_file(&item.target, &item.content, &item.name)?;
        }
    }

    println!("\nWok setup has been cleaned up successfully!");
    println!("Please restart your shell to apply changes.");

    Ok(())
}

fn validate_shell(shell: &str) -> Result<SupportedShells, Box<dyn std::error::Error>> {
    match shell.to_lowercase().as_str() {
        "bash" => Ok(SupportedShells::Bash),
        "zsh" => Ok(SupportedShells::Zsh),
        _ => Err(Box::from(format!("Unsupported shell type: {}", shell))),
    }
}

fn remove_file(
    file_path: &str,
    item_name: &impl std::fmt::Display,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::path::Path::new(file_path).exists() {
        fs::remove_file(file_path)?;
        println!("Removed {} from {}", item_name, file_path);
    } else {
        println!("File {} does not exist, skipping {}", file_path, item_name);
    }
    Ok(())
}

fn remove_line_from_file(
    file_path: &str,
    content: &str,
    item_name: &impl std::fmt::Display,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(file_path).exists() {
        println!(
            "File {} does not exist, skipping {} removal",
            file_path, item_name
        );
        return Ok(());
    }

    // Read the file
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut removed = false;

    for line in reader.lines() {
        let line = line?;
        // Skip the line if it matches the content
        if line.trim() == content.trim() {
            removed = true;
            continue;
        }
        lines.push(line);
    }

    if removed {
        // Write back the modified content
        let mut file = fs::File::create(file_path)?;
        for line in lines {
            writeln!(file, "{}", line)?;
        }
        println!("Removed {} from {}", item_name, file_path);
    } else {
        println!(
            "Content for {} not found in {}, skipping",
            item_name, file_path
        );
    }

    Ok(())
}
