use crate::core::refs::StaircaseRefs;

#[test]
fn test_public_ref() {
    assert_eq!(StaircaseRefs::public("my-staircase"), "refs/staircases/my-staircase");
    assert_eq!(StaircaseRefs::public("feature/branch"), "refs/staircases/feature/branch");
}

#[test]
fn test_state_refs() {
    let id = "uuid-123";
    assert_eq!(StaircaseRefs::state_record(id), "refs/staircase-state/uuid-123/record");
    assert_eq!(StaircaseRefs::state_descriptor(id), "refs/staircase-state/uuid-123/descriptor");
    assert_eq!(StaircaseRefs::state_step(id, "step-1"), "refs/staircase-state/uuid-123/steps/step-1");
}

#[test]
fn test_archive_refs() {
    let id = "uuid-456";
    assert_eq!(StaircaseRefs::archive_record(id), "refs/staircase-archive/uuid-456/record");
    assert_eq!(StaircaseRefs::archive_step(id, "step-2"), "refs/staircase-archive/uuid-456/steps/step-2");
    assert_eq!(StaircaseRefs::archive_owned(id, "ref-789"), "refs/staircase-archive/uuid-456/owned/ref-789");
}

#[test]
fn test_verification_refs() {
    assert_eq!(StaircaseRefs::verification("id1"), "refs/staircases/id1/verification");
    assert_eq!(StaircaseRefs::revision_verification("abc1234"), "refs/staircases/by-revision/abc1234/verification");
}
