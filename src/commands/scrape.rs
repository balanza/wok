use serde::Serialize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use wok::lib::git::{discover_repositories, DiscoveredRepo};
use wok::lib::spinner;

#[derive(Serialize)]
struct ScrapeProject {
    name: String,
    organization: String,
    remote: Option<String>,
}

#[derive(Serialize)]
struct WorkspaceScrape {
    timestamp: String,
    items: Vec<ScrapeProject>,
}

pub fn handle(
    workspace: &str,
    path: &str,
    org_filter: Option<Vec<String>>,
    export_mode: bool,
    import_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Discover repositories
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path).into());
    }

    let spinner = spinner::Spinner::new(format!("Scanning {} for repositories...", path));
    let repos = discover_repositories(path_obj)?;
    spinner.success(format!("Found {} repositories", repos.len()));

    // Filter by organization if specified
    let filtered_repos: Vec<DiscoveredRepo> = if let Some(ref orgs) = org_filter {
        repos
            .into_iter()
            .filter(|repo| orgs.contains(&repo.org))
            .collect()
    } else {
        repos
    };

    if filtered_repos.is_empty() {
        println!("No repositories found matching the criteria.");
        return Ok(());
    }

    // Convert to scrape format
    let scrape_projects: Vec<ScrapeProject> = filtered_repos
        .iter()
        .map(|r| ScrapeProject {
            name: r.name.clone(),
            organization: r.org.clone(),
            remote: Some(r.remote.clone()),
        })
        .collect();

    // Get current timestamp
    let timestamp = get_timestamp()?;

    let scrape_data = WorkspaceScrape {
        timestamp,
        items: scrape_projects,
    };

    // Handle different modes
    if export_mode {
        // Export mode: write to stdout
        export_to_stdout(&scrape_data)?;
    } else if import_mode {
        // Import mode: import all repositories
        import_repositories(workspace, &filtered_repos)?;
    } else {
        // Interactive mode: show prompt
        show_interactive_prompt(workspace, &scrape_data, &filtered_repos)?;
    }

    Ok(())
}

fn get_timestamp() -> Result<String, Box<dyn std::error::Error>> {
    let timestamp_output = Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output()?;

    Ok(String::from_utf8(timestamp_output.stdout)?
        .trim()
        .to_string())
}

fn export_to_stdout(
    scrape_data: &WorkspaceScrape,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&scrape_data)?;
    println!("{}", json);
    Ok(())
}

fn export_to_file(
    scrape_data: &WorkspaceScrape,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&scrape_data)?;
    fs::write(file_path, json)?;
    Ok(())
}

fn import_repositories(
    workspace: &str,
    repos: &[DiscoveredRepo],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Importing {} repositories...", repos.len());

    let mut cloned = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for repo in repos {
        let destination = format!("{}/{}/{}", workspace, repo.org, repo.name);

        // Check if project already exists
        if Path::new(&destination).exists() {
            println!("Skipping {} (already exists)", destination);
            skipped += 1;
            continue;
        }

        // Create organization directory if it doesn't exist
        let org_dir = format!("{}/{}", workspace, repo.org);
        fs::create_dir_all(&org_dir)?;

        // Clone the repository
        let spinner = spinner::Spinner::new(format!("Cloning {}/{}", repo.org, repo.name));

        match Command::new("git")
            .arg("clone")
            .arg(&repo.remote)
            .arg(&destination)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {
                spinner.success(format!("✓ {}/{}", repo.org, repo.name));
                cloned += 1;
            }
            Ok(_) => {
                spinner.error(format!("✗ {}/{}", repo.org, repo.name));
                failed += 1;
            }
            Err(e) => {
                spinner.error(format!("✗ {}/{} ({})", repo.org, repo.name, e));
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

fn show_interactive_prompt(
    workspace: &str,
    scrape_data: &WorkspaceScrape,
    repos: &[DiscoveredRepo],
) -> Result<(), Box<dyn std::error::Error>> {
    // Display found repositories
    println!("\nFound {} repositories:", repos.len());
    for repo in repos {
        println!("  {}/{}", repo.org, repo.name);
    }

    // Show options
    println!("\nOptions:");
    println!("  1. Import repositories into current workspace");
    println!("  2. Save to JSON file (can be imported later)");
    println!("  3. Abort");

    // Read user input
    print!("\nEnter your choice [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    match choice {
        "1" => {
            // Import repositories
            import_repositories(workspace, repos)?;
        }
        "2" => {
            // Save to JSON file with a random name
            let file_path = generate_export_filename()?;
            export_to_file(scrape_data, &file_path)?;
            println!("Exported {} repositories to {}", repos.len(), file_path);
        }
        "3" => {
            println!("Aborted.");
        }
        _ => {
            println!("Invalid choice. Aborted.");
        }
    }

    Ok(())
}

fn generate_export_filename() -> Result<String, Box<dyn std::error::Error>> {
    // Get current timestamp in a compact format
    let timestamp_output = Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()?;

    let timestamp = String::from_utf8(timestamp_output.stdout)?
        .trim()
        .to_string();

    // Generate a random 4-character hex string
    let random_output = Command::new("sh")
        .arg("-c")
        .arg("head -c 2 /dev/urandom | xxd -p")
        .output()?;

    let random_hex = String::from_utf8(random_output.stdout)?
        .trim()
        .to_string();

    Ok(format!("wok-scrape-{}-{}.json", timestamp, random_hex))
}
