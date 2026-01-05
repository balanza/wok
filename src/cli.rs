use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use std::env;

/// The inferred commands that `wok` can execute
#[derive(Debug, Clone, PartialEq)]
pub enum InferredCommand {
    /// Dashboard (no arguments)
    Dashboard,
    /// Setup command
    Setup { shell: Option<String>, manual: bool },
    /// Add repository (URL argument)
    Add { repo_url: String },
    /// Go to project (project path argument)
    Go { search: String },
    /// List repositories
    List {
        org: Option<Vec<String>>,
        format: String,
    },
    /// Fast-forward repositories
    FastForward { org: Option<Vec<String>> },
    /// Export workspace
    Export,
    /// Import workspace
    Import { input: String },
}

/// CLI argument parser with inference logic
pub struct WokCli;

impl WokCli {
    /// Build the clap Command structure
    pub fn build() -> Command {
        Command::new("wok")
            .version(env!("CARGO_PKG_VERSION"))
            .author("Emanuele De Cupis <emanuele@decup.is>")
            .about("Manage workspace repositories")
            .arg(
                Arg::new("list")
                    .short('l')
                    .long("list")
                    .help("List all repositories in the workspace")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("fast-forward")
                    .long("ff")
                    .help("Fast-forward all main branches for all repositories")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("export")
                    .long("export")
                    .help("Export the workspace")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("import")
                    .long("import")
                    .help("Import a workspace from a file or stdin")
                    .value_name("FILE")
                    .num_args(0..=1) // allow 0 or 1 argument
                    .default_missing_value("-"), // if no arg given, treat as "-"
            )
            .arg(
                Arg::new("org")
                    .long("org")
                    .help("Filter by comma-separated list of organizations")
                    .value_delimiter(',')
                    .action(ArgAction::Append),
            )
            .arg(
                Arg::new("format")
                    .short('F')
                    .long("format")
                    .help("Select the output format: json, flat, or tree")
                    .default_value("tree"),
            )
            .arg(
                Arg::new("setup")
                    .long("setup")
                    .help("Run the setup command")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("shell")
                    .short('s')
                    .long("shell")
                    .help("Shell type for setup (e.g. bash, zsh, fish)"),
            )
            .arg(
                Arg::new("manual")
                    .short('m')
                    .long("manual")
                    .help("Manual mode for setup (print setup script without running)")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("project")
                    .value_name("PROJECT")
                    .help("URL to add or project to navigate to")
                    .index(1)
                    .required(false)
                    .num_args(1..=1), // Allow 1 positional argument
            )
            .group(
                ArgGroup::new("subcommands")
                    .args(&["list", "fast-forward", "export", "import", "project"])
                    .required(false)
                    .multiple(false),
            )
            .group(
                ArgGroup::new("setup_options")
                    .args(&["setup", "shell", "manual"])
                    .required(false)
                    .multiple(true)
                    .conflicts_with_all(&["subcommands", "list_options", "fast_forward_options"]),
            )
            .group(
                ArgGroup::new("list_options")
                    .args(&["org", "format"])
                    .required(false)
                    .multiple(true)
                    .requires("list"),
            )
            .group(
                ArgGroup::new("fast_forward_options")
                    .args(&["org"])
                    .required(false)
                    .multiple(true)
                    .requires("fast-forward"),
            )
    }

    /// Parse arguments and infer the command
    pub fn parse() -> Result<InferredCommand, String> {
        let matches = Self::build().get_matches();
        Self::infer_command(&matches)
    }

    /// Infer the command from parsed arguments
    fn infer_command(matches: &ArgMatches) -> Result<InferredCommand, String> {
        // Handle flag-based commands first
        if matches.get_flag("list") {
            return Ok(InferredCommand::List {
                org: matches
                    .get_many::<String>("org")
                    .map(|v| v.cloned().collect()),
                format: matches
                    .get_one::<String>("format")
                    .unwrap_or(&"tree".to_string())
                    .clone(),
            });
        }

        if matches.get_flag("fast-forward") {
            return Ok(InferredCommand::FastForward {
                org: matches
                    .get_many::<String>("org")
                    .map(|v| v.cloned().collect()),
            });
        }

        if matches.get_flag("export") {
            return Ok(InferredCommand::Export);
        }

        if let Some(import_file) = matches.get_one::<String>("import") {
            return Ok(InferredCommand::Import {
                input: import_file.clone(),
            });
        }

        // Handle positional argument
        if let Some(positional) = matches.get_one::<String>("project") {
            if Self::is_url(positional) {
                return Ok(InferredCommand::Add {
                    repo_url: positional.clone(),
                });
            } else {
                return Ok(InferredCommand::Go {
                    search: positional.clone(),
                });
            }
        }

        // Check if setup options are provided
        if matches.get_flag("setup") || matches.contains_id("shell") || matches.get_flag("manual") {
            return Ok(InferredCommand::Setup {
                shell: matches.get_one::<String>("shell").cloned(),
                manual: matches.get_flag("manual"),
            });
        }

        // No arguments - default to dashboard
        Ok(InferredCommand::Dashboard)
    }

