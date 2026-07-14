use git_staircase::model::*;
use git_staircase::presentation::{Presentation, ToPresentation};
use git_staircase::workspace::review_provider::*;
use std::collections::HashMap;

#[test]
fn test_step_presentation() {
    let step = Step {
        id: "step1".to_string(),
        name: "First Step".to_string(),
        cut: "abcdef1234567890".to_string(),
        branch: Some("feature/1".to_string()),
    };
    let presentation = step.to_presentation();

    match presentation {
        Presentation::List(items) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Presentation::Human(inner) => {
                    assert_eq!(
                        inner.as_ref(),
                        &Presentation::Plain("First Step (abcdef1)".to_string())
                    );
                }
                _ => panic!("Expected Human presentation"),
            }
            match &items[1] {
                Presentation::Porcelain(inner) => {
                    assert_eq!(
                        inner.as_ref(),
                        &Presentation::Record(vec![
                            "step1".to_string(),
                            "First Step".to_string(),
                            "abcdef1234567890".to_string(),
                        ])
                    );
                }
                _ => panic!("Expected Porcelain presentation"),
            }
        }
        _ => panic!("Expected List presentation for Step"),
    }
}

#[test]
fn test_staircase_metadata_presentation() {
    let metadata = StaircaseMetadata {
        id: "staircase1".to_string(),
        name: "My Staircase".to_string(),
        target: "main".to_string(),
        steps: vec![Step {
            id: "s1".to_string(),
            name: "S1".to_string(),
            cut: "cut1".to_string(),
            branch: None,
        }],
        verification_policy: Some(VerificationPolicy {
            build_command: Some("make".to_string()),
            test_command: Some("test".to_string()),
            verify_each_prefix: true,
        }),
        landing_policy: None,
        primary_branch_layout: None,
        branch_layout_base: None,
        user_metadata: None,
        lifecycle: None,
    };

    let presentation = metadata.to_presentation();
    if let Presentation::List(items) = presentation {
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected List presentation for StaircaseMetadata");
    }
}

#[test]
fn test_unified_review_status_presentation() {
    let mut details = HashMap::new();
    details.insert("key".to_string(), "value".to_string());
    let status = UnifiedReviewStatus {
        provider_label: "github".to_string(),
        status: "open".to_string(),
        host: "github.com".to_string(),
        project: "owner/repo".to_string(),
        details,
    };
    let presentation = status.to_presentation();
    if let Presentation::List(items) = presentation {
         assert!(items.len() >= 1);
    } else {
        panic!("Expected List presentation for UnifiedReviewStatus");
    }
}
