mod cli;
mod commands;

use clap::Parser;
use dirs;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    let workspace = env::var("WOK_SPACE").unwrap_or_else(|_| default_workspace());

    return match cli.command {
        cli::Commands::Add { repo_url } => commands::add::handle(&workspace, &repo_url),
        cli::Commands::List { org, format } => commands::list::handle(&workspace, org, &format),
        cli::Commands::Ff { org } => commands::ff::handle(&workspace, org),
        cli::Commands::Go { search } => commands::go::handle(&workspace, &search),
        cli::Commands::Export {} => commands::export::handle(&workspace),
        cli::Commands::Import { input } => commands::import::handle(&workspace, &input),
        cli::Commands::Setup { shell, manual } => commands::setup::handle(&shell, manual),
    };
}

fn default_workspace() -> String {
    let home = dirs::home_dir().expect("Failed to get home directory");
    format!("{}/Workspace", home.to_string_lossy())
}
