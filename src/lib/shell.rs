use std::process::Command;

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
