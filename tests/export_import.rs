use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;

/// Helper function to create a bare git repository that can be cloned
fn create_bare_repo(path: &std::path::Path) -> String {
    fs::create_dir_all(path).unwrap();

    // Initialize bare repository
    Command::new("git")
        .arg("init")
        .arg("--bare")
        .current_dir(path)
        .output()
        .expect("Failed to init bare repo");

    path.to_string_lossy().to_string()
}

/// Helper function to initialize a git repository with a local remote
fn init_git_repo_with_local_remote(path: &std::path::Path, remote_path: &str) {
    fs::create_dir_all(path).unwrap();

    Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("Failed to init git repo");

    // Create an initial commit
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("Failed to set git email");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("Failed to set git name");

    fs::write(path.join("README.md"), "# Test Repository\n").expect("Failed to write README");

    Command::new("git")
        .args(&["add", "."])
        .current_dir(path)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .expect("Failed to git commit");

    // Add remote pointing to local bare repo
    Command::new("git")
        .args(&["remote", "add", "origin", remote_path])
        .current_dir(path)
        .output()
        .expect("Failed to add remote");

    // Push to bare repo
    Command::new("git")
        .args(&["push", "-u", "origin", "master"])
        .current_dir(path)
        .output()
        .expect("Failed to push to bare repo");
}

/// Helper function to create a stub workspace with test repositories
fn create_stub_workspace(temp_dir: &std::path::Path, workspace_path: &std::path::Path) {
    // Create bare repositories that can be cloned
    let bare_repos = temp_dir.join("bare_repos");
    fs::create_dir_all(&bare_repos).unwrap();

    let bare1 = create_bare_repo(&bare_repos.join("project1.git"));
    let bare2 = create_bare_repo(&bare_repos.join("project2.git"));
    let bare3 = create_bare_repo(&bare_repos.join("project3.git"));

    // Create org1 with two projects
    let org1_path = workspace_path.join("org1");
    fs::create_dir_all(&org1_path).unwrap();

    let project1_path = org1_path.join("project1");
    init_git_repo_with_local_remote(&project1_path, &bare1);

    let project2_path = org1_path.join("project2");
    init_git_repo_with_local_remote(&project2_path, &bare2);

    // Create org2 with one project
    let org2_path = workspace_path.join("org2");
    fs::create_dir_all(&org2_path).unwrap();

    let project3_path = org2_path.join("project3");
    init_git_repo_with_local_remote(&project3_path, &bare3);

    // Create a project without a remote (should be skipped on import)
    let project4_path = org2_path.join("project-no-remote");
    fs::create_dir_all(&project4_path).unwrap();
    Command::new("git")
        .arg("init")
        .current_dir(&project4_path)
        .output()
        .expect("Failed to init git repo");
}

#[test]
fn test_export_to_file() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let workspace = temp_dir.child("workspace");
    workspace.create_dir_all().unwrap();

    // Create stub repositories
    create_stub_workspace(temp_dir.path(), workspace.path());

    let export_file = temp_dir.child("export.json");

    // Run export command
    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.env("WOK_SPACE", workspace.path())
        .arg("--export")
        .assert()
        .success();

    // Capture output and write to file
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let output = cmd
        .env("WOK_SPACE", workspace.path())
        .arg("--export")
        .output()
        .unwrap();

    fs::write(export_file.path(), &output.stdout).unwrap();

    // Verify export file exists and has valid JSON
    assert!(export_file.exists());

    let content = fs::read_to_string(export_file.path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify JSON structure
    assert!(json.get("timestamp").is_some());
    assert!(json.get("items").is_some());

    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 4); // 3 repos with remotes + 1 without

    // Verify specific projects exist in export
    let project_names: Vec<&str> = items
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();

    assert!(project_names.contains(&"project1"));
    assert!(project_names.contains(&"project2"));
    assert!(project_names.contains(&"project3"));
    assert!(project_names.contains(&"project-no-remote"));
}

