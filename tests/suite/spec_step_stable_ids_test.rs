use crate::common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_step_stable_ids() {
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path();

    // ARRANGE: Create a repository with three branches forming an implicit staircase
    run_git(repo_path, &["init", "-b", "main"]);
    fs::write(repo_path.join("init.txt"), "initial").unwrap();
    run_git(repo_path, &["add", "init.txt"]);
    run_git(repo_path, &["commit", "-m", "initial"]);

    run_git(repo_path, &["checkout", "-b", "feature/step1"]);
    fs::write(repo_path.join("step1.txt"), "step1").unwrap();
    run_git(repo_path, &["add", "step1.txt"]);
    run_git(repo_path, &["commit", "-m", "step1"]);

    run_git(repo_path, &["checkout", "-b", "feature/step2"]);
    fs::write(repo_path.join("step2.txt"), "step2").unwrap();
    run_git(repo_path, &["add", "step2.txt"]);
    run_git(repo_path, &["commit", "-m", "step2"]);

    run_git(repo_path, &["checkout", "-b", "feature/step3"]);
    fs::write(repo_path.join("step3.txt"), "step3").unwrap();
    run_git(repo_path, &["add", "step3.txt"]);
    run_git(repo_path, &["commit", "-m", "step3"]);

    // ACT: Adopt the staircase
    let (success, _, stderr) = run_staircase(
        repo_path,
        &[
            "adopt",
            "feature",
            "feature/step1",
            "feature/step2",
            "feature/step3",
        ],
    );
    assert!(success, "adopt failed: {}", stderr);

    // ACT: Get steps in JSON format to inspect IDs
    let (success, output, stderr) = run_staircase(repo_path, &["steps", "feature", "--json"]);
    assert!(success, "steps --json failed: {}", stderr);

    let steps: serde_json::Value = serde_json::from_str(&output).unwrap();
    let steps_array = steps.as_array().expect("Expected steps array");
    assert_eq!(steps_array.len(), 3);

    // ASSERT: Each step should have an 'id' field that is a valid UUID
    let mut initial_ids = Vec::new();
    for step in steps_array {
        let id = step
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Step missing id field");
        assert!(uuid::Uuid::parse_str(id).is_ok(), "Invalid step ID: {}", id);
        initial_ids.push(id.to_string());
    }

    // ACT: Reorder steps (e.g., 1, 3, 2)
    let (success, _, stderr) =
        run_staircase(repo_path, &["reorder", "feature", "--order", "1,3,2"]);
    assert!(success, "reorder failed: {}", stderr);

    // ACT: Get steps again
    let (success, output, stderr) = run_staircase(repo_path, &["steps", "feature", "--json"]);
    assert!(success, "steps --json failed: {}", stderr);

    let reordered_steps: serde_json::Value = serde_json::from_str(&output).unwrap();
    let reordered_steps_array = reordered_steps.as_array().expect("Expected steps array");
    assert_eq!(reordered_steps_array.len(), 3);

    // ASSERT: Conceptual steps should retain their UUIDs even as their positions change
    // Original: 1, 2, 3
    // New: 1, 3, 2
    let new_ids: Vec<String> = reordered_steps_array
        .iter()
        .map(|s| s.get("id").unwrap().as_str().unwrap().to_string())
        .collect();

    assert_eq!(new_ids[0], initial_ids[0], "Step 1 should keep its ID");
    assert_eq!(
        new_ids[1], initial_ids[2],
        "Step 2 (formerly 3) should keep its ID"
    );
    assert_eq!(
        new_ids[2], initial_ids[1],
        "Step 3 (formerly 2) should keep its ID"
    );
}
