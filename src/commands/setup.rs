use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use wok::lib::shell::{
    detect_shell, prepare_shell, validate_shell, ShellSetupItem, ShellSetupItemName,
    ShellSetupItemStatus, SupportedShells,
};

pub fn handle(shell: &Option<String>, manual: bool) -> Result<(), Box<dyn std::error::Error>> {
    let shell_type = match shell {
        Some(s) => validate_shell(s)?,
        None => detect_shell()?,
    };

    let binary_path = env::current_exe()?.to_string_lossy().to_string();
    let home_dir = dirs::home_dir()
        .ok_or("Unable to find home directory")?
        .to_string_lossy()
        .to_string();

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

    if manual {
        print_instructions(&shell_type, &items)?;
    } else {
        install(items)?;
    }

    Ok(())
}

fn install(items: Vec<ShellSetupItem>) -> Result<(), Box<dyn std::error::Error>> {
    for item in items {
        if item.status == ShellSetupItemStatus::Done {
            println!("{} is already set up.", item.name);
            continue;
        }

        if item.entire_file {
            // write item.content to item.target
            let target_path = std::path::Path::new(&item.target);
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut file = std::fs::File::create(&item.target)?;
            file.write_all(item.content.as_bytes())?;
            println!("Wrote {} to {}.", item.name, &item.target);
        } else {
            // append item.content to item.target
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&item.target)?;
            file.write_all(item.content.as_bytes())?;
            println!("Appended {} to {}.", item.name, &item.target);
        }
    }
    Ok(())
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

