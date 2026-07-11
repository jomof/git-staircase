use crate::error::Result;
use crate::git::GitRepo;
use crate::model::{IdentityKind};

use super::ResolvedStaircase;
pub fn compute_identity(
    repo: &GitRepo,
    staircase: &ResolvedStaircase,
    kind: IdentityKind,
) -> Result<String> {
    if kind == IdentityKind::Lineage && !staircase.is_managed() {
        super::resolved::adopt(repo, staircase.metadata())?;
    }
    let staircase = staircase.metadata();
    match kind {
        IdentityKind::Lineage => Ok(staircase.id.clone()),
        IdentityKind::Nominal => Ok(staircase.name.clone()),
        IdentityKind::Revision => {
            let format = repo.get_object_format()?;
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut data = format!("format:{}\ntarget:{}\n", format, target_oid);
            for (i, step) in staircase.steps.iter().enumerate() {
                data.push_str(&format!("step{}:{}\n", i, step.cut));
            }
            repo.hash_data(&data)
        }
        IdentityKind::Body => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let top_oid = staircase
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let data = format!("target:{}\ntop:{}\n", target_oid, top_oid);
            repo.hash_data(&data)
        }
        IdentityKind::Decomposition => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut patches = Vec::new();
            let mut last_cut = target_oid;
            for step in &staircase.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patches.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patches.join("\n---\n"))
        }
        IdentityKind::Outcome => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let target_tree = repo.get_tree_id(&target_oid)?;
            let top_oid = staircase
                .steps
                .last()
                .map(|s| s.cut.as_str())
                .unwrap_or(&target_oid);
            let top_tree = repo.get_tree_id(top_oid)?;
            let data = format!("base-tree:{}\ntop-tree:{}\n", target_tree, top_tree);
            repo.hash_data(&data)
        }
        IdentityKind::PatchSeries => {
            let target_oid = repo.resolve_ref(&staircase.target)?;
            let mut patch_ids = Vec::new();
            let mut last_cut = target_oid;
            for step in &staircase.steps {
                let patch_id = repo.get_patch_id(&last_cut, &step.cut)?;
                patch_ids.push(patch_id);
                last_cut = step.cut.clone();
            }
            repo.hash_data(&patch_ids.join("\n"))
        }
        IdentityKind::Review => Ok("".to_string()),
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::git::GitRepo;
    use crate::model::{IdentityKind, StaircaseMetadata, Step};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed. Stderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
        let path = dir.join(file);
        fs::write(path, contents).unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "--allow-empty", "-m", msg]);
        run_git(dir, &["rev-parse", "HEAD"])
    }

    fn setup_repo() -> (TempDir, GitRepo, String) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        run_git(&path, &["init", "-b", "main"]);
        let target = commit(&path, "init.txt", "initial", "initial commit");
        (tmp, GitRepo::new(path), target)
    }

    #[test]
    fn test_identity_lineage_and_nominal() {
        let (_tmp, repo, target) = setup_repo();
        let staircase = StaircaseMetadata {
            id: "test-uuid".to_string(),
            name: "test-name".to_string(),
            target: target,
            steps: vec![],
            verification_policy: None,
        };

        assert_eq!(
            compute_identity(
                &repo,
                &ResolvedStaircase::Managed(staircase.clone()),
                IdentityKind::Lineage
            )
            .unwrap(),
            "test-uuid"
        );
        assert_eq!(
            compute_identity(
                &repo,
                &ResolvedStaircase::Managed(staircase.clone()),
                IdentityKind::Nominal
            )
            .unwrap(),
            "test-name"
        );
    }

    #[test]
    fn test_identity_revision() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![Step {
                name: "s1".to_string(),
                cut: c1.clone(),
                branch: None,
            }],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Revision,
        )
        .unwrap();

        // Change step cut, should change revision ID
        let c2 = commit(dir, "f2.txt", "2", "c2");
        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "s1".to_string(),
                cut: c2.clone(),
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Revision,
        )
        .unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_identity_body() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Body,
        )
        .unwrap();

        // Join steps, body ID should stay the same
        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "s1+2".to_string(),
                cut: c2.clone(),
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Body,
        )
        .unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_identity_decomposition() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();

        // Rebase by committing same content with different messages on same target
        run_git(dir, &["checkout", &target]);
        let c1_new = commit(dir, "f1.txt", "1", "c1 rebased");
        let c2_new = commit(dir, "f2.txt", "2", "c2 rebased");

        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1_new.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2_new.clone(),
                    branch: None,
                },
            ],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();
        assert_eq!(id1, id2);

        // Squash steps, decomposition ID should change
        let s3 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![Step {
                name: "s1+2".to_string(),
                cut: c2_new.clone(),
                branch: None,
            }],
        };
        let id3 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s3.clone()),
            IdentityKind::Decomposition,
        )
        .unwrap();
        assert_ne!(id2, id3);
    }

    #[test]
    fn test_identity_outcome() {
        let (_tmp, repo, target) = setup_repo();
        let dir = &repo.workdir;
        let c1 = commit(dir, "f1.txt", "1", "c1");
        let c2 = commit(dir, "f2.txt", "2", "c2");

        let s1 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target.clone(),
            verification_policy: None,
            steps: vec![
                Step {
                    name: "s1".to_string(),
                    cut: c1.clone(),
                    branch: None,
                },
                Step {
                    name: "s2".to_string(),
                    cut: c2.clone(),
                    branch: None,
                },
            ],
        };

        let id1 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s1.clone()),
            IdentityKind::Outcome,
        )
        .unwrap();

        // Reorder commits to produce same final tree, outcome ID should stay the same
        run_git(dir, &["checkout", "main"]);
        // Start from initial commit again
        run_git(dir, &["checkout", &target]);
        commit(dir, "f2.txt", "2", "c2 reordered");
        commit(dir, "f1.txt", "1", "c1 reordered");
        let top_new = run_git(dir, &["rev-parse", "HEAD"]);

        let s2 = StaircaseMetadata {
            id: "uuid".to_string(),
            name: "name".to_string(),
            target: target,
            verification_policy: None,
            steps: vec![Step {
                name: "reordered".to_string(),
                cut: top_new,
                branch: None,
            }],
        };
        let id2 = compute_identity(
            &repo,
            &ResolvedStaircase::Managed(s2.clone()),
            IdentityKind::Outcome,
        )
        .unwrap();
        assert_eq!(id1, id2);
    }
}
