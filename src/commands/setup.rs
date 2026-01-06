use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use wok::lib::constants::{GOTO_MARKER, MULTIPLE_MATCHES_MARKER};
use wok::lib::shell::{detect_shell, SupportedShells};
struct WokrcScriptParams {
    binary_path: String,
    goto_marker: String,
    multiple_matches_marker: String,
}

pub fn handle(shell: &Option<String>, manual: bool) -> Result<(), Box<dyn std::error::Error>> {
    let shell_type = match shell {
        Some(s) => validate_shell(s)?,
        None => detect_shell()?,
    };

    let binary_path = env::current_exe()?.to_string_lossy().to_string();
    let goto_marker = GOTO_MARKER.to_string();
    let multiple_matches_marker = MULTIPLE_MATCHES_MARKER.to_string();
    let params = WokrcScriptParams {
        binary_path,
        goto_marker,
        multiple_matches_marker,
    };

    let wokrc_script = compile_wokrc_script(&shell_type, params)?;

    if manual {
        print_instructions(&shell_type, &wokrc_script)?;
    } else {
        install_wokrc_script(&wokrc_script, &shell_type)?;
        install_autocomplete_script(&shell_type)?;
    }

    Ok(())
}

fn validate_shell(shell: &str) -> Result<SupportedShells, Box<dyn std::error::Error>> {
    match shell.to_lowercase().as_str() {
        "bash" => Ok(SupportedShells::Bash),
        "zsh" => Ok(SupportedShells::Zsh),
        _ => Err(Box::from(format!("Unsupported shell type: {}", shell))),
    }
}

fn compile_wokrc_script(
    shell: &SupportedShells,
    params: WokrcScriptParams,
) -> Result<String, Box<dyn std::error::Error>> {
    let raw = match shell {
        SupportedShells::Zsh => include_str!("../../templates/.wokrc.zsh"),
        SupportedShells::Bash => include_str!("../../templates/.wokrc.bash"),
    };

    let compiled_script: String = compile_template(
        raw,
        &[
            (
                "WOK_BINARY_PATH".to_string(),
                params.binary_path.to_string(),
            ),
            (
                "WOK_GOTO_MARKER".to_string(),
                params.goto_marker.to_string(),
            ),
            (
                "WOK_MULTIPLE_MATCHES_MARKER".to_string(),
                params.multiple_matches_marker.to_string(),
            ),
        ]
        .iter()
        .cloned()
        .collect(),
    )?;

    Ok(compiled_script)
}

fn compile_template(
    template: &str,
    params: &std::collections::HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut result = template.to_string();
    for (key, value) in params {
        // Tags are in the format __KEY__
        result = result.replace(&format!("__{}__", key), value);
    }
    Ok(result)
}

fn install_wokrc_script(
    script: &str,
    shell: &SupportedShells,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Installing wokrc script for {} shell...", shell.to_string());

    let wokrc_path = wokrc_path(&shell)?;
    let mut file = std::fs::File::create(&wokrc_path)?;
    file.write_all(script.as_bytes())?;
    println!(".wokrc script installed at: {}.", wokrc_path);

    let rc_file = shell_rc_file(&shell)?;
    let mut rc_file_content = std::fs::read_to_string(&rc_file)?;
    if !rc_file_content.contains(&format!("[ -f {} ] && source {}", wokrc_path, wokrc_path)) {
        rc_file_content.push_str(&format!(
            "\n[ -f {} ] && source {}\n",
            wokrc_path, wokrc_path
        ));
        std::fs::write(&rc_file, rc_file_content)?;
        println!("Added source command to {}.", rc_file);
    } else {
        println!("Source command already exists in {}, skipping.", rc_file);
    }

    Ok(())
}

