use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{IdentityKind, StaircaseMetadata, Step, VerificationPolicy, VerificationResult};

pub fn write_metadata(repo: &GitRepo, metadata: &StaircaseMetadata) -> Result<String> {
    let mut descriptor = String::new();
    descriptor.push_str("git-staircase-descriptor 1\n");
    let format = repo.get_object_format()?;
    descriptor.push_str(&format!("object-format {}\n", format));
    descriptor.push_str(&format!("lineage {}\n", metadata.id));
    descriptor.push_str("state clean\n");
    descriptor.push_str(&format!("target-ref {}\n", metadata.target));

    let target_oid = repo.resolve_commit(&metadata.target)?;
    descriptor.push_str(&format!("target-oid {}\n", target_oid));

    if let Some(ref policy) = metadata.verification_policy {
        if let Some(ref cmd) = policy.build_command {
            descriptor.push_str(&format!("build-command {}\n", cmd));
        }
        if let Some(ref cmd) = policy.test_command {
            descriptor.push_str(&format!("test-command {}\n", cmd));
        }
        if policy.verify_each_prefix {
            descriptor.push_str("verify-each-prefix true\n");
        }
    }

    for step in &metadata.steps {
        descriptor.push_str("\n");
        descriptor.push_str(&format!("step {}\n", step.name));
        descriptor.push_str(&format!("cut {}\n", step.cut));
        if let Some(ref branch) = step.branch {
            descriptor.push_str(&format!("materializing-ref refs/heads/{}\n", branch));
        }
    }

    let blob_oid = repo.run_with_stdin(&["hash-object", "-w", "--stdin"], &descriptor)?;
    let blob_oid = blob_oid.trim().to_string();

    let public_ref = format!("refs/staircases/{}", metadata.name);
    repo.run(&["update-ref", &public_ref, &blob_oid])?;

    let state_ref = format!("refs/staircase-state/{}/descriptor", metadata.id);
    repo.run(&["update-ref", &state_ref, &blob_oid])?;

    // Also update step refs for reachability
    for step in &metadata.steps {
        let step_ref = format!("refs/staircase-state/{}/steps/{}", metadata.id, step.name);
        repo.run(&["update-ref", &step_ref, &step.cut])?;
    }

    Ok(blob_oid)
}

pub fn read_metadata(repo: &GitRepo, id_or_name: &str) -> Result<StaircaseMetadata> {
    let name_ref = format!("refs/staircases/{}", id_or_name);
    let id_ref = format!("refs/staircase-state/{}/descriptor", id_or_name);

    let (ref_name, is_name) = if repo.resolve_ref_opt(&name_ref)?.is_some() {
        (name_ref, true)
    } else if repo.resolve_ref_opt(&id_ref)?.is_some() {
        (id_ref, false)
    } else {
        return Err(StaircaseError::Other(format!(
            "Staircase not found: {}",
            id_or_name
        )));
    };

    let content = repo.run(&["cat-file", "-p", &ref_name])?;
    let mut meta = parse_descriptor(&content)?;

    if is_name {
        meta.name = id_or_name.to_string();
    } else {
        // If we read from ID, we might need to find the name elsewhere
        // For now, let's see if we can find a ref in refs/staircases/ pointing to this descriptor
        let oid = repo.resolve_ref(&ref_name)?;
        if let Ok(stdout) = repo.run(&["for-each-ref", "--points-at", &oid, "refs/staircases/"]) {
            if let Some(line) = stdout.lines().next() {
                let refname = line.split_whitespace().last().unwrap_or("");
                if let Some(name) = refname.strip_prefix("refs/staircases/") {
                    meta.name = name.to_string();
                }
            }
        }
        if meta.name.is_empty() {
            meta.name = meta.id.clone();
        }
    }
    Ok(meta)
}

