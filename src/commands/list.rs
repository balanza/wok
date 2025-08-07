use std::io::{self, Write};
use std::thread;

#[derive(Debug, Clone)]
struct ProjectItem {
    id: String,
    name: String,
    organization: String,
    remote: Option<String>,
    is_current: bool,
}

pub fn handle(
    workspace: &str,
    orgs: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let filter_orgs = orgs.clone().unwrap_or_else(Vec::new);
    let list: Vec<ProjectItem> = list_project_items(workspace, filter_orgs)?;
    let mut writer = io::stdout();
    write_project_tree_compact(list, &mut writer)?;
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

fn write_project_tree_compact<W: Write>(list: Vec<ProjectItem>, writer: &mut W) -> io::Result<()> {
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