fn print_instructions(
    shell: &SupportedShells,
    wokrc_script: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let wokrc_path = wokrc_path(&shell)?;

    // Get autocomplete script and paths based on shell type
    let (autocomplete_script, autocomplete_path, autocomplete_source_cmd, autocomplete_rc_lines) =
        match shell {
            SupportedShells::Bash => {
                let script = include_str!("../../templates/autocomplete.bash");
                let path = "$HOME/.wok_completion_bash".to_string();
                let source_cmd = "$> source ~/.wok_completion_bash".to_string();
                let rc_lines =
                    "[ -f ~/.wok_completion_bash ] && source ~/.wok_completion_bash".to_string();
                (script, path, source_cmd, rc_lines)
            }
            SupportedShells::Zsh => {
                let script = include_str!("../../templates/autocomplete.zsh");
                let path =
                    "$HOME/.zsh/completions/_wok (create ~/.zsh/completions directory first)"
                        .to_string();
                let source_cmd =
                    "$> fpath=(~/.zsh/completions $fpath)\n$> autoload -Uz compinit && compinit"
                        .to_string();
                let rc_lines =
                    "fpath=(~/.zsh/completions $fpath)\nautoload -Uz compinit && compinit"
                        .to_string();
                (script, path, source_cmd, rc_lines)
            }
        };

    let raw = include_str!("../../templates/manual_setup.txt");
    let instructions = compile_template(
        raw,
        &[
            ("WOKRC_PATH".to_string(), wokrc_path.clone()),
            ("WOKRC_NAME".to_string(), ".wokrc".to_string()),
            ("WOKRC_SCRIPT".to_string(), wokrc_script.to_string()),
            ("AUTOCOMPLETE_PATH".to_string(), autocomplete_path),
            (
                "AUTOCOMPLETE_SCRIPT".to_string(),
                autocomplete_script.to_string(),
            ),
            (
                "AUTOCOMPLETE_SOURCE_CMD".to_string(),
                autocomplete_source_cmd,
            ),
            ("AUTOCOMPLETE_RC_LINES".to_string(), autocomplete_rc_lines),
            (
                "SHELL_RC_FILE".to_string(),
                shell_rc_file(&shell)?.to_string(),
            ),
            ("SHELL_TYPE".to_string(), shell.to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
    )?;

    less(&instructions)?;
    Ok(())
}

fn wokrc_path(shell: &SupportedShells) -> Result<String, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    let wokrc_path = match shell {
        SupportedShells::Bash => home_dir.join(".wokrc_bash"),
        SupportedShells::Zsh => home_dir.join(".wokrc_zsh"),
    };
    Ok(wokrc_path.to_string_lossy().to_string())
}

fn shell_rc_file(shell: &SupportedShells) -> Result<String, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
    let rc_file = match shell {
        SupportedShells::Bash => home_dir.join(".bashrc"),
        SupportedShells::Zsh => home_dir.join(".zshrc"),
    };
    Ok(rc_file.to_string_lossy().to_string())
}

fn install_autocomplete_script(shell: &SupportedShells) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Installing autocomplete script for {} shell...",
        shell.to_string()
    );

    match shell {
        SupportedShells::Bash => {
            let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
            let autocomplete_path = home_dir.join(".wok_completion_bash");
            let autocomplete_script = include_str!("../../templates/autocomplete.bash");

            // Write autocomplete script
            let mut file = std::fs::File::create(&autocomplete_path)?;
            file.write_all(autocomplete_script.as_bytes())?;
            println!(
                "Autocomplete script installed at: {}.",
                autocomplete_path.display()
            );

            // Add source command to .bashrc
            let rc_file = shell_rc_file(&shell)?;
            let mut rc_file_content = std::fs::read_to_string(&rc_file)?;
            let source_line = format!(
                "[ -f {} ] && source {}",
                autocomplete_path.display(),
                autocomplete_path.display()
            );

            if !rc_file_content.contains(&source_line) {
                rc_file_content.push_str(&format!("\n{}\n", source_line));
                std::fs::write(&rc_file, rc_file_content)?;
                println!("Added autocomplete source command to {}.", rc_file);
            } else {
                println!(
                    "Autocomplete source command already exists in {}, skipping.",
                    rc_file
                );
            }
        }
        SupportedShells::Zsh => {
            let home_dir = dirs::home_dir().ok_or("Unable to find home directory")?;
            let completions_dir = home_dir.join(".zsh/completions");
            let autocomplete_path = completions_dir.join("_wok");
            let autocomplete_script = include_str!("../../templates/autocomplete.zsh");

            // Create completions directory if it doesn't exist
            std::fs::create_dir_all(&completions_dir)?;

            // Write autocomplete script
            let mut file = std::fs::File::create(&autocomplete_path)?;
            file.write_all(autocomplete_script.as_bytes())?;
            println!(
                "Autocomplete script installed at: {}.",
                autocomplete_path.display()
            );

            // Add fpath and compinit to .zshrc
            let rc_file = shell_rc_file(&shell)?;
            let mut rc_file_content = std::fs::read_to_string(&rc_file)?;
            let fpath_line = format!("fpath=(~/.zsh/completions $fpath)");
            let compinit_line = "autoload -Uz compinit && compinit";

            let mut changes_made = false;
            if !rc_file_content.contains(&fpath_line) {
                rc_file_content.push_str(&format!("\n{}\n", fpath_line));
                changes_made = true;
            }

            if !rc_file_content.contains("compinit") {
                rc_file_content.push_str(&format!("{}\n", compinit_line));
                changes_made = true;
            }

            if changes_made {
                std::fs::write(&rc_file, rc_file_content)?;
                println!("Added autocomplete configuration to {}.", rc_file);
            } else {
                println!(
                    "Autocomplete configuration already exists in {}, skipping.",
                    rc_file
                );
            }
        }
    }

    Ok(())
}

