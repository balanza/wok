use super::constants::{GOTO_MARKER, MULTIPLE_MATCHES_MARKER};
use super::templates::compile_template;
use std::{fmt::Display, process::Command};

pub enum SupportedShells {
    Bash,
    Zsh,
}

impl ToString for SupportedShells {
    fn to_string(&self) -> String {
        match self {
            SupportedShells::Bash => "bash".to_string(),
            SupportedShells::Zsh => "zsh".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShellSetupItemStatus {
    Done,
    Todo,
    OutOfDate,
}

#[derive(Debug)]
pub enum ShellSetupItemName {
    WokrcFile,
    WokrcConfiguration,
    AutocompleteFile,
}

impl Display for ShellSetupItemName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellSetupItemName::WokrcFile => write!(f, "Wokrc File"),
            ShellSetupItemName::WokrcConfiguration => write!(f, "Wokrc Configuration"),
            ShellSetupItemName::AutocompleteFile => write!(f, "Autocomplete File"),
        }
    }
}

#[derive(Debug)]
pub struct ShellSetupItem {
    pub name: ShellSetupItemName,
    pub target: String,
    pub content: String,
    pub entire_file: bool,
    pub status: ShellSetupItemStatus,
}

struct WokrcScriptParams {
    binary_path: String,
    goto_marker: String,
    multiple_matches_marker: String,
}

pub fn detect_shell() -> Result<SupportedShells, Box<dyn std::error::Error>> {
    // Get current process PID
    let pid = std::process::id();

    // Use ps to get PPID of current process
    let ppid_output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "ppid="])
        .output()
        .expect("failed to execute ps to get ppid");

    let ppid_str = str::from_utf8(&ppid_output.stdout).unwrap().trim();

    // Use ps to get command name of parent process
    let parent_cmd_output = Command::new("ps")
        .args(&["-p", ppid_str, "-o", "comm="])
        .output()
        .expect("failed to execute ps to get parent command name");

    let parent_cmd = str::from_utf8(&parent_cmd_output.stdout).unwrap().trim();

    match parent_cmd {
        "bash" => Ok(SupportedShells::Bash),
        "zsh" => Ok(SupportedShells::Zsh),
        _ => Err(Box::from(format!("Unsupported shell type: {}", parent_cmd))),
    }
}

pub fn prepare_shell(
    shell: &SupportedShells,
    binary_path: String,
    home_dir: &str,
) -> Result<Vec<ShellSetupItem>, Vec<Box<dyn std::error::Error>>> {
    let items = vec![
        wokrc_file_setup_item(shell, binary_path.clone(), home_dir),
        wokrc_configuration_setup_item(shell, home_dir),
        autocomplete_file_setup_item(shell, home_dir),
    ];

    collect_results(items)
}

fn wokrc_file_setup_item(
    shell: &SupportedShells,
    binary_path: String,
    home_dir: &str,
) -> Result<ShellSetupItem, Box<dyn std::error::Error>> {
    let wokrc_file_path = wokrc_file_path(&shell, home_dir);
    let wokrc_content = match compile_wokrc_script(
        &shell,
        WokrcScriptParams {
            binary_path: binary_path.clone(),
            goto_marker: GOTO_MARKER.to_string(),
            multiple_matches_marker: MULTIPLE_MATCHES_MARKER.to_string(),
        },
    ) {
        Ok(content) => content,
        Err(e) => {
            return Err(Box::from(format!(
                "Failed to compile wokrc script for {}: {}",
                shell.to_string(),
                e
            )))
        }
    };

    let status = match check_file_status(&wokrc_file_path, &wokrc_content) {
        Ok(status) => status,
        Err(e) => {
            return Err(Box::from(format!(
                "Failed to check file status for {}: {}",
                wokrc_file_path, e
            )))
        }
    };

    Ok(ShellSetupItem {
        name: ShellSetupItemName::WokrcFile,
        target: wokrc_file_path,
        content: wokrc_content,
        entire_file: true,
        status,
    })
}

fn autocomplete_file_setup_item(
    shell: &SupportedShells,
    home_dir: &str,
) -> Result<ShellSetupItem, Box<dyn std::error::Error>> {
    let autocomplete_file_path = autocomplete_file_path(&shell, home_dir);
    let autocomplete_content = match compile_autocomplete_script(&shell) {
        Ok(content) => content,
        Err(e) => {
            return Err(Box::from(format!(
                "Failed to compile autocomplete script for {}: {}",
                shell.to_string(),
                e
            )))
        }
    };

    let status = match check_file_status(&autocomplete_file_path, &autocomplete_content) {
        Ok(status) => status,
        Err(e) => {
            return Err(Box::from(format!(
                "Failed to check file status for {}: {}",
                autocomplete_file_path, e
            )))
        }
    };

    Ok(ShellSetupItem {
        name: ShellSetupItemName::AutocompleteFile,
        target: autocomplete_file_path,
        content: autocomplete_content,
        entire_file: true,
        status,
    })
}

