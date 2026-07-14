use git_staircase::GitRepo;
use git_staircase::core::operation::recovery::capture_draft;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

#[test]
fn test_repro_broken_symlink_data_loss() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    
    // Initialize repo
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["init", "-b", "main"])
        .output()
        .unwrap();
        
    // Create initial commit
    fs::write(dir.join("root.txt"), "root").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "root.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "initial"])
        .output()
        .unwrap();
        
    let repo = GitRepo::new(dir.to_path_buf());
    
    // To trigger capture_conflicted_state, we need a conflict.
    fs::write(dir.join("conflict.txt"), "a").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "conflict.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "conflict base"])
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["branch", "other"])
        .output()
        .unwrap();
        
    fs::write(dir.join("conflict.txt"), "b").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "conflict.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "b"])
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["checkout", "other"])
        .output()
        .unwrap();
        
    fs::write(dir.join("conflict.txt"), "c").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "conflict.txt"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "c"])
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["merge", "main"])
        .output()
        .unwrap(); 
        
    // Now we have a conflict. Create a broken symlink as a MODIFIED file.
    fs::write(dir.join("link"), "target").unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["add", "link"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "add link"])
        .output()
        .unwrap();
        
    fs::remove_file(dir.join("link")).unwrap();
    symlink("non-existent", dir.join("link")).unwrap();
    
    let draft = capture_draft(&repo).unwrap().unwrap();
    let found = draft.dirty_files.iter().any(|f| f.path == "link");
    
    assert!(found, "Broken symlink 'link' should be captured in dirty_files even if it doesn't exist");
}
