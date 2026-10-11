//! Behavioral tests for the wokrc script
//! (templates/.wokrc.{bash,zsh}).
//!
//! A child process cannot cd its parent shell, so the wokrc script performs
//! every directory change wok makes: goto navigation, the interactive
//! selection menu, and the root↔worktree toggle. These tests cover the
//! toggle plus the goto-marker and stdout-passthrough paths it relies on.
//! They work by compiling the real template (same placeholder substitution
//! `wok --setup` performs), pointing it at a stub binary that emits a goto
//! marker on fd3, and driving it from a real bash/zsh process.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use wok::lib::constants::{GOTO_MARKER, MULTIPLE_MATCHES_MARKER};

/// Compile the wokrc script the way `wok --setup` does: substitute the
/// binary path and marker placeholders.
fn compile_wokrc_script(shell: &str, stub: &Path) -> String {
    let template = match shell {
        "bash" => include_str!("../templates/.wokrc.bash"),
        _ => include_str!("../templates/.wokrc.zsh"),
    };
    template
        .replace("__WOK_BINARY_PATH__", &stub.to_string_lossy())
        .replace("__WOK_GOTO_MARKER__", GOTO_MARKER)
        .replace("__WOK_MULTIPLE_MATCHES_MARKER__", MULTIPLE_MATCHES_MARKER)
}

struct WokrcFixture {
    root: TempDir,
}

impl WokrcFixture {
    fn new(shell: &str) -> Self {
        let root = TempDir::new().unwrap();
        let a = root.child("a");
        let b = root.child("b");
        a.create_dir_all().unwrap();
        b.create_dir_all().unwrap();

        // Stub binary: emits a goto marker for <root>/b on fd3 and a
        // stdout line, like the real wok does on navigation.
        let stub = root.child("stub");
        fs::write(
            stub.path(),
            format!(
                "#!/bin/sh\nprintf '{}{}' >&3\nprintf 'stub output\\n'\n",
                GOTO_MARKER,
                b.path().to_string_lossy()
            ),
        )
        .unwrap();
        let mut perms = fs::metadata(stub.path()).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(stub.path(), perms).unwrap();

        let wokrc = root.child(format!("wokrc.{}", shell));
        fs::write(wokrc.path(), compile_wokrc_script(shell, stub.path())).unwrap();

        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.child(name).path().to_path_buf()
    }

    fn wokrc(&self, shell: &str) -> PathBuf {
        self.path(&format!("wokrc.{}", shell))
    }

    fn a(&self) -> PathBuf {
        self.path("a")
    }

    fn err_file(&self) -> PathBuf {
        self.path("stderr.txt")
    }

    /// Scenario: navigate via the stub, then ping-pong with every toggle
    /// form. POSIX-compatible so bash and zsh both run it.
    fn write_toggle_scenario(&self, shell: &str) -> PathBuf {
        let script = format!(
            r#"source "{wokrc}"
cd "{a}"
wok proj -w work-x
[ "$(basename "$PWD")" = b ] || {{ echo FAIL_nav; exit 1; }}
wok -
[ "$(basename "$PWD")" = a ] || {{ echo FAIL_toggle_bare; exit 1; }}
wok -w -
[ "$(basename "$PWD")" = b ] || {{ echo FAIL_toggle_w; exit 1; }}
wok --worktree -
[ "$(basename "$PWD")" = a ] || {{ echo FAIL_toggle_long; exit 1; }}
echo SCENARIO_OK
"#,
            wokrc = self.wokrc(shell).to_string_lossy(),
            a = self.a().to_string_lossy(),
        );
        let path = self.path("toggle_scenario.sh");
        fs::write(&path, script).unwrap();
        path
    }

    /// Scenario: in a fresh shell (no __wok_last) the toggle must fail with
    /// a clear error and must not record anything.
    fn write_error_scenario(&self, shell: &str) -> PathBuf {
        let script = format!(
            r#"source "{wokrc}"
cd "{a}"
wok - 2> "{err}"
rc=$?
[ $rc -ne 0 ] || {{ echo FAIL_should_error; exit 1; }}
grep -q "no previous wok location" "{err}" || {{ echo FAIL_message; exit 1; }}
[ -z "${{__wok_last:-}}" ] || {{ echo FAIL_recorded; exit 1; }}
echo ERROR_OK
"#,
            wokrc = self.wokrc(shell).to_string_lossy(),
            a = self.a().to_string_lossy(),
            err = self.err_file().to_string_lossy(),
        );
        let path = self.path("error_scenario.sh");
        fs::write(&path, script).unwrap();
        path
    }
}

fn shell_is_available(shell: &str) -> bool {
    Command::new(shell)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_toggle_scenario(shell: &str) {
    if !shell_is_available(shell) {
        eprintln!("skipping: {} is not available", shell);
        return;
    }
    let fixture = WokrcFixture::new(shell);

    // Navigation + all toggle forms
    let out = Command::new(shell)
        .arg(fixture.write_toggle_scenario(shell))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} toggle scenario failed\nstdout: {}\nstderr: {}",
        shell,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SCENARIO_OK"),
        "{} toggle scenario did not complete\nstdout: {}",
        shell,
        stdout
    );
    // The binary's stdout passes through the wokrc script untouched
    assert!(
        stdout.contains("stub output"),
        "{} wokrc script swallowed binary stdout\nstdout: {}",
        shell,
        stdout
    );

    // Toggle with no recorded location errors out
    let out = Command::new(shell)
        .arg(fixture.write_error_scenario(shell))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} error scenario failed\nstdout: {}\nstderr: {}",
        shell,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ERROR_OK"),
        "{} error scenario did not complete\nstdout: {}",
        shell,
        stdout
    );
}

#[test]
fn test_wokrc_toggle_bash() {
    run_toggle_scenario("bash");
}

#[test]
fn test_wokrc_toggle_zsh() {
    run_toggle_scenario("zsh");
}
