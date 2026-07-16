use git_staircase::GitRepo;
use git_staircase::workspace::repo_provider::probe_repo_workspace;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_extend_project_path_bug() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().join("repo_client");
    fs::create_dir_all(&repo_dir).unwrap();

    let dot_repo = repo_dir.join(".repo");
    fs::create_dir_all(&dot_repo).unwrap();

    // Test case: <extend-project> with dest-path that contains "path" as a substring.
    // parse_attr(tag, "path") should be None if path="..." is missing,
    // but it matches "path=\"" inside "dest-path=\"".

    let manifest_bug = r#"
<manifest>
  <remote name="origin" fetch=".." />
  <default revision="main" remote="origin" />
  <project name="my-project" path="p1" />
  <extend-project name="my-project" dest-path="p1-new" revision="changed-rev" />
</manifest>
"#;
    // In this tag:
    // parse_attr(tag, "name") -> "my-project"
    // parse_attr(tag, "path") -> "new" (WRONG! it matches "path=\"new\"" if dest-path="p1-new")
    // Wait, let's look at how parse_attr works again.
    // pattern = format!("{}=\"", "path") -> "path=\""
    // tag = "<extend-project name=\"my-project\" dest-path=\"p1-new\" revision=\"changed-rev\" />"
    // "dest-path=\"" contains "path=\"" at offset 5.
    // tag[5..] is "path=\"p1-new\" ..."
    // parse_attr will return "p1-new".

    fs::write(dot_repo.join("manifest.xml"), manifest_bug).unwrap();

    // The project will be at "p1-new" if the extend-project is applied correctly.
    let project_dir = repo_dir.join("p1-new");
    fs::create_dir_all(&project_dir).unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&project_dir)
        .output()
        .unwrap();

    let repo = GitRepo::new(project_dir);
    let candidate = probe_repo_workspace(&repo)
        .unwrap()
        .expect("Should find workspace");

    // If the bug is present:
    // path_selector = parse_attr(tag, "path") -> "p1-new"
    // project.path is "p1".
    // "p1-new" != "p1", so the extend-project is NOT applied.
    // The revision remains "main".

    let finger = candidate.fingerprint;
    assert_eq!(
        finger.get("revision").unwrap(),
        "changed-rev",
        "Extend project should have changed the revision"
    );
}