#[test]
fn test_import_from_file() {
    let temp_dir = assert_fs::TempDir::new().unwrap();

    // Create source workspace with stub repos
    let source_workspace = temp_dir.child("source");
    source_workspace.create_dir_all().unwrap();
    create_stub_workspace(temp_dir.path(), source_workspace.path());

    // Export from source workspace
    let export_file = temp_dir.child("export.json");
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let output = cmd
        .env("WOK_SPACE", source_workspace.path())
        .arg("--export")
        .output()
        .unwrap();
    fs::write(export_file.path(), &output.stdout).unwrap();

    // Create target workspace (empty)
    let target_workspace = temp_dir.child("target");
    target_workspace.create_dir_all().unwrap();

    // Import to target workspace
    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.env("WOK_SPACE", target_workspace.path())
        .arg("--import")
        .arg(export_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Importing"))
        .stdout(predicate::str::contains("Import complete"));

    // Verify projects were imported
    assert!(target_workspace.child("org1/project1").exists());
    assert!(target_workspace.child("org1/project2").exists());
    assert!(target_workspace.child("org2/project3").exists());

    // Verify git remotes are set correctly (should point to the local bare repos)
    let project1_remote = Command::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(target_workspace.child("org1/project1").path())
        .output()
        .unwrap();
    let remote_url = String::from_utf8_lossy(&project1_remote.stdout).trim().to_string();
    assert!(remote_url.contains("bare_repos/project1.git"));
}

#[test]
fn test_import_from_stdin() {
    let temp_dir = assert_fs::TempDir::new().unwrap();

    // Create source workspace with stub repos
    let source_workspace = temp_dir.child("source");
    source_workspace.create_dir_all().unwrap();
    create_stub_workspace(temp_dir.path(), source_workspace.path());

    // Export from source workspace
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let export_output = cmd
        .env("WOK_SPACE", source_workspace.path())
        .arg("--export")
        .output()
        .unwrap();

    // Create target workspace (empty)
    let target_workspace = temp_dir.child("target");
    target_workspace.create_dir_all().unwrap();

    // Import to target workspace via stdin using piped_stdin trait
    use std::process::Stdio;
    use std::io::Write;

    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.env("WOK_SPACE", target_workspace.path())
        .arg("--import")
        .stdin(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&export_output.stdout).unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    // Verify projects were imported
    assert!(target_workspace.child("org1/project1").exists());
    assert!(target_workspace.child("org1/project2").exists());
    assert!(target_workspace.child("org2/project3").exists());
}

#[test]
fn test_import_skips_existing_projects() {
    let temp_dir = assert_fs::TempDir::new().unwrap();

    // Create source workspace
    let source_workspace = temp_dir.child("source");
    source_workspace.create_dir_all().unwrap();
    create_stub_workspace(temp_dir.path(), source_workspace.path());

    // Export from source
    let export_file = temp_dir.child("export.json");
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let output = cmd
        .env("WOK_SPACE", source_workspace.path())
        .arg("--export")
        .output()
        .unwrap();
    fs::write(export_file.path(), &output.stdout).unwrap();

    // Create target workspace with one existing project
    let target_workspace = temp_dir.child("target");
    target_workspace.create_dir_all().unwrap();

    let existing_project = target_workspace.child("org1/project1");
    existing_project.create_dir_all().unwrap();
    fs::write(existing_project.child("existing.txt"), "Already exists").unwrap();

    // Import to target workspace
    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.env("WOK_SPACE", target_workspace.path())
        .arg("--import")
        .arg(export_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Skipping"))
        .stdout(predicate::str::contains("already exists"));

    // Verify existing project was not overwritten
    assert!(existing_project.child("existing.txt").exists());

    // Verify other projects were imported
    assert!(target_workspace.child("org1/project2").exists());
    assert!(target_workspace.child("org2/project3").exists());
}

#[test]
fn test_export_import_roundtrip() {
    let temp_dir = assert_fs::TempDir::new().unwrap();

    // Create original workspace
    let original_workspace = temp_dir.child("original");
    original_workspace.create_dir_all().unwrap();
    create_stub_workspace(temp_dir.path(), original_workspace.path());

    // Export
    let export_file = temp_dir.child("export.json");
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let output = cmd
        .env("WOK_SPACE", original_workspace.path())
        .arg("--export")
        .output()
        .unwrap();
    fs::write(export_file.path(), &output.stdout).unwrap();

    // Import to new workspace
    let restored_workspace = temp_dir.child("restored");
    restored_workspace.create_dir_all().unwrap();

    let mut cmd = Command::cargo_bin("wok").unwrap();
    cmd.env("WOK_SPACE", restored_workspace.path())
        .arg("--import")
        .arg(export_file.path())
        .assert()
        .success();

    // Export again from restored workspace
    let export_file2 = temp_dir.child("export2.json");
    let mut cmd = Command::cargo_bin("wok").unwrap();
    let output2 = cmd
        .env("WOK_SPACE", restored_workspace.path())
        .arg("--export")
        .output()
        .unwrap();
    fs::write(export_file2.path(), &output2.stdout).unwrap();

    // Compare exports (should be identical except for timestamp and projects without remotes)
    let export1: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(export_file.path()).unwrap()).unwrap();
    let export2: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(export_file2.path()).unwrap()).unwrap();

    // Filter out projects without remotes from export1
    let items1 = export1["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["remote"].as_str().unwrap_or("") != ""
        })
        .cloned()
        .collect::<Vec<_>>();

    let items2 = export2["items"].as_array().unwrap().clone();

    // Both should have 3 projects (excluding the one without remote)
    assert_eq!(items1.len(), 3);
    assert_eq!(items2.len(), 3);

    // Verify all project names match (order might differ)
    let names1: Vec<&str> = items1.iter().map(|i| i["name"].as_str().unwrap()).collect();
    let names2: Vec<&str> = items2.iter().map(|i| i["name"].as_str().unwrap()).collect();

    assert!(names1.contains(&"project1"));
    assert!(names1.contains(&"project2"));
    assert!(names1.contains(&"project3"));

    assert!(names2.contains(&"project1"));
    assert!(names2.contains(&"project2"));
    assert!(names2.contains(&"project3"));
}
