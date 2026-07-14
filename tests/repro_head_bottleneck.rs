use git_staircase::GitRepo;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_head_resolution_bottleneck() {
    let dir = tempdir().unwrap();
    let repo_dir = dir.path();
    std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Create a dummy commit so HEAD is resolvable
    std::fs::write(repo_dir.join("file"), "content").unwrap();
    std::process::Command::new("git")
        .arg("add")
        .arg("file")
        .current_dir(repo_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(repo_dir)
        .output()
        .unwrap();

    let repo = GitRepo::new(repo_dir.to_path_buf());

    // Measure time for 100 resolutions of HEAD
    let start = Instant::now();
    for _ in 0..100 {
        repo.resolve_commit("HEAD").unwrap();
    }
    let duration = start.elapsed();

    // On modern systems, 300 git processes should take significantly longer than memoized access.
    // If it's memoized, it should take microseconds. If not, it takes seconds.
    // We set a threshold of 500ms for 100 calls, which is generous for non-memoized but
    // should be easily beaten by memoization.
    assert!(
        duration.as_millis() < 500,
        "HEAD resolution is too slow ({}ms), likely not memoized",
        duration.as_millis()
    );
}
