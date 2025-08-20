use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use wok::lib::constants::GOTO_MARKER;

enum SupportedShells {
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

struct WokrcScriptParams {
    binary_path: String,
    goto_marker: String,
}

pub fn handle(shell: &Option<String>, manual: bool) -> Result<(), Box<dyn std::error::Error>> {
    let shell_type = match shell {
        Some(s) => validate_shell(s)?,
        None => detect_shell()?,
    };

    let binary_path = env::current_exe()?.to_string_lossy().to_string();
    let goto_marker = GOTO_MARKER.to_string();
    let params = WokrcScriptParams {
        binary_path,
        goto_marker,
    };

    let wokrc_script = compile_wokrc_script(&shell_type, params)?;

    if manual {
        print_instructions(&shell_type, &wokrc_script)?;
    } else {
        install_wokrc_script(&wokrc_script, &shell_type)?;
    }

    Ok(())
}

fn detect_shell() -> Result<SupportedShells, Box<dyn std::error::Error>> {
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
    let raw = include_str!("../../templates/manual_setup.txt");
    let instructions = compile_template(
        raw,
        &[
            ("WOKRC_PATH".to_string(), wokrc_path.clone()),
            ("WOKRC_NAME".to_string(), ".wokrc".to_string()),
            ("WOKRC_SCRIPT".to_string(), wokrc_script.to_string()),
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