fn parse_descriptor(content: &str) -> Result<StaircaseMetadata> {
    let mut id = String::new();
    let mut target = String::new();
    let mut steps = Vec::new();
    let mut current_step: Option<Step> = None;
    let mut verification_policy = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }

        match parts[0] {
            "lineage" => id = parts[1].to_string(),
            "target-ref" => target = parts[1].to_string(),
            "build-command" => {
                let policy = verification_policy.get_or_insert_with(|| VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.build_command = Some(parts[1].to_string());
            }
            "test-command" => {
                let policy = verification_policy.get_or_insert_with(|| VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.test_command = Some(parts[1].to_string());
            }
            "verify-each-prefix" => {
                let policy = verification_policy.get_or_insert_with(|| VerificationPolicy {
                    build_command: None,
                    test_command: None,
                    verify_each_prefix: false,
                });
                policy.verify_each_prefix = parts[1] == "true";
            }
            "step" => {
                if let Some(step) = current_step.take() {
                    steps.push(step);
                }
                current_step = Some(Step {
                    name: parts[1].to_string(),
                    cut: String::new(),
                    branch: None,
                });
            }
            "cut" => {
                if let Some(ref mut step) = current_step {
                    step.cut = parts[1].to_string();
                }
            }
            "materializing-ref" => {
                if let Some(ref mut step) = current_step {
                    let b = parts[1].strip_prefix("refs/heads/").unwrap_or(parts[1]);
                    step.branch = Some(b.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(step) = current_step {
        steps.push(step);
    }

    Ok(StaircaseMetadata {
        id,
        name: String::new(),
        target,
        steps,
        verification_policy,
    })
}

pub fn record_verification(
    repo: &GitRepo,
    key: &str,
    kind: IdentityKind,
    results: &[VerificationResult],
) -> Result<String> {
    let ref_name = match kind {
        IdentityKind::Lineage => format!("refs/staircases/{}/verification", key),
        IdentityKind::Revision => format!("refs/staircases/by-revision/{}/verification", key),
        _ => {
            return Err(StaircaseError::Other(format!(
                "Unsupported identity kind for verification: {:?}",
                kind
            )));
        }
    };

    let commit_msg = format!(
        "Record verification for staircase {} (kind: {:?})",
        key, kind
    );

    commit_json_data(repo, &ref_name, &results, "verification.json", &commit_msg)
}

pub fn list_staircases(repo: &GitRepo) -> Result<Vec<StaircaseMetadata>> {
    let mut staircases = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Check public names
    if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", "refs/staircases/"]) {
        for line in stdout.lines() {
            let refname = line.trim();
            if refname.starts_with("refs/staircases/") {
                let name = refname.strip_prefix("refs/staircases/").unwrap();
                if !name.contains('/') {
                    if let Ok(meta) = read_metadata(repo, name) {
                        seen_ids.insert(meta.id.clone());
                        staircases.push(meta);
                    }
                }
            }
        }
    }

    // Check internal state for any missed ones (unnamed managed staircases)
    if let Ok(stdout) = repo.run(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/staircase-state/",
    ]) {
        for line in stdout.lines() {
            let refname = line.trim();
            if refname.ends_with("/descriptor") {
                let parts: Vec<&str> = refname
                    .strip_prefix("refs/staircase-state/")
                    .unwrap()
                    .split('/')
                    .collect();
                if parts.len() == 2 && parts[1] == "descriptor" {
                    let id = parts[0];
                    if !seen_ids.contains(id) {
                        if let Ok(meta) = read_metadata(repo, id) {
                            seen_ids.insert(meta.id.clone());
                            staircases.push(meta);
                        }
                    }
                }
            }
        }
    }
    Ok(staircases)
}

pub fn delete_staircase_refs(repo: &GitRepo, id: &str, name: &str) -> Result<()> {
    // Delete state refs
    let state_prefix = format!("refs/staircase-state/{}/", id);
    if let Ok(stdout) = repo.run(&["for-each-ref", "--format=%(refname)", &state_prefix]) {
        for line in stdout.lines() {
            let refname = line.trim();
            repo.run(&["update-ref", "-d", refname])?;
        }
    }
    // Delete public ref
    let ref_name = format!("refs/staircases/{}", name);
    repo.run(&["update-ref", "-d", &ref_name])?;
    Ok(())
}

fn commit_json_data<T: serde::Serialize>(
    repo: &GitRepo,
    ref_name: &str,
    data: &T,
    filename: &str,
    commit_msg: &str,
) -> Result<String> {
    let json = serde_json::to_string_pretty(data)?;

    // 1. Hash and write the JSON blob
    let blob_oid = repo.run_with_stdin(&["hash-object", "-w", "--stdin"], &json)?;
    let blob_oid = blob_oid.trim();

    // 2. Create a tree containing the blob
    let tree_input = format!("100644 blob {}\t{}\n", blob_oid, filename);
    let tree_oid = repo.run_with_stdin(&["mktree"], &tree_input)?;
    let tree_oid = tree_oid.trim();

    // 3. Create a commit
    let mut commit_args = vec!["commit-tree", tree_oid, "-m", commit_msg];

    // Check if ref already exists to use as parent
    let parent_oid = repo.resolve_commit_opt(ref_name).unwrap_or(None);
    if let Some(ref parent) = parent_oid {
        commit_args.push("-p");
        commit_args.push(parent);
    }

    let commit_oid = repo.run(&commit_args)?;
    let commit_oid = commit_oid.trim();

    // 4. Update the ref
    repo.run(&["update-ref", ref_name, commit_oid])?;

    Ok(commit_oid.to_string())
}
