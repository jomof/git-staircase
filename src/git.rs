use crate::error::{Result, StaircaseError};
use crate::model::{BranchInfo, IdentityKind, StaircaseMetadata, VerificationResult};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub workdir: PathBuf,
}

impl GitRepo {
    pub fn new(workdir: PathBuf) -> Self {
        GitRepo { workdir }
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()?;

        if !output.status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", args.join(" ")),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String> {
        let mut child = Command::new("git")
            .current_dir(&self.workdir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let child_stdin = child.stdin.as_mut().ok_or_else(|| {
                StaircaseError::Other("Failed to open stdin for git command".into())
            })?;
            child_stdin.write_all(stdin.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", args.join(" ")),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn run_interactive(&self, args: &[&str]) -> Result<()> {
        let status = Command::new("git")
            .current_dir(&self.workdir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()?;

        if !status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", args.join(" ")),
                stdout: String::new(),
                stderr: String::new(),
            });
        }

        Ok(())
    }

    pub fn resolve_ref(&self, rev: &str) -> Result<String> {
        let stdout = self.run(&["rev-parse", "--verify", rev])?;
        Ok(stdout.trim().to_string())
    }

    pub fn resolve_ref_opt(&self, rev: &str) -> Result<Option<String>> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(["rev-parse", "--verify", rev])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()?;

        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .output()?;

        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(StaircaseError::GitCommandFailed {
                command: format!("git merge-base --is-ancestor {} {}", ancestor, descendant),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }),
        }
    }

    pub fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        let stdout = self.run(&["merge-base", a, b])?;
        Ok(stdout.trim().to_string())
    }

    pub fn commits_between(&self, base: &str, tip: &str) -> Result<Vec<String>> {
        let stdout = self.run(&["rev-list", "--reverse", &format!("{}..{}", base, tip)])?;
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }

    pub fn write_metadata(&self, metadata: &StaircaseMetadata) -> Result<String> {
        let json = serde_json::to_string_pretty(metadata)?;

        // 1. Hash and write the metadata JSON blob
        let blob_oid = self.run_with_stdin(&["hash-object", "-w", "--stdin"], &json)?;
        let blob_oid = blob_oid.trim();

        // 2. Create a tree containing the blob
        let tree_input = format!("100644 blob {}\tstaircase.json\n", blob_oid);
        let tree_oid = self.run_with_stdin(&["mktree"], &tree_input)?;
        let tree_oid = tree_oid.trim();

        // 3. Create a commit
        let commit_msg = format!("Update staircase {}", metadata.name);
        let mut commit_args = vec!["commit-tree", tree_oid, "-m", &commit_msg];

        // Check if ref already exists to use as parent
        let ref_name = format!("refs/staircases/{}/meta", metadata.id);
        let parent_oid = self.resolve_ref(&ref_name).ok();
        if let Some(ref parent) = parent_oid {
            commit_args.push("-p");
            commit_args.push(parent);
        }

        let commit_oid = self.run(&commit_args)?;
        let commit_oid = commit_oid.trim();

        // 4. Update the ref
        self.run(&["update-ref", &ref_name, commit_oid])?;

        Ok(commit_oid.to_string())
    }

    pub fn read_metadata(&self, id: &str) -> Result<StaircaseMetadata> {
        let ref_name = format!("refs/staircases/{}/meta", id);
        // Verify ref exists
        self.resolve_ref(&ref_name)?;

        let json = self.run(&["cat-file", "-p", &format!("{}:staircase.json", ref_name)])?;
        let metadata: StaircaseMetadata = serde_json::from_str(&json)?;
        Ok(metadata)
    }

    pub fn record_verification(
        &self,
        key: &str,
        kind: IdentityKind,
        results: &[VerificationResult],
    ) -> Result<String> {
        let json = serde_json::to_string_pretty(results)?;

        let blob_oid = self.run_with_stdin(&["hash-object", "-w", "--stdin"], &json)?;
        let blob_oid = blob_oid.trim();

        let tree_input = format!("100644 blob {}\tverification.json\n", blob_oid);
        let tree_oid = self.run_with_stdin(&["mktree"], &tree_input)?;
        let tree_oid = tree_oid.trim();

        let commit_msg = format!(
            "Record verification for staircase {} (kind: {:?})",
            key, kind
        );
        let mut commit_args = vec!["commit-tree", tree_oid, "-m", &commit_msg];

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
        let parent_oid = self.resolve_ref(&ref_name).ok();
        if let Some(ref parent) = parent_oid {
            commit_args.push("-p");
            commit_args.push(parent);
        }

        let commit_oid = self.run(&commit_args)?;
        let commit_oid = commit_oid.trim();

        self.run(&["update-ref", &ref_name, commit_oid])?;

        Ok(commit_oid.to_string())
    }

    pub fn list_staircases(&self) -> Result<Vec<StaircaseMetadata>> {
        let stdout = match self.run(&["for-each-ref", "--format=%(refname)", "refs/staircases/"]) {
            Ok(out) => out,
            Err(StaircaseError::GitCommandFailed { .. }) => {
                // If refs/staircases/ doesn't exist yet, it might error or return empty.
                // Usually it returns empty if no refs match, but let's be safe.
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };

        let mut staircases = Vec::new();
        for line in stdout.lines() {
            let refname = line.trim();
            if refname.starts_with("refs/staircases/") {
                // Check if it is a main staircase ref (not a step ref)
                // Main ref is refs/staircases/<id>
                // Step ref is refs/staircases/<id>/steps/<name>
                let parts: Vec<&str> = refname
                    .strip_prefix("refs/staircases/")
                    .unwrap()
                    .split('/')
                    .collect();
                if parts.len() == 2 && parts[1] == "meta" {
                    let id = parts[0];
                    if let Ok(meta) = self.read_metadata(id) {
                        staircases.push(meta);
                    }
                }
            }
        }
        Ok(staircases)
    }

    pub fn update_branch(&self, branch_name: &str, oid: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{}", branch_name);
        self.run(&["update-ref", &ref_name, oid])?;
        Ok(())
    }

    pub fn update_step_ref(&self, id: &str, step_name: &str, cut: &str) -> Result<()> {
        let ref_name = format!("refs/staircases/{}/steps/{}", id, step_name);
        self.run(&["update-ref", &ref_name, cut])?;
        Ok(())
    }

    pub fn delete_step_ref(&self, id: &str, step_name: &str) -> Result<()> {
        let ref_name = format!("refs/staircases/{}/steps/{}", id, step_name);
        self.run(&["update-ref", "-d", &ref_name])?;
        Ok(())
    }

    pub fn get_object_format(&self) -> Result<String> {
        let stdout = self.run(&["rev-parse", "--show-object-format"])?;
        Ok(stdout.trim().to_string())
    }

    pub fn get_tree_id(&self, rev: &str) -> Result<String> {
        let stdout = self.run(&["rev-parse", &format!("{}^{{tree}}", rev)])?;
        Ok(stdout.trim().to_string())
    }

    pub fn hash_data(&self, data: &str) -> Result<String> {
        let stdout = self.run_with_stdin(&["hash-object", "--stdin"], data)?;
        Ok(stdout.trim().to_string())
    }

    pub fn get_patch_id(&self, base: &str, tip: &str) -> Result<String> {
        let diff = self.run(&["diff-tree", "-p", base, tip])?;
        let stdout = self.run_with_stdin(&["patch-id"], &diff)?;
        Ok(stdout.split_whitespace().next().unwrap_or("").to_string())
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(["symbolic-ref", "-q", "HEAD"])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn local_branches(&self) -> Result<Vec<BranchInfo>> {
        let stdout = self.run(&[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)%09%(upstream)",
            "refs/heads/",
        ])?;

        let mut branches = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let refname = parts[0].to_string();
                let oid = parts[1].to_string();
                let upstream = if parts.len() >= 3 && !parts[2].is_empty() {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                branches.push(BranchInfo {
                    refname,
                    oid,
                    upstream,
                });
            }
        }
        Ok(branches)
    }

    pub fn delete_staircase_refs(&self, id: &str) -> Result<()> {
        // Delete all step refs first
        let step_refs_prefix = format!("refs/staircases/{}/steps/", id);
        if let Ok(stdout) = self.run(&["for-each-ref", "--format=%(refname)", &step_refs_prefix]) {
            for line in stdout.lines() {
                let refname = line.trim();
                self.run(&["update-ref", "-d", refname])?;
            }
        }
        // Delete meta ref
        let ref_name = format!("refs/staircases/{}/meta", id);
        self.run(&["update-ref", "-d", &ref_name])?;
        Ok(())
    }
}
