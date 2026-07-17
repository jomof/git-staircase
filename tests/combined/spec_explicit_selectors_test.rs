
use crate::common::*;
use git_staircase::ResolvedStaircase;
use git_staircase::cli::{BaseStaircaseSelectorArgs, StaircaseSelectorArgs};
use git_staircase::core;

#[test]
fn test_explicit_selectors_resolve_ambiguity() {
    let (_tmp, repo) = setup_repo();
    let dir = &repo.workdir;

    // 1. Create a managed staircase named 'auth'
    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["checkout", "-b", "managed-auth"]);
    let _c1 = commit(dir, "m.txt", "m", "managed commit");

    let discoveries = core::discover(&repo, Some("main"), None, false).unwrap();
    let mut s = match &discoveries[0] {
        git_staircase::Discovery::Linear(s) => s.clone(),
        _ => panic!("Expected linear discovery"),
    };
    s.name = "auth".to_string();
    let managed_s = core::adopt(&repo, &s).unwrap();
    let lineage_id = managed_s.id.clone();
    let revision_oid = repo.resolve_ref("refs/staircases/auth").unwrap();

    // 2. Create an implicit staircase also named 'auth' (using a branch named 'auth')
    run_git(dir, &["checkout", "main"]);
    run_git(dir, &["checkout", "-b", "auth"]);
    let _c2 = commit(dir, "i.txt", "i", "implicit commit");

    // 3. Verify that bare 'auth' is ambiguous
    let args_bare = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: Some("auth".to_string()),
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };

    let result = args_bare.resolve(&repo);
    assert!(result.is_err(), "Bare 'auth' should be ambiguous");
    assert!(result.unwrap_err().to_string().contains("ambiguous"));

    // 4. Test --name auth
    let args_name = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: Some("auth".to_string()),
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };
    let rs = args_name.resolve(&repo).expect("Should resolve by --name");
    assert!(matches!(rs.staircase, ResolvedStaircase::Managed(_)));
    assert_eq!(rs.metadata().name, "auth");

    // 5. Test --id <uuid>
    let args_id = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: Some("main".to_string()),
            id: Some(lineage_id.clone()),
            record: None,
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };
    let rs = args_id.resolve(&repo).expect("Should resolve by --id");
    assert!(matches!(rs.staircase, ResolvedStaircase::Managed(_)));
    assert_eq!(rs.metadata().id, lineage_id);

    // 6. Test --ref refs/staircases/auth
    let args_ref = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: Some("main".to_string()),
            id: None,
            record: None,
            explicit_name: None,
            r#ref: Some("refs/staircases/auth".to_string()),
            structural_key: None,
        },
        steps: None,
    };
    let rs = args_ref.resolve(&repo).expect("Should resolve by --ref");
    assert!(matches!(rs.staircase, ResolvedStaircase::Managed(_)));
    assert_eq!(rs.metadata().name, "auth");

    // 7. Test --record <oid>
    let args_rev = StaircaseSelectorArgs {
        base: BaseStaircaseSelectorArgs {
            name: None,
            onto: Some("main".to_string()),
            id: None,
            record: Some(revision_oid.clone()),
            explicit_name: None,
            r#ref: None,
            structural_key: None,
        },
        steps: None,
    };
    let rs = args_rev.resolve(&repo).expect("Should resolve by --record");
    assert!(matches!(rs.staircase, ResolvedStaircase::Managed(_)));
    assert_eq!(
        repo.resolve_ref("refs/staircases/auth").unwrap(),
        revision_oid
    );
}