fn print_instructions(
    shell: &SupportedShells,
    items: &[ShellSetupItem],
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract wokrc info from items
    let wokrc_item = items
        .iter()
        .find(|item| matches!(item.name, ShellSetupItemName::WokrcFile))
        .ok_or("WokrcFile item not found")?;

    let wokrc_path = &wokrc_item.target;
    let wokrc_script = &wokrc_item.content;

    // Extract autocomplete info from items
    let autocomplete_item = items
        .iter()
        .find(|item| matches!(item.name, ShellSetupItemName::AutocompleteFile))
        .ok_or("AutocompleteFile item not found")?;

    let autocomplete_script = &autocomplete_item.content;
    let autocomplete_path = &autocomplete_item.target;

    // Extract shell rc info from items
    let wokrc_config_item = items
        .iter()
        .find(|item| matches!(item.name, ShellSetupItemName::WokrcConfiguration))
        .ok_or("WokrcConfiguration item not found")?;

    let shell_rc_file = &wokrc_config_item.target;
    let autocomplete_rc_lines = &wokrc_config_item.content;

    // Get shell-specific display strings
    let (autocomplete_source_cmd, display_autocomplete_path) = match shell {
        SupportedShells::Bash => {
            let source_cmd = format!("$> source {}", autocomplete_path);
            (source_cmd, autocomplete_path.clone())
        }
        SupportedShells::Zsh => {
            let source_cmd =
                "$> fpath=(~/.zsh/completions $fpath)\n$> autoload -Uz compinit && compinit"
                    .to_string();
            let display_path = format!(
                "{} (create ~/.zsh/completions directory first)",
                autocomplete_path
            );
            (source_cmd, display_path)
        }
    };

    let raw = include_str!("../../templates/manual_setup.txt");
    let instructions = compile_template(
        raw,
        &[
            ("WOKRC_PATH".to_string(), wokrc_path.clone()),
            ("WOKRC_NAME".to_string(), ".wokrc".to_string()),
            ("WOKRC_SCRIPT".to_string(), wokrc_script.to_string()),
            ("AUTOCOMPLETE_PATH".to_string(), display_autocomplete_path),
            (
                "AUTOCOMPLETE_SCRIPT".to_string(),
                autocomplete_script.to_string(),
            ),
            (
                "AUTOCOMPLETE_SOURCE_CMD".to_string(),
                autocomplete_source_cmd,
            ),
            (
                "AUTOCOMPLETE_RC_LINES".to_string(),
                autocomplete_rc_lines.clone(),
            ),
            ("SHELL_RC_FILE".to_string(), shell_rc_file.to_string()),
            ("SHELL_TYPE".to_string(), shell.to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
    )?;

    less(&instructions)?;
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
        let wokrc_bash = home_dir.join(".wokrc.bash");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let binary_path = "/fake/binary/path".to_string();
        let items = prepare_shell(
            &SupportedShells::Bash,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();

        let autocomplete_path = home_dir.join(".bash_completion.d/wok");
        assert!(autocomplete_path.exists());

        let content = fs::read_to_string(&autocomplete_path).unwrap();
        assert!(content.contains("_wok_completion"));
        assert!(content.contains("complete -F _wok_completion wok"));

        assert!(wokrc_bash.exists());
        let bashrc_content = fs::read_to_string(&bashrc).unwrap();
        assert!(bashrc_content.contains("[ -f"));
        assert!(bashrc_content.contains(".wokrc.bash"));
        assert!(bashrc_content.contains("source"));
    }

    #[test]
    fn test_install_autocomplete_zsh() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let zshrc = home_dir.join(".zshrc");
        let wokrc_zsh = home_dir.join(".wokrc.zsh");
        fs::write(&zshrc, "# existing zshrc\n").unwrap();

        let binary_path = "/fake/binary/path".to_string();
        let items = prepare_shell(
            &SupportedShells::Zsh,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();

        let completions_dir = home_dir.join(".zsh/completions");
        assert!(completions_dir.exists());

        let autocomplete_path = completions_dir.join("_wok");
        assert!(autocomplete_path.exists());

        let content = fs::read_to_string(&autocomplete_path).unwrap();
        assert!(content.contains("#compdef wok"));
        assert!(content.contains("_wok()"));

        assert!(wokrc_zsh.exists());
        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        assert!(zshrc_content.contains("[ -f"));
        assert!(zshrc_content.contains(".wokrc.zsh"));
        assert!(zshrc_content.contains("source"));
    }

    #[test]
    fn test_install_autocomplete_bash_idempotent() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let bashrc = home_dir.join(".bashrc");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let binary_path = "/fake/binary/path".to_string();
        let items = prepare_shell(
            &SupportedShells::Bash,
            binary_path.clone(),
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();
        let items = prepare_shell(
            &SupportedShells::Bash,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();

        let bashrc_content = fs::read_to_string(&bashrc).unwrap();
        let source_count = bashrc_content.matches(".wokrc.bash").count();
        assert_eq!(source_count, 2);
    }

    #[test]
    fn test_install_autocomplete_zsh_idempotent() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let zshrc = home_dir.join(".zshrc");
        fs::write(&zshrc, "# existing zshrc\n").unwrap();

        let binary_path = "/fake/binary/path".to_string();
        let items = prepare_shell(
            &SupportedShells::Zsh,
            binary_path.clone(),
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();
        let items = prepare_shell(
            &SupportedShells::Zsh,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();

        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        // The wokrc config line contains .wokrc.zsh twice: "[ -f path/.wokrc.zsh ] && source path/.wokrc.zsh"
        // So we expect exactly 2 occurrences total (not 4, which would mean it was added twice)
        let source_count = zshrc_content.matches(".wokrc.zsh").count();
        assert_eq!(source_count, 2);
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

        let binary_path = "/fake/binary/path".to_string();
        let items = prepare_shell(
            &SupportedShells::Zsh,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items).unwrap();

        let zshrc_content = fs::read_to_string(&zshrc).unwrap();
        assert!(zshrc_content.contains("[ -f"));
        assert!(zshrc_content.contains(".wokrc.zsh"));

        let source_count = zshrc_content.matches(".wokrc.zsh").count();
        assert_eq!(source_count, 2);
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

    #[test]
    fn test_setup_truly_idempotent() {
        let _guard = HOME_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();

        let bashrc = home_dir.join(".bashrc");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let binary_path = "/fake/binary/path".to_string();

        // First run
        let items1 = prepare_shell(
            &SupportedShells::Bash,
            binary_path.clone(),
            home_dir.to_str().unwrap(),
        )
        .unwrap();
        install(items1).unwrap();

        let wokrc_path = home_dir.join(".wokrc.bash");
        let first_content = fs::read_to_string(&wokrc_path).unwrap();
        let first_modified = fs::metadata(&wokrc_path).unwrap().modified().unwrap();

        // Second run - should not rewrite files
        let items2 = prepare_shell(
            &SupportedShells::Bash,
            binary_path,
            home_dir.to_str().unwrap(),
        )
        .unwrap();

        // Check that wokrc file is marked as Done
        let wokrc_item = items2
            .iter()
            .find(|item| matches!(item.name, ShellSetupItemName::WokrcFile))
            .unwrap();

        assert_eq!(
            wokrc_item.status,
            ShellSetupItemStatus::Done,
            "Wokrc file should be Done on second run"
        );

        install(items2).unwrap();

        let second_content = fs::read_to_string(&wokrc_path).unwrap();
        let second_modified = fs::metadata(&wokrc_path).unwrap().modified().unwrap();

        // Content should be identical
        assert_eq!(
            first_content, second_content,
            "Wokrc file content should not change"
        );

        // File should not have been rewritten (modification time should be the same)
        assert_eq!(
            first_modified, second_modified,
            "Wokrc file should not be rewritten on second run"
        );
    }
}
