mod cli;
mod commands;

use dirs;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = cli::WokCli::parse().unwrap();

    let (workspace, source) = determine_workspace();

    return match command {
        cli::InferredCommand::Dashboard => commands::dashboard::handle(&workspace, &source),
        cli::InferredCommand::Add { repo_url } => commands::add::handle(&workspace, &repo_url),
        cli::InferredCommand::List { org, format } => {
            commands::list::handle(&workspace, org, &format)
        }
        cli::InferredCommand::FastForward { org } => commands::ff::handle(&workspace, org),
        cli::InferredCommand::Go { search } => commands::go::handle(&workspace, &search),
        cli::InferredCommand::Export {} => commands::export::handle(&workspace),
        cli::InferredCommand::Import { input } => commands::import::handle(&workspace, &input),
        cli::InferredCommand::Setup { shell, manual } => commands::setup::handle(&shell, manual),
    };
}

fn determine_workspace() -> (String, String) {
    match env::var("WOK_SPACE") {
        Ok(val) => (val, "env".to_string()),
        Err(_) => (default_workspace(), "default".to_string()),
    }
}

fn default_workspace() -> String {
    let home = dirs::home_dir().expect("Failed to get home directory");
    format!("{}/Workspace", home.to_string_lossy())
}
