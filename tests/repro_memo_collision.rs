mod common;
use common::TestContext;

#[test]
fn test_memoization_collision_same_length() {
    let ctx = TestContext::new();
    let repo = &ctx.repo;

    // Content A
    let content_a = "content_a";
    // Content B (different content, same length as A)
    let content_b = "content_b";

    // Hash content A
    let hash_a = repo.hash_data(content_a).expect("Failed to hash A");

    // Hash content B
    let hash_b = repo.hash_data(content_b).expect("Failed to hash B");

    // ASSERT that hash_a and hash_b are different
    assert_ne!(
        hash_a, hash_b,
        "Hash of '{}' and '{}' should be different",
        content_a, content_b
    );
}

#[test]
fn test_memoization_collision_different_length() {
    let ctx = TestContext::new();
    let repo = &ctx.repo;

    // Content A
    let content_a = "short";
    // Content B
    let content_b = "very_long_content";

    // Hash content A
    let hash_a = repo.hash_data(content_a).expect("Failed to hash A");

    // Hash content B
    let hash_b = repo.hash_data(content_b).expect("Failed to hash B");

    // ASSERT that hash_a and hash_b are different
    assert_ne!(
        hash_a, hash_b,
        "Hash of '{}' and '{}' should be different",
        content_a, content_b
    );
}
