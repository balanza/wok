use clap::{Parser, Subcommand};

/// `wok` - Workspace repo management tool
#[derive(Parser, Debug)]
#[command(
    name = "wok",
    version = env!("CARGO_PKG_VERSION"),
    author = "Emanuele De Cupis <emanuele@decup.is>",
    about = "Manage workspace repositories"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands supported by `wok`
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Clone the repo into $WOK_SPACE/acme/foo
    Add {
        /// Git repository URL to clone (e.g. git@github.com:acme/foo.git)
        #[arg()]
        repo_url: String,
    },

    /// List all repositories in the workspace, grouped by organization
    #[command(visible_alias = "ls")]
    List {
        /// Filter by comma-separated list of organizations (e.g. --org acme,ymca)
        #[arg(long, value_delimiter = ',')]
        org: Option<Vec<String>>,
    },

    /// Fast-forward all main branches for all repositories
    Ff {
        /// Filter by comma-separated list of organizations (e.g. --org acme,ymca)
        #[arg(long, value_delimiter = ',')]
        org: Option<Vec<String>>,
    },

    /// Move to project directory. Use fuzzy search to find the project.
    Go {
        #[arg()]
        search: String,
    },

    /// Export the workspace to a json content
    Export {},

    /// Import a workspace from a json content
    Import {
        /// Input content
        #[arg()]
        input: String,
    },
}
