use std::fs;
use std::env;
use tempfile::tempdir;

#[test]
fn test_description_with_markdown_header() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();
    
    // Initialize a git repo
    std::process::Command::new("git")
        .args(&["init", "-b", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Create a dummy commit to have a staircase
    fs::write(repo_dir.join("a.txt"), "a").unwrap();
    std::process::Command::new("git").args(&["add", "."]).current_dir(repo_dir).output().unwrap();
    std::process::Command::new("git").args(&["commit", "-m", "initial"]).current_dir(repo_dir).output().unwrap();
    fs::write(repo_dir.join("b.txt"), "b").unwrap();
    std::process::Command::new("git").args(&["add", "."]).current_dir(repo_dir).output().unwrap();
    std::process::Command::new("git").args(&["commit", "-m", "second"]).current_dir(repo_dir).output().unwrap();
    
    // Create a managed staircase
    let adopt_output = std::process::Command::new("cargo")
        .args(&["run", "--", "adopt", "my-staircase", "main", "--onto", "HEAD^"])
        .current_dir(env::current_dir().unwrap())
        .env("GIT_DIR", repo_dir.join(".git"))
        .env("GIT_WORK_TREE", repo_dir)
        .output()
        .unwrap();
    assert!(adopt_output.status.success(), "Adopt failed: {}", String::from_utf8_lossy(&adopt_output.stderr));

    // Create a mock editor script
    let editor_script = repo_dir.join("mock_editor.sh");
    fs::write(&editor_script, r#"#!/bin/sh
cat <<INNEREOF > "$1"
# Title: My Staircase
# This line should be ignored as a comment
# Header 1
This is a description.
## Header 2
More description.
INNEREOF
"#).unwrap();
    
    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&editor_script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Run describe --edit
    let edit_output = std::process::Command::new("cargo")
        .args(&["run", "--", "describe", "--edit", "my-staircase"])
        .current_dir(env::current_dir().unwrap())
        .env("GIT_DIR", repo_dir.join(".git"))
        .env("GIT_WORK_TREE", repo_dir)
        .env("EDITOR", &editor_script)
        .output()
        .unwrap();
    assert!(edit_output.status.success(), "Describe edit failed: {}", String::from_utf8_lossy(&edit_output.stderr));

    // Now check the description
    let describe_output = std::process::Command::new("cargo")
        .args(&["run", "--", "describe", "my-staircase", "--json"])
        .current_dir(env::current_dir().unwrap())
        .env("GIT_DIR", repo_dir.join(".git"))
        .env("GIT_WORK_TREE", repo_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&describe_output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect(&format!("Failed to parse JSON: {}", stdout));
    let description = json["description"].as_str().unwrap_or("");
    
    println!("Description: {}", description);
    
    // Check if Headers are present
    assert!(description.contains("# Header 1"), "Description missing '# Header 1'. Found: '{}'", description);
    assert!(description.contains("## Header 2"), "Description missing '## Header 2'. Found: '{}'", description);
}
