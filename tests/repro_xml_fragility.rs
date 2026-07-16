#[test]
fn test_fragile_xml_parsing_with_gt_in_attribute() {
    // This test is actually for parse_manifest_xml_file, but it's private.
    // I can test it through probe_repo_workspace if I setup a .repo directory.

    let tmp = tempfile::tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_dir).unwrap();

    let dot_repo = repo_dir.join(".repo");
    std::fs::create_dir_all(&dot_repo).unwrap();

    // Create a manifest with a '>' in an attribute value
    let manifest_content = r#"<manifest>
  <remote name="origin" fetch=".." review="https://example.com/check?q=>" />
  <default revision="main" remote="origin" />
  <project name="test-project" path="test-project" />
</manifest>"#;
    std::fs::write(dot_repo.join("manifest.xml"), manifest_content).unwrap();

    // We also need a .git directory in the project path to make it a candidate
    let project_dir = repo_dir.join("test-project");
    std::fs::create_dir_all(project_dir.join(".git")).unwrap();

    let repo = git_staircase::GitRepo::new(project_dir);

    let candidate = git_staircase::workspace::repo_provider::probe_repo_workspace(&repo).unwrap();

    assert!(
        candidate.is_some(),
        "Should have found a workspace even with '>' in attribute"
    );
    let c = candidate.unwrap();
    assert_eq!(c.provider, "repo");
}
