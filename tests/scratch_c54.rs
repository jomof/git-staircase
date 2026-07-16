use std::process::Command;
mod common;
use common::*;

#[test]
fn integration_branch_at_anchor_is_not_discovered() {
    let (tmp, _repo) = setup_repo();
    // main is at its anchor (initially it has no remote, but let's assume 'main' is onto).
    // if onto is main, and we have branch main.

    let (list_ok, stdout, _) =
        run_staircase_in(tmp.path(), &["list", "--onto", "main", "--porcelain"]);
    assert!(list_ok);
    assert!(
        stdout.is_empty(),
        "Should not discover anything when main is at main. Output: {}",
        stdout
    );
}

fn run_staircase_in(path: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let ws_dir = std::env::temp_dir().join(format!(".ws_storage_{:p}", path));
    let binary = get_test_binary_path();
    let out = Command::new(&binary)
        .current_dir(path)
        .env("GIT_STAIRCASE_WORKSPACE_DIR", &ws_dir)
        .args(args)
        .output()
        .expect("failed to run binary");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}