fn less(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn 'less' with stdin piped
    let mut child = Command::new("less")
        .arg("--wordwrap")
        .stdin(Stdio::piped())
        .spawn()?;

    // Get a handle to less's stdin
    if let Some(stdin) = child.stdin.as_mut() {
        // Write text to less
        stdin.write_all(text.as_bytes())?;
    }

    // Wait for less to exit
    child.wait()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use std::fs;
    use std::sync::Mutex;

    // Mutex to ensure tests that modify HOME run serially
    static HOME_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_install_autocomplete_bash() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let bashrc = home_dir.join(".bashrc");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        install_autocomplete_script(&SupportedShells::Bash).unwrap();

        let autocomplete_path = home_dir.join(".wok_completion_bash");
        assert!(autocomplete_path.exists());

        let content = fs::read_to_string(&autocomplete_path).unwrap();
        assert!(content.contains("_wok_completion"));
        assert!(content.contains("complete -F _wok_completion wok"));

        let bashrc_content = fs::read_to_string(&bashrc).unwrap();
        assert!(bashrc_content.contains("[ -f"));
        assert!(bashrc_content.contains(".wok_completion_bash"));
        assert!(bashrc_content.contains("source"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_install_autocomplete_zsh() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let zshrc = home_dir.join(".zshrc");
        fs::write(&zshrc, "# existing zshrc\n").unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        install_autocomplete_script(&SupportedShells::Zsh).unwrap();

        let completions_dir = home_dir.join(".zsh/completions");
        assert!(completions_dir.exists());

        let autocomplete_path = completions_dir.join("_wok");
        assert!(autocomplete_path.exists());

        let content = fs::read_to_string(&autocomplete_path).unwrap();
        assert!(content.contains("#compdef wok"));
        assert!(content.contains("_wok()"));

        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        assert!(zshrc_content.contains("fpath=(~/.zsh/completions $fpath)"));
        assert!(zshrc_content.contains("compinit"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_install_autocomplete_bash_idempotent() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let bashrc = home_dir.join(".bashrc");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        install_autocomplete_script(&SupportedShells::Bash).unwrap();
        install_autocomplete_script(&SupportedShells::Bash).unwrap();

        let bashrc_content = fs::read_to_string(&bashrc).unwrap();
        let source_count = bashrc_content.matches(".wok_completion_bash").count();
        assert_eq!(source_count, 2);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_install_autocomplete_zsh_idempotent() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let zshrc = home_dir.join(".zshrc");
        fs::write(&zshrc, "# existing zshrc\n").unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        install_autocomplete_script(&SupportedShells::Zsh).unwrap();
        install_autocomplete_script(&SupportedShells::Zsh).unwrap();

        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        let fpath_count = zshrc_content
            .matches("fpath=(~/.zsh/completions $fpath)")
            .count();
        assert_eq!(fpath_count, 1);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_install_autocomplete_zsh_existing_compinit() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let zshrc = home_dir.join(".zshrc");
        fs::write(
            &zshrc,
            "# existing zshrc\nautoload -Uz compinit\ncompinit\n",
        )
        .unwrap();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        install_autocomplete_script(&SupportedShells::Zsh).unwrap();

        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        assert!(zshrc_content.contains("fpath=(~/.zsh/completions $fpath)"));

        let compinit_count = zshrc_content.matches("compinit").count();
        assert_eq!(compinit_count, 2);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_print_instructions_includes_autocomplete_bash() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        let _shell = SupportedShells::Bash;
        let wokrc_script = "test wokrc script";

        let wokrc_path = format!("{}/.wokrc_bash", home_dir.display());
        let raw = include_str!("../../templates/manual_setup.txt");

        let autocomplete_script = include_str!("../../templates/autocomplete.bash");
        let autocomplete_path = "$HOME/.wok_completion_bash".to_string();
        let autocomplete_source_cmd = "$> source ~/.wok_completion_bash".to_string();
        let autocomplete_rc_lines =
            "[ -f ~/.wok_completion_bash ] && source ~/.wok_completion_bash".to_string();

        let instructions = compile_template(
            raw,
            &[
                ("WOKRC_PATH".to_string(), wokrc_path.clone()),
                ("WOKRC_NAME".to_string(), ".wokrc".to_string()),
                ("WOKRC_SCRIPT".to_string(), wokrc_script.to_string()),
                ("AUTOCOMPLETE_PATH".to_string(), autocomplete_path),
                (
                    "AUTOCOMPLETE_SCRIPT".to_string(),
                    autocomplete_script.to_string(),
                ),
                (
                    "AUTOCOMPLETE_SOURCE_CMD".to_string(),
                    autocomplete_source_cmd,
                ),
                ("AUTOCOMPLETE_RC_LINES".to_string(), autocomplete_rc_lines),
                (
                    "SHELL_RC_FILE".to_string(),
                    format!("{}/.bashrc", home_dir.display()),
                ),
                ("SHELL_TYPE".to_string(), "bash".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        )
        .unwrap();

        assert!(instructions.contains("_wok_completion"));
        assert!(instructions.contains(".wok_completion_bash"));
        assert!(instructions.contains("source ~/.wok_completion_bash"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_print_instructions_includes_autocomplete_zsh() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home_dir);

        let _shell = SupportedShells::Zsh;
        let wokrc_script = "test wokrc script";

        let wokrc_path = format!("{}/.wokrc_zsh", home_dir.display());
        let raw = include_str!("../../templates/manual_setup.txt");

        let autocomplete_script = include_str!("../../templates/autocomplete.zsh");
        let autocomplete_path =
            "$HOME/.zsh/completions/_wok (create ~/.zsh/completions directory first)".to_string();
        let autocomplete_source_cmd =
            "$> fpath=(~/.zsh/completions $fpath)\n$> autoload -Uz compinit && compinit"
                .to_string();
        let autocomplete_rc_lines =
            "fpath=(~/.zsh/completions $fpath)\nautoload -Uz compinit && compinit".to_string();

        let instructions = compile_template(
            raw,
            &[
                ("WOKRC_PATH".to_string(), wokrc_path.clone()),
                ("WOKRC_NAME".to_string(), ".wokrc".to_string()),
                ("WOKRC_SCRIPT".to_string(), wokrc_script.to_string()),
                ("AUTOCOMPLETE_PATH".to_string(), autocomplete_path),
                (
                    "AUTOCOMPLETE_SCRIPT".to_string(),
                    autocomplete_script.to_string(),
                ),
                (
                    "AUTOCOMPLETE_SOURCE_CMD".to_string(),
                    autocomplete_source_cmd,
                ),
                ("AUTOCOMPLETE_RC_LINES".to_string(), autocomplete_rc_lines),
                (
                    "SHELL_RC_FILE".to_string(),
                    format!("{}/.zshrc", home_dir.display()),
                ),
                ("SHELL_TYPE".to_string(), "zsh".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        )
        .unwrap();

        assert!(instructions.contains("#compdef wok"));
        assert!(instructions.contains("_wok()"));
        assert!(instructions.contains("fpath=(~/.zsh/completions $fpath)"));
        assert!(instructions.contains("compinit"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_validate_shell() {
        assert!(matches!(validate_shell("bash"), Ok(SupportedShells::Bash)));
        assert!(matches!(validate_shell("BASH"), Ok(SupportedShells::Bash)));
        assert!(matches!(validate_shell("zsh"), Ok(SupportedShells::Zsh)));
        assert!(matches!(validate_shell("ZSH"), Ok(SupportedShells::Zsh)));
        assert!(validate_shell("fish").is_err());
        assert!(validate_shell("invalid").is_err());
    }
}
