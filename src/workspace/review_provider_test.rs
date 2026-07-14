#[cfg(test)]
mod tests {
    use crate::workspace::review_provider::{
        prepare_review_state, ReviewAssociation, ReviewOperationPlan, ReviewPlanItem,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct MockAssociation {
        subject_id: String,
        local_oid: String,
        retired: bool,
    }

    impl ReviewAssociation for MockAssociation {
        fn subject_id(&self) -> &str {
            &self.subject_id
        }
        fn is_retired(&self) -> bool {
            self.retired
        }
        fn set_retired(&mut self, retired: bool) {
            self.retired = retired;
        }
        fn update_local_oid(&mut self, oid: String) {
            self.local_oid = oid;
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct MockPlanItem {
        subject_id: String,
        local_oid: String,
    }

    impl ReviewPlanItem for MockPlanItem {
        fn subject_id(&self) -> &str {
            &self.subject_id
        }
        fn local_oid(&self) -> &str {
            &self.local_oid
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct MockPlan {
        items: Vec<MockPlanItem>,
    }

    impl ReviewOperationPlan for MockPlan {
        type Item = MockPlanItem;
        fn items(&self) -> &[Self::Item] {
            &self.items
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct MockState {
        associations: Vec<MockAssociation>,
    }

    #[test]
    fn test_prepare_review_state_create() {
        let plan = MockPlan {
            items: vec![MockPlanItem {
                subject_id: "s1".into(),
                local_oid: "oid1".into(),
            }],
        };

        let state = prepare_review_state(
            &plan,
            None,
            |p| {
                Ok(MockState {
                    associations: p
                        .items()
                        .iter()
                        .map(|i| MockAssociation {
                            subject_id: i.subject_id().into(),
                            local_oid: i.local_oid().into(),
                            retired: false,
                        })
                        .collect(),
                })
            },
            |_, _| Ok(()),
            |s| &mut s.associations,
            |i| {
                Ok(MockAssociation {
                    subject_id: i.subject_id().into(),
                    local_oid: i.local_oid().into(),
                    retired: false,
                })
            },
        )
        .unwrap();

        assert_eq!(state.associations.len(), 1);
        assert_eq!(state.associations[0].subject_id, "s1");
        assert_eq!(state.associations[0].local_oid, "oid1");
    }

    #[test]
    fn test_prepare_review_state_update_and_retire() {
        let existing_state = MockState {
            associations: vec![
                MockAssociation {
                    subject_id: "s1".into(),
                    local_oid: "old_oid1".into(),
                    retired: false,
                },
                MockAssociation {
                    subject_id: "s2".into(),
                    local_oid: "oid2".into(),
                    retired: false,
                },
            ],
        };

        let plan = MockPlan {
            items: vec![
                MockPlanItem {
                    subject_id: "s1".into(),
                    local_oid: "new_oid1".into(),
                },
                MockPlanItem {
                    subject_id: "s3".into(),
                    local_oid: "oid3".into(),
                },
            ],
        };

        let state = prepare_review_state(
            &plan,
            Some(existing_state),
            |_| panic!("should not call create_state"),
            |_, _| Ok(()),
            |s| &mut s.associations,
            |i| {
                Ok(MockAssociation {
                    subject_id: i.subject_id().into(),
                    local_oid: i.local_oid().into(),
                    retired: false,
                })
            },
        )
        .unwrap();

        assert_eq!(state.associations.len(), 3);
        
        let s1 = state.associations.iter().find(|a| a.subject_id == "s1").unwrap();
        assert_eq!(s1.local_oid, "new_oid1");
        assert!(!s1.retired);

        let s2 = state.associations.iter().find(|a| a.subject_id == "s2").unwrap();
        assert!(s2.retired);

        let s3 = state.associations.iter().find(|a| a.subject_id == "s3").unwrap();
        assert_eq!(s3.local_oid, "oid3");
        assert!(!s3.retired);
    }
}
