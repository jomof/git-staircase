use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::{IdentityKind, VerificationResult};

pub fn verify(
    onto: Option<&str>,
    repo: &GitRepo,
    name: &str,
    build_command_override: Option<String>,
    test_command_override: Option<String>,
    aggregate_only: Option<bool>,
    each_prefix: Option<bool>,
) -> Result<Vec<VerificationResult>> {
    let rs = super::discovery::resolve_staircase(repo, name, onto)?
        .ok_or_else(|| StaircaseError::Other(format!("Staircase '{}' not found", name)))?;

    let s = rs.metadata();

    let policy = s.verification_policy.as_ref();

    let build_cmd = build_command_override.or(policy.and_then(|p| p.build_command.clone()));
    let test_cmd = test_command_override.or(policy.and_then(|p| p.test_command.clone()));

    let verify_each = each_prefix.unwrap_or_else(|| {
        if aggregate_only.unwrap_or(false) {
            false
        } else {
            policy.map(|p| p.verify_each_prefix).unwrap_or(false)
        }
    });

    let mut results = Vec::new();

    let mut targets = Vec::new();
    if verify_each {
        for step in &s.steps {
            targets.push((step.name.clone(), step.cut.clone()));
        }
    } else {
        if let Some(last_step) = s.steps.last() {
            targets.push(("Aggregate".to_string(), last_step.cut.clone()));
        }
    }

    if targets.is_empty() {
        return Err(StaircaseError::Other("No steps to verify".to_string()));
    }

    // Save current branch to restore later
    let original_branch = repo
        .run(&["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    for (step_name, cut) in targets {
        // Checkout the cut
        repo.run(&["checkout", &cut])?;

        let mut success = true;
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let Some(ref cmd) = build_cmd {
            let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
            stdout.push_str(&out);
            stderr.push_str(&err);
            if !ok {
                success = false;
            }
        }

        if success {
            if let Some(ref cmd) = test_cmd {
                let (ok, out, err) = run_shell_command(&repo.workdir, cmd)?;
                stdout.push_str(&out);
                stderr.push_str(&err);
                if !ok {
                    success = false;
                }
            }
        }

        results.push(VerificationResult {
            step_name,
            cut,
            success,
            stdout,
            stderr,
        });

        if !success {
            break;
        }
    }

    // Restore original branch
    let _ = repo.run(&["checkout", &original_branch]);

    // Record results
    let (key, kind) = if rs.is_managed() {
        (s.id.clone(), IdentityKind::Lineage)
    } else {
        (
            super::discovery::compute_identity(repo, &rs, IdentityKind::Revision)?,
            IdentityKind::Revision,
        )
    };
    repo.record_verification(&key, kind, &results)?;

    Ok(results)
}

fn run_shell_command(dir: &std::path::Path, command: &str) -> Result<(bool, String, String)> {
    let output = std::process::Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok((output.status.success(), stdout, stderr))
}
