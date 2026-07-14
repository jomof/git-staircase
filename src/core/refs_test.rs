use crate::core::refs::StaircaseRefs;
use crate::model::LifecycleState;

#[test]
fn test_public_ref() {
    assert_eq!(
        StaircaseRefs::public("my-staircase"),
        "refs/staircases/my-staircase"
    );
    assert_eq!(
        StaircaseRefs::public("feature/branch"),
        "refs/staircases/feature/branch"
    );
}

#[test]
fn test_state_refs() {
    let id = "uuid-123";
    assert_eq!(
        StaircaseRefs::state_record(id),
        "refs/staircase-state/uuid-123/record"
    );
    assert_eq!(
        StaircaseRefs::state_descriptor(id),
        "refs/staircase-state/uuid-123/descriptor"
    );
    assert_eq!(
        StaircaseRefs::state_step(id, "step-1"),
        "refs/staircase-state/uuid-123/steps/step-1"
    );
}

#[test]
fn test_archive_refs() {
    let id = "uuid-456";
    assert_eq!(
        StaircaseRefs::archive_record(id),
        "refs/staircase-archive/uuid-456/record"
    );
    assert_eq!(
        StaircaseRefs::archive_step(id, "step-2"),
        "refs/staircase-archive/uuid-456/steps/step-2"
    );
    assert_eq!(
        StaircaseRefs::archive_owned(id, "ref-789"),
        "refs/staircase-archive/uuid-456/owned/ref-789"
    );
}

#[test]
fn test_verification_refs() {
    assert_eq!(
        StaircaseRefs::verification("id1"),
        "refs/staircases/id1/verification"
    );
    assert_eq!(
        StaircaseRefs::revision_verification("abc1234"),
        "refs/staircases/by-revision/abc1234/verification"
    );
}

#[test]
fn test_refs_manager_active() {
    let id = "uuid-active";
    let name = "my-staircase";
    let step_id = "step-1";

    assert_eq!(
        StaircaseRefs::record(id, LifecycleState::Active),
        "refs/staircase-state/uuid-active/record"
    );
    assert_eq!(
        StaircaseRefs::step(id, step_id, LifecycleState::Active),
        "refs/staircase-state/uuid-active/steps/step-1"
    );
    assert_eq!(
        StaircaseRefs::public_optional(Some(name), LifecycleState::Active),
        Some("refs/staircases/my-staircase".to_string())
    );
}

#[test]
fn test_refs_manager_archived() {
    let id = "uuid-archived";
    let name = "old-staircase";
    let step_id = "step-2";

    assert_eq!(
        StaircaseRefs::record(id, LifecycleState::Archived),
        "refs/staircase-archive/uuid-archived/record"
    );
    assert_eq!(
        StaircaseRefs::step(id, step_id, LifecycleState::Archived),
        "refs/staircase-archive/uuid-archived/steps/step-2"
    );
    // Public ref should NOT exist for archived staircases unless specifically managed otherwise,
    // but the current logic seems to delete it.
    assert_eq!(
        StaircaseRefs::public_optional(Some(name), LifecycleState::Archived),
        None
    );
}
