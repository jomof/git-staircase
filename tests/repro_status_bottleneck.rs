use git_staircase::core::status::*;
use git_staircase::git::GitRepo;
use git_staircase::model::*;
use tempfile::TempDir;

#[test]
fn test_status_bottleneck_modified_steps() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_path_buf();
    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();

    // Create some commits
    let run_git = |args: &[&str]| {
        std::process::Command::new("git")
            .current_dir(&repo_path)
            .args(args)
            .output()
            .unwrap()
    };
    run_git(&["commit", "--allow-empty", "-m", "initial"]);
    let base = String::from_utf8(run_git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_string();
    run_git(&["commit", "--allow-empty", "-m", "step1"]);
    let step1_cut = String::from_utf8(run_git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_string();
    run_git(&["commit", "--allow-empty", "-m", "step1_mod"]);
    let step1_actual = String::from_utf8(run_git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_string();

    // Create a branch for step1
    run_git(&["branch", "feat-1", &step1_actual]);

    let repo = GitRepo::new(repo_path);
    let metadata = StaircaseMetadata {
        id: "test".to_string(),
        name: "test".to_string(),
        target: base.clone(),
        steps: vec![Step {
            id: "s1".to_string(),
            name: "s1".to_string(),
            cut: step1_cut.clone(),
            branch: Some("feat-1".to_string()),
        }],
        landing_policy: None,
        verification_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    // Manually call preload_ancestry with what status would use
    repo.preload_ancestry(&[&step1_cut]).unwrap();
    assert!(
        repo.memoizer.get_ancestry(&base, &step1_actual).is_none(),
        "Preload should NOT have populated the memoizer for the modified OID!"
    );

    let _status = get_status_metadata(&repo, metadata, false).unwrap();

    // After status runs, it should be in there (because it ran git merge-base)
    assert!(repo.memoizer.get_ancestry(&base, &step1_actual).is_some());
}
