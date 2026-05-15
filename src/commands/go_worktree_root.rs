use std::os::unix::io::AsRawFd;
use wok::lib::constants::GOTO_MARKER;
use wok::lib::fd::write_to_fd;

use crate::commands::go_worktree::resolve_project_path;

pub fn handle(workspace: &str, project: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = resolve_project_path(workspace, project)?;
    let dest = project_path.to_string_lossy().to_string();
    let goto_dest = format!("{}{}", GOTO_MARKER, &dest);
    write_to_fd(3, &goto_dest)?;
    if is_tty() {
        println!("{}", &dest);
    }
    Ok(())
}

fn is_tty() -> bool {
    unsafe { libc::isatty(std::io::stdout().as_raw_fd()) != 0 }
}
