use git_staircase::GitRepo;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_repro_manifest_xml_parsing_bracket() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().join("repo_client");
    fs::create_dir_all(&repo_dir.join(".repo")).unwrap();

    // Create a manifest where a project's name contains a '>' character.
    let manifest_content = r#"<manifest>
  <remote name="origin" fetch=".." />
  <default revision="main" remote="origin" />
  <project name="foo > bar" path="my-project" />
</manifest>"#;
    fs::write(repo_dir.join(".repo/manifest.xml"), manifest_content).unwrap();

    let project_dir = repo_dir.join("my-project");
    fs::create_dir_all(&project_dir).unwrap();
    std::process::Command::new("git")
        .current_dir(&project_dir)
        .args(&["init", "-b", "main"])
        .output()
        .unwrap();

    let repo = GitRepo::new(project_dir);
    let candidate = git_staircase::workspace::repo_provider::probe_repo_workspace(&repo)
        .expect("probe should succeed")
        .expect("should find a repo candidate");

    assert_eq!(
        candidate.current_project.as_ref().unwrap().identity,
        "foo > bar",
        "Project was not found or misparsed because of '>' in manifest attribute!"
    );
}