fn wokrc_configuration_setup_item(
    shell: &SupportedShells,
    home_dir: &str,
) -> Result<ShellSetupItem, Box<dyn std::error::Error>> {
    let shellrc_file_path = shellrc_file_path(&shell, home_dir);
    let wokrc_path = wokrc_file_path(&shell, home_dir);
    let wokrc_configuration = format!("[ -f {} ] && source {}", wokrc_path, wokrc_path);

    let status = match check_file_contains_status(&shellrc_file_path, &wokrc_configuration) {
        Ok(status) => status,
        Err(e) => {
            return Err(Box::from(format!(
                "Failed to check file status for {}: {}",
                shellrc_file_path, e
            )))
        }
    };

    Ok(ShellSetupItem {
        name: ShellSetupItemName::WokrcConfiguration,
        target: shellrc_file_path,
        content: wokrc_configuration,
        entire_file: false,
        status,
    })
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

fn wokrc_file_path(shell: &SupportedShells, home_dir: &str) -> String {
    match shell {
        SupportedShells::Zsh => format!("{}/.wokrc.zsh", home_dir),
        SupportedShells::Bash => format!("{}/.wokrc.bash", home_dir),
    }
}

fn compile_autocomplete_script(
    shell: &SupportedShells,
) -> Result<String, Box<dyn std::error::Error>> {
    let raw = match shell {
        SupportedShells::Zsh => include_str!("../../templates/autocomplete.zsh"),
        SupportedShells::Bash => include_str!("../../templates/autocomplete.bash"),
    };

    let compiled_script: String = compile_template(raw, &[].iter().cloned().collect())?;

    Ok(compiled_script)
}

fn autocomplete_file_path(shell: &SupportedShells, home_dir: &str) -> String {
    match shell {
        SupportedShells::Zsh => format!("{}/.zsh/completions/_wok", home_dir),
        SupportedShells::Bash => {
            format!("{}/.bash_completion.d/wok", home_dir)
        }
    }
}

fn check_file_status(
    file_path: &str,
    expected_content: &str,
) -> Result<ShellSetupItemStatus, Box<dyn std::error::Error>> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Ok(ShellSetupItemStatus::Todo);
    }

    let actual = std::fs::read_to_string(file_path).map_err(|e| -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", file_path, e),
        ))
    })?;

    if actual != expected_content {
        Ok(ShellSetupItemStatus::OutOfDate)
    } else {
        Ok(ShellSetupItemStatus::Done)
    }
}

fn check_file_contains_status(
    file_path: &str,
    expected_content: &str,
) -> Result<ShellSetupItemStatus, Box<dyn std::error::Error>> {
    let path = std::path::Path::new(file_path);
    if !path.exists() {
        return Err(Box::from(format!("File not found: {}", file_path)));
    }

    let actual = std::fs::read_to_string(file_path).map_err(|e| -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", file_path, e),
        ))
    })?;

    if actual.contains(expected_content) {
        Ok(ShellSetupItemStatus::Done)
    } else {
        Ok(ShellSetupItemStatus::Todo)
    }
}

fn shellrc_file_path(shell: &SupportedShells, home_dir: &str) -> String {
    match shell {
        SupportedShells::Zsh => format!("{}/.zshrc", home_dir),
        SupportedShells::Bash => format!("{}/.bashrc", home_dir),
    }
}

