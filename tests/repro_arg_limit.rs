#![allow(unused)]
use git_staircase::GitRepo;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_preload_ancestry_arg_limit() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    let run_git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    };

    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);

    fs::write(repo_path.join("file"), "content").unwrap();
    run_git(&["add", "file"]);
    run_git(&["commit", "-m", "initial"]);

    let repo = GitRepo::new(repo_path.to_path_buf());

    // 100,000 unique strings of 40 chars each. ~4MB, exceeding typical 2MB ARG_MAX.
    let many_oids: Vec<String> = (0..100000).map(|i| format!("{:040}", i)).collect();
    let many_oids_refs: Vec<&str> = many_oids.iter().map(|s| s.as_str()).collect();

    let res = repo.preload_ancestry_ext(&many_oids_refs, &[]);
    match res {
        Err(e) => {
            let err_str = format!("{:?}", e);
            assert!(err_str.contains("Argument list too long") || err_str.contains("os error 7"));
        }
        Ok(_) => panic!("Expected error but got success"),
    }
}
