use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use std::env;

/// The inferred commands that `wok` can execute
#[derive(Debug, Clone, PartialEq)]
pub enum InferredCommand {
    /// Dashboard (no arguments)
    Dashboard,
    /// Setup command
    Setup { shell: Option<String>, manual: bool },
    /// Clean command (remove setup changes)
    Clean { shell: Option<String> },
    /// Remove command (delete project from workspace)
    Remove { search: String },
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
    /// Scrape a directory for git repositories
    Scrape {
        path: String,
        org: Option<Vec<String>>,
        export_mode: bool,
        import_mode: bool,
    },
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
                    .help("Export the workspace or scrape results to stdout")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("import")
                    .long("import")
                    .help("Import a workspace from a file, or import scraped repositories")
                    .value_name("FILE")
                    .num_args(0..=1) // allow 0 or 1 argument
                    .default_missing_value("-"), // if no arg given, treat as "-"
            )
            .arg(
                Arg::new("scrape")
                    .long("scrape")
                    .help("Scrape a directory recursively for git repositories")
                    .value_name("PATH")
                    .num_args(1),
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
                Arg::new("clean")
                    .long("clean")
                    .help("Remove all setup changes (delete files and remove lines from profile)")
                    .action(ArgAction::SetTrue)
                    .conflicts_with_all(&["setup", "manual", "list", "fast-forward", "export", "import", "project", "scrape", "rm"]),
            )
            .arg(
                Arg::new("rm")
                    .long("rm")
                    .help("Remove a project from the workspace")
                    .value_name("PROJECT")
                    .num_args(1)
                    .conflicts_with_all(&["setup", "clean", "list", "fast-forward", "export", "import", "project", "scrape"]),
            )
            .arg(
                Arg::new("shell")
                    .short('s')
                    .long("shell")
                    .help("Shell type for setup/clean (e.g. bash, zsh, fish)")
                    .conflicts_with_all(&["list", "fast-forward", "export", "import", "project", "scrape"]),
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
                ArgGroup::new("setup_options")
                    .args(&["setup", "manual"])
                    .required(false)
                    .multiple(true)
                    .conflicts_with_all(&["list", "fast-forward", "export", "import", "project", "scrape", "clean", "rm"]),
            )
    }

    /// Parse arguments and infer the command
    pub fn parse() -> Result<InferredCommand, String> {
        // Extract trailing args (after --) before clap consumes them
        let raw_args: Vec<String> = env::args().collect();
        let trailing: Vec<String> = if let Some(pos) = raw_args.iter().position(|a| a == "--") {
            raw_args[pos + 1..].to_vec()
        } else {
            vec![]
        };

        // Pass only args before -- to clap
        let clap_args: Vec<String> = if let Some(pos) = raw_args.iter().position(|a| a == "--") {
            raw_args[..pos].to_vec()
        } else {
            raw_args
        };

        let matches = Self::build().get_matches_from(clap_args);
        Self::infer_command_inner(&matches, trailing)
    }

    /// Infer the command from parsed arguments
    fn infer_command_inner(
        matches: &ArgMatches,
        trailing: Vec<String>,
    ) -> Result<InferredCommand, String> {
        // Handle scrape command first (since it can combine with export/import)
        if let Some(scrape_path) = matches.get_one::<String>("scrape") {
            // Scrape cannot be combined with list, ff, or project
            if matches.get_flag("list") {
                return Err("Cannot use --scrape with --list".to_string());
            }
            if matches.get_flag("fast-forward") {
                return Err("Cannot use --scrape with --ff".to_string());
            }
            if matches.contains_id("project") {
                return Err("Cannot use --scrape with a project argument".to_string());
            }

            let org = matches
                .get_many::<String>("org")
                .map(|v| v.cloned().collect());

            // Check if --export is specified with scrape
            let export_mode = matches.get_flag("export");

            // Check if --import is specified with scrape
            let import_mode = matches.contains_id("import") && !export_mode;

            // Cannot have both export and import with scrape
            if export_mode && import_mode {
                return Err("Cannot use both --export and --import with --scrape".to_string());
            }

            return Ok(InferredCommand::Scrape {
                path: scrape_path.clone(),
                org,
                export_mode,
                import_mode,
            });
        }

        // Handle flag-based commands
        if matches.get_flag("list") {
            // List cannot be combined with export/import
            if matches.get_flag("export") {
                return Err("Cannot use --list with --export".to_string());
            }
            if matches.contains_id("import") {
                return Err("Cannot use --list with --import".to_string());
            }

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
            // Fast-forward cannot be combined with export/import
            if matches.get_flag("export") {
                return Err("Cannot use --ff with --export".to_string());
            }
            if matches.contains_id("import") {
                return Err("Cannot use --ff with --import".to_string());
            }

            return Ok(InferredCommand::FastForward {
                org: matches
                    .get_many::<String>("org")
                    .map(|v| v.cloned().collect()),
            });
        }

        // Export without scrape
        if matches.get_flag("export") {
            // Cannot be combined with import or project
            if matches.contains_id("import") {
                return Err("Cannot use --export with --import".to_string());
            }
            if matches.contains_id("project") {
                return Err("Cannot use --export with a project argument".to_string());
            }
            return Ok(InferredCommand::Export);
        }

        // Import without scrape
        if let Some(import_file) = matches.get_one::<String>("import") {
            // Cannot be combined with project
            if matches.contains_id("project") {
                return Err("Cannot use --import with a project argument".to_string());
            }
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

        // Check if clean options are provided
        if matches.get_flag("clean") {
            return Ok(InferredCommand::Clean {
                shell: matches.get_one::<String>("shell").cloned(),
            });
        }

        // Check if remove options are provided
        if let Some(search) = matches.get_one::<String>("rm") {
            return Ok(InferredCommand::Remove {
                search: search.clone(),
            });
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
                    let command = WokCli::infer_command_inner(&matches, vec![]).unwrap();
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
                    let match_result = WokCli::build().try_get_matches_from(args.clone());

                    // Either parsing should fail, or inference should fail
                    if let Ok(matches) = match_result {
                        let result = WokCli::infer_command_inner(&matches, vec![]);
                        assert!(result.is_err(), "Expected inference to fail for args: {:?}", args);
                    }
                    // If parsing failed, that's also acceptable
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

        test_scrape:
            vec!["wok", "--scrape", "/path/to/dir"],
            InferredCommand::Scrape {
                path: "/path/to/dir".into(),
                org: None,
                export_mode: false,
                import_mode: false,
            },

        test_scrape_with_org:
            vec!["wok", "--scrape", "/path/to/dir", "--org", "org1,org2"],
            InferredCommand::Scrape {
                path: "/path/to/dir".into(),
                org: Some(vec!["org1".into(), "org2".into()]),
                export_mode: false,
                import_mode: false,
            },

        test_scrape_with_export:
            vec!["wok", "--scrape", "/path/to/dir", "--export"],
            InferredCommand::Scrape {
                path: "/path/to/dir".into(),
                org: None,
                export_mode: true,
                import_mode: false,
            },

        test_scrape_with_import:
            vec!["wok", "--scrape", "/path/to/dir", "--import"],
            InferredCommand::Scrape {
                path: "/path/to/dir".into(),
                org: None,
                export_mode: false,
                import_mode: true,
            },

        test_scrape_with_org_and_export:
            vec!["wok", "--scrape", "/path/to/dir", "--org", "myorg", "--export"],
            InferredCommand::Scrape {
                path: "/path/to/dir".into(),
                org: Some(vec!["myorg".into()]),
                export_mode: true,
                import_mode: false,
            },

        test_remove:
            vec!["wok", "--rm", "myproject"],
            InferredCommand::Remove {
                search: "myproject".into(),
            },

    }

    cli_test_error! {
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

        test_conflict_setup_and_scrape:
            vec!["wok", "--shell", "bash", "--scrape", "/path"]

        test_conflict_list_and_export:
            vec!["wok", "--list", "--export"]

        test_conflict_list_and_import:
            vec!["wok", "--list", "--import"]

        test_conflict_ff_and_export:
            vec!["wok", "--ff", "--export"]

        test_conflict_ff_and_import:
            vec!["wok", "--ff", "--import"]

        test_conflict_export_and_import:
            vec!["wok", "--export", "--import"]

        test_conflict_export_and_project:
            vec!["wok", "--export", "my-project"]

        test_conflict_import_and_project:
            vec!["wok", "--import", "file.txt", "my-project"]

        test_conflict_scrape_and_list:
            vec!["wok", "--scrape", "/path", "--list"]

        test_conflict_scrape_and_ff:
            vec!["wok", "--scrape", "/path", "--ff"]

        test_conflict_scrape_and_project:
            vec!["wok", "--scrape", "/path", "my-project"]

        test_conflict_remove_and_setup:
            vec!["wok", "--rm", "myproject", "--setup"]

        test_conflict_remove_and_clean:
            vec!["wok", "--rm", "myproject", "--clean"]

        test_conflict_remove_and_list:
            vec!["wok", "--rm", "myproject", "--list"]

        test_conflict_remove_and_project:
            vec!["wok", "--rm", "myproject", "another-project"]
    }
}