fn collect_results(
    items: Vec<Result<ShellSetupItem, Box<dyn std::error::Error>>>,
) -> Result<Vec<ShellSetupItem>, Vec<Box<dyn std::error::Error>>> {
    let mut oks: Vec<ShellSetupItem> = Vec::new();
    let mut errs: Vec<Box<dyn std::error::Error>> = Vec::new();

    for item in items {
        match item {
            Ok(v) => oks.push(v),
            Err(e) => errs.push(e),
        }
    }

    if errs.is_empty() {
        Ok(oks)
    } else {
        Err(errs)
    }
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
    fn test_supported_shells_to_string() {
        assert_eq!(SupportedShells::Bash.to_string(), "bash");
        assert_eq!(SupportedShells::Zsh.to_string(), "zsh");
    }

    #[test]
    fn test_shell_setup_item_name_display() {
        assert_eq!(
            format!("{}", ShellSetupItemName::WokrcFile),
            "Wokrc File"
        );
        assert_eq!(
            format!("{}", ShellSetupItemName::WokrcConfiguration),
            "Wokrc Configuration"
        );
        assert_eq!(
            format!("{}", ShellSetupItemName::AutocompleteFile),
            "Autocomplete File"
        );
    }

    #[test]
    fn test_check_file_status_todo() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let status = check_file_status(file_path.to_str().unwrap(), "content").unwrap();
        assert_eq!(status, ShellSetupItemStatus::Todo);
    }

    #[test]
    fn test_check_file_status_done() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "expected content").unwrap();

        let status = check_file_status(file_path.to_str().unwrap(), "expected content").unwrap();
        assert_eq!(status, ShellSetupItemStatus::Done);
    }

    #[test]
    fn test_check_file_status_out_of_date() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "old content").unwrap();

        let status = check_file_status(file_path.to_str().unwrap(), "new content").unwrap();
        assert_eq!(status, ShellSetupItemStatus::OutOfDate);
    }

    #[test]
    fn test_check_file_contains_status_done() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let status = check_file_contains_status(file_path.to_str().unwrap(), "line 2").unwrap();
        assert_eq!(status, ShellSetupItemStatus::Done);
    }

    #[test]
    fn test_check_file_contains_status_todo() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let status =
            check_file_contains_status(file_path.to_str().unwrap(), "line 4").unwrap();
        assert_eq!(status, ShellSetupItemStatus::Todo);
    }

    #[test]
    fn test_check_file_contains_status_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let result = check_file_contains_status(file_path.to_str().unwrap(), "content");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_wokrc_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();

        let bash_path = wokrc_file_path(&SupportedShells::Bash, home_dir);
        assert!(bash_path.ends_with("/.wokrc.bash"));

        let zsh_path = wokrc_file_path(&SupportedShells::Zsh, home_dir);
        assert!(zsh_path.ends_with("/.wokrc.zsh"));
    }

    #[test]
    fn test_shellrc_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();

        let bash_path = shellrc_file_path(&SupportedShells::Bash, home_dir);
        assert!(bash_path.ends_with("/.bashrc"));

        let zsh_path = shellrc_file_path(&SupportedShells::Zsh, home_dir);
        assert!(zsh_path.ends_with("/.zshrc"));
    }

    #[test]
    fn test_autocomplete_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();

        let bash_path = autocomplete_file_path(&SupportedShells::Bash, home_dir);
        assert!(bash_path.ends_with("/.bash_completion.d/wok"));

        let zsh_path = autocomplete_file_path(&SupportedShells::Zsh, home_dir);
        assert!(zsh_path.ends_with("/.zsh/completions/_wok"));
    }

    #[test]
    fn test_compile_wokrc_script_bash() {
        let params = WokrcScriptParams {
            binary_path: "/test/binary/path".to_string(),
            goto_marker: "TEST_GOTO::".to_string(),
            multiple_matches_marker: "TEST_MULTIPLE::".to_string(),
        };

        let script = compile_wokrc_script(&SupportedShells::Bash, params).unwrap();
        assert!(script.contains("/test/binary/path"));
        assert!(script.contains("TEST_GOTO::"));
        assert!(script.contains("TEST_MULTIPLE::"));
    }

    #[test]
    fn test_compile_wokrc_script_zsh() {
        let params = WokrcScriptParams {
            binary_path: "/test/binary/path".to_string(),
            goto_marker: "TEST_GOTO::".to_string(),
            multiple_matches_marker: "TEST_MULTIPLE::".to_string(),
        };

        let script = compile_wokrc_script(&SupportedShells::Zsh, params).unwrap();
        assert!(script.contains("/test/binary/path"));
        assert!(script.contains("TEST_GOTO::"));
        assert!(script.contains("TEST_MULTIPLE::"));
    }

    #[test]
    fn test_compile_autocomplete_script_bash() {
        let script = compile_autocomplete_script(&SupportedShells::Bash).unwrap();
        assert!(script.contains("_wok_completion"));
        assert!(script.contains("complete -F _wok_completion wok"));
    }

    #[test]
    fn test_compile_autocomplete_script_zsh() {
        let script = compile_autocomplete_script(&SupportedShells::Zsh).unwrap();
        assert!(script.contains("#compdef wok"));
        assert!(script.contains("_wok()"));
    }

    #[test]
    fn test_collect_results_all_ok() {
        let temp_dir = TempDir::new().unwrap();
        let item1 = Ok(ShellSetupItem {
            name: ShellSetupItemName::WokrcFile,
            target: temp_dir.path().join("test1.txt").to_str().unwrap().to_string(),
            content: "content1".to_string(),
            entire_file: true,
            status: ShellSetupItemStatus::Todo,
        });
        let item2 = Ok(ShellSetupItem {
            name: ShellSetupItemName::AutocompleteFile,
            target: temp_dir.path().join("test2.txt").to_str().unwrap().to_string(),
            content: "content2".to_string(),
            entire_file: true,
            status: ShellSetupItemStatus::Todo,
        });

        let result = collect_results(vec![item1, item2]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_collect_results_with_errors() {
        let temp_dir = TempDir::new().unwrap();
        let item1 = Ok(ShellSetupItem {
            name: ShellSetupItemName::WokrcFile,
            target: temp_dir.path().join("test1.txt").to_str().unwrap().to_string(),
            content: "content1".to_string(),
            entire_file: true,
            status: ShellSetupItemStatus::Todo,
        });
        let item2: Result<ShellSetupItem, Box<dyn std::error::Error>> =
            Err(Box::from("test error"));

        let result = collect_results(vec![item1, item2]);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].to_string(), "test error");
    }

    #[test]
    fn test_prepare_shell_bash() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();

        // Create the shell rc file that's required by wokrc_configuration_setup_item
        let bashrc = temp_dir.path().join(".bashrc");
        fs::write(&bashrc, "# existing bashrc\n").unwrap();

        let binary_path = "/test/binary".to_string();
        let result = prepare_shell(&SupportedShells::Bash, binary_path, home_dir);
        assert!(result.is_ok());

        let items = result.unwrap();
        assert_eq!(items.len(), 3);

        // Check that we have all three items
        let has_wokrc = items.iter().any(|i| matches!(i.name, ShellSetupItemName::WokrcFile));
        let has_config = items
            .iter()
            .any(|i| matches!(i.name, ShellSetupItemName::WokrcConfiguration));
        let has_autocomplete = items
            .iter()
            .any(|i| matches!(i.name, ShellSetupItemName::AutocompleteFile));

        assert!(has_wokrc);
        assert!(has_config);
        assert!(has_autocomplete);
    }

    #[test]
    fn test_prepare_shell_zsh() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();

        // Create the shell rc file that's required by wokrc_configuration_setup_item
        let zshrc = temp_dir.path().join(".zshrc");
        fs::write(&zshrc, "# existing zshrc\n").unwrap();

        let binary_path = "/test/binary".to_string();
        let result = prepare_shell(&SupportedShells::Zsh, binary_path, home_dir);
        assert!(result.is_ok());

        let items = result.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_wokrc_file_setup_item_detects_unchanged_file() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();
        let binary_path = "/test/binary".to_string();

        // First, generate the expected content
        let expected_content = compile_wokrc_script(
            &SupportedShells::Bash,
            WokrcScriptParams {
                binary_path: binary_path.clone(),
                goto_marker: GOTO_MARKER.to_string(),
                multiple_matches_marker: MULTIPLE_MATCHES_MARKER.to_string(),
            },
        )
        .unwrap();

        // Write the wokrc file with the exact expected content
        let wokrc_path = temp_dir.path().join(".wokrc.bash");
        fs::write(&wokrc_path, &expected_content).unwrap();

        // Now call wokrc_file_setup_item
        let result = wokrc_file_setup_item(&SupportedShells::Bash, binary_path, home_dir);
        assert!(result.is_ok());

        let item = result.unwrap();
        // The file should be detected as Done since the content matches
        assert_eq!(
            item.status,
            ShellSetupItemStatus::Done,
            "Expected status to be Done when file content matches"
        );
    }

    #[test]
    fn test_wokrc_file_setup_item_detects_changed_file() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();
        let binary_path = "/test/binary".to_string();

        // Write the wokrc file with different/old content
        let wokrc_path = temp_dir.path().join(".wokrc.bash");
        fs::write(&wokrc_path, "# old content that doesn't match\n").unwrap();

        // Now call wokrc_file_setup_item
        let result = wokrc_file_setup_item(&SupportedShells::Bash, binary_path, home_dir);
        assert!(result.is_ok());

        let item = result.unwrap();
        // The file should be detected as OutOfDate since the content doesn't match
        assert_eq!(
            item.status,
            ShellSetupItemStatus::OutOfDate,
            "Expected status to be OutOfDate when file content differs"
        );
    }

    #[test]
    fn test_wokrc_file_setup_item_detects_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_str().unwrap();
        let binary_path = "/test/binary".to_string();

        // Don't create the wokrc file - let it be missing

        // Now call wokrc_file_setup_item
        let result = wokrc_file_setup_item(&SupportedShells::Bash, binary_path, home_dir);
        assert!(result.is_ok());

        let item = result.unwrap();
        // The file should be detected as Todo since it doesn't exist
        assert_eq!(
            item.status,
            ShellSetupItemStatus::Todo,
            "Expected status to be Todo when file doesn't exist"
        );
    }
}
