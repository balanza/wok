use std::os::unix::io::AsRawFd;
use wok::lib::constants::{GOTO_MARKER, MULTIPLE_MATCHES_MARKER};
use wok::lib::fd::write_to_fd;
use wok::lib::fuzzy::FuzzySearcher;
use wok::lib::git::{create_worktree, list_worktrees, WorktreeItem};
use wok::lib::projects::{list_project_items, ProjectItem};

pub fn handle(
    workspace: &str,
    project: Option<&str>,
    search: Option<&str>,
    extra_args: &[String],
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, project)?;
    let canonical_project = project_path.canonicalize().unwrap_or(project_path.clone());
    let worktrees: Vec<WorktreeItem> = list_worktrees(&project_path)?
        .into_iter()
        .filter(|wt| {
            wt.path.canonicalize().unwrap_or(wt.path.clone()) != canonical_project
        })
        .collect();

    match search {
        None => handle_list(&project_path, &worktrees),
        Some("__root") => {
            let dest = project_path.to_string_lossy().to_string();
            let goto_dest = format!("{}{}", GOTO_MARKER, &dest);
            write_to_fd(3, &goto_dest)?;
            if is_tty() {
                println!("{}", &dest);
            }
            Ok(())
        }
        Some(term) => handle_search(&project_path, &worktrees, term, extra_args, yes),
    }
}

fn resolve_project_path(
    workspace: &str,
    project: Option<&str>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    match project {
        Some(search_term) => {
            let list: Vec<ProjectItem> = list_project_items(workspace, vec![])?;
            let filtered = fuzzy_search_projects(&list, search_term);

            if filtered.is_empty() {
                return Err(Box::from("No projects found matching the search criteria."));
            }
            if filtered.len() > 1 {
                let names: Vec<String> = filtered
                    .iter()
                    .map(|p| format!("{}/{}", p.organization, p.name))
                    .collect();
                return Err(Box::from(format!(
                    "Ambiguous project name, multiple matches found: {}",
                    names.join(", ")
                )));
            }

            Ok(std::path::PathBuf::from(format!(
                "{}/{}/{}",
                workspace, filtered[0].organization, filtered[0].name
            )))
        }
        None => {
            let current_dir = std::env::current_dir()?;
            let workspace_path = std::path::Path::new(workspace).canonicalize()?;
            let canonical_current = current_dir.canonicalize()?;

            if !canonical_current.starts_with(&workspace_path) {
                return Err(Box::from(
                    "Could not determine current project. Please specify a project name.",
                ));
            }

            // The project path is workspace/org/name
            // or we might be inside a worktree: workspace/org/.worktrees/name/worktree
            let relative = canonical_current.strip_prefix(&workspace_path)?;
            let components: Vec<_> = relative.components().collect();
            if components.len() < 2 {
                return Err(Box::from(
                    "Could not determine current project. Please specify a project name.",
                ));
            }

            let org = components[0].as_os_str().to_string_lossy();
            let second = components[1].as_os_str().to_string_lossy();

            if second == ".worktrees" {
                // Inside a worktree: workspace/org/.worktrees/project/worktree
                if components.len() < 3 {
                    return Err(Box::from(
                        "Could not determine current project. Please specify a project name.",
                    ));
                }
                let name = components[2].as_os_str().to_string_lossy();
                Ok(workspace_path.join(org.as_ref()).join(name.as_ref()))
            } else {
                Ok(workspace_path.join(org.as_ref()).join(second.as_ref()))
            }
        }
    }
}

fn handle_list(
    project_path: &std::path::Path,
    worktrees: &[WorktreeItem],
) -> Result<(), Box<dyn std::error::Error>> {
    if worktrees.is_empty() {
        if is_tty() {
            println!("No worktrees found.");
        }
        return Ok(());
    }

    // Always write to FD3 for the shell wrapper menu (no-op if FD3 is not available).
    // Format: "label::/path" — the shell wrapper displays the label and cd's to the path.
    let mut entries: Vec<String> = Vec::with_capacity(worktrees.len() + 1);
    entries.push(format!("__root::{}", project_path.to_string_lossy()));
    for wt in worktrees {
        entries.push(format!("{}::{}", wt.name, wt.path.to_string_lossy()));
    }
    let data = format!("{}\n{}", MULTIPLE_MATCHES_MARKER, entries.join("\n"));
    write_to_fd(3, &data)?;

    // Non-TTY context without FD3 (e.g. shell completion): also print names to stdout
    if !is_tty() {
        println!("__root");
        for wt in worktrees {
            println!("{}", wt.name);
        }
    }

    Ok(())
}

fn handle_search(
    project_path: &std::path::Path,
    worktrees: &[WorktreeItem],
    term: &str,
    extra_args: &[String],
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Exact match only — no fuzzy search
    if let Some(wt) = worktrees.iter().find(|wt| wt.name == term) {
        let dest = wt.path.to_string_lossy().to_string();
        let goto_dest = format!("{}{}", GOTO_MARKER, &dest);
        write_to_fd(3, &goto_dest)?;
        if is_tty() {
            println!("{}", &dest);
        }
        return Ok(());
    }

    // Unknown worktree — prompt the user unless --yes is set
    if !yes {
        eprint!("Worktree '{}' does not exist. Create it? [y/N] ", term);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            return Ok(());
        }
    }

    let new_path = create_worktree(project_path, term, extra_args)?;
    let goto_dest = format!("{}{}", GOTO_MARKER, new_path.to_string_lossy());
    write_to_fd(3, &goto_dest)?;
    if is_tty() {
        println!("{}", new_path.to_string_lossy());
    }
    Ok(())
}

fn is_tty() -> bool {
    unsafe { libc::isatty(std::io::stdout().as_raw_fd()) != 0 }
}

fn fuzzy_search_projects(list: &[ProjectItem], search: &str) -> Vec<ProjectItem> {
    let flat_list = list
        .iter()
        .map(|item| format!("{}/{}", item.organization, item.name))
        .collect::<Vec<_>>();
    let searcher = FuzzySearcher::new(0.6);
    searcher
        .search(search, &flat_list)
        .into_iter()
        .map(|m| m.text)
        .filter_map(|text| {
            list.iter()
                .find(|item| format!("{}/{}", item.organization, item.name) == text)
                .cloned()
        })
        .collect()
}

