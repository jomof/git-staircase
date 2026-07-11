use git_staircase::core::discovery::compute_implicit_id;
use git_staircase::model::Step;

#[test]
fn test_id_stability_and_format() {
    let steps = vec![Step {
        name: "step1".to_string(),
        cut: "1111111111111111111111111111111111111111".to_string(),
        branch: Some("step1".to_string()),
    }];
    let target_oid = "0000000000000000000000000000000000000000";
    let object_format = "sha1";

    // This will fail to compile initially because compute_implicit_id only takes steps
    let id = compute_implicit_id(object_format, target_oid, &steps);

    assert!(
        id.starts_with("implicit@"),
        "ID should start with implicit@, got {}",
        id
    );

    // We expect a stable hash. For version 1, target_oid, 1 step, step1 cut and name.
    // Let's say we expect a specific value once we implement it.
    // For now, let's just assert it's 16 hex chars after implicit@
    let hash_part = &id["implicit@".len()..];
    assert_eq!(
        hash_part.len(),
        16,
        "Hash part should be 16 hex chars, got {}",
        hash_part
    );
    assert!(
        hash_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash part should be hex, got {}",
        hash_part
    );
}

#[test]
fn test_id_is_stable_across_calls() {
    let steps = vec![Step {
        name: "step1".to_string(),
        cut: "1111111111111111111111111111111111111111".to_string(),
        branch: Some("step1".to_string()),
    }];
    let target_oid = "0000000000000000000000000000000000000000";
    let object_format = "sha1";

    let id1 = compute_implicit_id(object_format, target_oid, &steps);
    let id2 = compute_implicit_id(object_format, target_oid, &steps);

    assert_eq!(id1, id2, "IDs should be identical for same input");
}