    /// Check if a string looks like a URL
    fn is_url(s: &str) -> bool {
        // Check for common Git URL patterns
        s.starts_with("git@")
            || s.starts_with("https://")
            || s.starts_with("http://")
            || s.starts_with("ssh://")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_detection() {
        assert!(WokCli::is_url("git@github.com:user/repo.git"));
        assert!(WokCli::is_url("https://github.com/user/repo"));
    }

    macro_rules! cli_test {
        ($($name:ident: $args:expr, $command:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let args = $args;
                    let matches = WokCli::build().get_matches_from(args);
                    let command = WokCli::infer_command(&matches).unwrap();
                    assert_eq!(
                        command,
                        $command
                    );
                }
            )*
        }
    }

    macro_rules! cli_test_error {
        ($($name:ident: $args:expr)*) => {
            $(
                #[test]
                fn $name() {
                    let args = $args;
                    let match_result = WokCli::build().try_get_matches_from(args);

                    if let Ok(matches) = &match_result {
                        let result = WokCli::infer_command(&matches);
                        assert!(result.is_err());
                    }

                    assert!(match_result.is_err());
                }
            )*
        }
    }

    cli_test! {
        test_add:
            vec!["wok", "git@github.com:user/repo.git"],
             InferredCommand::Add {
                repo_url: "git@github.com:user/repo.git".into()
            },

        test_go:
            vec!["wok", "my-project"],
            InferredCommand::Go {
                search: "my-project".into()
            },

        test_list:
            vec!["wok", "--list"],
            InferredCommand::List {
                org: None,
                format: "tree".into(),
            },

        test_list_with_org:
            vec!["wok", "--list", "--org", "org1,org2"],
            InferredCommand::List {
                org: Some(vec!["org1".into(), "org2".into()]),
                format: "tree".into(),
            },

        test_list_with_format:
            vec!["wok", "--list", "--format", "json"],
            InferredCommand::List {
                org: None,
                format: "json".into(),
            },

        test_fast_forward:
            vec!["wok", "--ff"],
            InferredCommand::FastForward {
                org: None,
            },

        test_fast_forward_with_org:
            vec!["wok", "--ff", "--org", "org1,org2"],
            InferredCommand::FastForward {
                org: Some(vec!["org1".into(), "org2".into()]),
            },

        test_export:
            vec!["wok", "--export"],
            InferredCommand::Export,

        test_import_with_file:
            vec!["wok", "--import", "file.txt"],
            InferredCommand::Import {
                input: "file.txt".into(),
            },

        test_import_stdin:
            vec!["wok", "--import"],
            InferredCommand::Import {
                input: "-".into(),
            },

        test_dashboard_no_args:
            vec!["wok"],
            InferredCommand::Dashboard,

        test_setup_flag:
            vec!["wok", "--setup"],
            InferredCommand::Setup { shell: None, manual: false },

        test_setup_with_shell:
            vec!["wok", "--shell", "bash"],
            InferredCommand::Setup { shell: Some("bash".into()), manual: false },

        test_setup_with_manual:
            vec!["wok", "--manual"],
            InferredCommand::Setup { shell: None, manual: true },

        test_setup_with_shell_and_manual:
            vec!["wok", "--shell", "zsh", "--manual"],
            InferredCommand::Setup { shell: Some("zsh".into()), manual: true },

    }

    cli_test_error! {
        test_conflict_list_and_ff:
            vec!["wok", "--list", "--ff"]

        test_conflict_list_and_project:
            vec!["wok", "--list", "my-project"]

        test_conflict_ff_and_project:
            vec!["wok", "--ff", "my-project"]

        test_conflict_setup_and_list:
            vec!["wok", "--shell", "bash", "--list"]

        test_conflict_setup_and_ff:
            vec!["wok", "--manual", "--ff"]

        test_conflict_setup_and_export:
            vec!["wok", "--shell", "bash", "--export"]

        test_conflict_setup_and_import:
            vec!["wok", "--manual", "--import", "file.txt"]

        test_conflict_setup_and_project:
            vec!["wok", "--shell", "zsh", "user/repo"]

        test_conflict_list_and_export:
            vec!["wok", "--list", "--export"]

        test_conflict_list_and_import:
            vec!["wok", "--list", "--import", "file.txt"]

        test_conflict_ff_and_export:
            vec!["wok", "--ff", "--export"]

        test_conflict_ff_and_import:
            vec!["wok", "--ff", "--import", "file.txt"]

        test_conflict_export_and_import:
            vec!["wok", "--export", "--import", "file.txt"]

        test_conflict_export_and_project:
            vec!["wok", "--export", "user/repo"]

        test_conflict_import_and_project:
            vec!["wok", "--import", "file.txt", "user/repo"]
    }
}
