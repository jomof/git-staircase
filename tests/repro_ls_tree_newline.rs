mod common;
use common::*;

#[test]
fn test_ls_tree_newline_bug() {
    // ARRANGE
    let ctx = TestContext::new();
    let filename = "a\nb";
    
    // Create a file with a newline in the name
    ctx.commit(filename, "content", "commit with newline");
    let head = ctx.repo.resolve_commit("HEAD").unwrap();

    // ACT
    let entries = ctx.repo.ls_tree(&head).unwrap();

    // ASSERT
    let entry = entries.iter().find(|e| e.name.contains("a") && e.name.contains("b"))
        .expect("Should find the entry");
    
    // If the bug exists, the name will be "\"a\\nb\"" (quoted and escaped)
    // instead of "a\nb"
    assert_eq!(entry.name, filename, "Filename was not parsed correctly!");
}
