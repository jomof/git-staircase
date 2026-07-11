use crate::error::{Result, StaircaseError};
use crate::model::{
    BranchInfo, IdentityKind, StaircaseMetadata, Step, VerificationPolicy, VerificationResult,
};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub workdir: PathBuf,
}

impl GitRepo {
    pub fn new(workdir: PathBuf) -> Self {
        GitRepo { workdir }
    }

    fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.workdir);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd
    }

    fn commit_json_data<T: serde::Serialize>(
        &self,
        ref_name: &str,
        data: &T,
        filename: &str,
        commit_msg: &str,
    ) -> Result<String> {
        let json = serde_json::to_string_pretty(data)?;

        // 1. Hash and write the JSON blob
        let blob_oid = self.run_with_stdin(&["hash-object", "-w", "--stdin"], &json)?;
        let blob_oid = blob_oid.trim();

        // 2. Create a tree containing the blob
        let tree_input = format!("100644 blob {}\t{}\n", blob_oid, filename);
        let tree_oid = self.run_with_stdin(&["mktree"], &tree_input)?;
        let tree_oid = tree_oid.trim();

        // 3. Create a commit
        let mut commit_args = vec!["commit-tree", tree_oid, "-m", commit_msg];

        // Check if ref already exists to use as parent
        let parent_oid = self.resolve_ref_opt(ref_name).unwrap_or(None);
        if let Some(ref parent) = parent_oid {
            commit_args.push("-p");
            commit_args.push(parent);
        }

        let commit_oid = self.run(&commit_args)?;
        let commit_oid = commit_oid.trim();

        // 4. Update the ref
        self.run(&["update-ref", ref_name, commit_oid])?;

        Ok(commit_oid.to_string())
    }

    fn exec(
        &self,
        args: &[&str],
        stdin: Option<&str>,
        interactive: bool,
    ) -> Result<std::process::Output> {
        let mut cmd = self.git_cmd();
        cmd.args(args);

        if interactive {
            let status = cmd.status()?;
            Ok(std::process::Output {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        } else if let Some(input) = stdin {
            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut child_stdin = child.stdin.take().ok_or_else(|| {
                StaircaseError::Other("Failed to open stdin for git command".into())
            })?;

            let input = input.to_string();
            thread::scope(|s| {
                s.spawn(move || {
                    let _ = child_stdin.write_all(input.as_bytes());
                });
                child.wait_with_output()
            })
            .map_err(Into::into)
        } else {
            cmd.output().map_err(Into::into)
        }
    }

    fn check_status(&self, output: &std::process::Output, args: &[&str]) -> Result<()> {
        if !output.status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", args.join(" ")),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = self.exec(args, None, false)?;
        self.check_status(&output, args)?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String> {
        let output = self.exec(args, Some(stdin), false)?;
        self.check_status(&output, args)?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn run_interactive(&self, args: &[&str]) -> Result<()> {
        let output = self.exec(args, None, true)?;
        self.check_status(&output, args)?;
        Ok(())
    }

    pub fn resolve_ref(&self, rev: &str) -> Result<String> {
        let stdout = self.run(&["rev-parse", "--verify", rev])?;
        Ok(stdout.trim().to_string())
    }

    pub fn resolve_ref_opt(&self, rev: &str) -> Result<Option<String>> {
        let args = ["rev-parse", "--verify", rev];
        let output = self.exec(&args, None, false)?;

        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let args = ["merge-base", "--is-ancestor", ancestor, descendant];
        let output = self.exec(&args, None, false)?;

        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => {
                self.check_status(&output, &args)?;
                Ok(false)
            }
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
        let mut descriptor = String::new();
        descriptor.push_str("git-staircase-descriptor 1\n");
        let format = self.get_object_format()?;
        descriptor.push_str(&format!("object-format {}\n", format));
        descriptor.push_str(&format!("lineage {}\n", metadata.id));
        descriptor.push_str("state clean\n");
        descriptor.push_str(&format!("target-ref {}\n", metadata.target));

        let target_oid = self.resolve_ref(&metadata.target)?;
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

        let blob_oid = self.run_with_stdin(&["hash-object", "-w", "--stdin"], &descriptor)?;
        let blob_oid = blob_oid.trim().to_string();

        let public_ref = format!("refs/staircases/{}", metadata.name);
        self.run(&["update-ref", &public_ref, &blob_oid])?;

        let state_ref = format!("refs/staircase-state/{}/descriptor", metadata.id);
        self.run(&["update-ref", &state_ref, &blob_oid])?;

        // Also update step refs for reachability
        for step in &metadata.steps {
            let step_ref = format!("refs/staircase-state/{}/steps/{}", metadata.id, step.name);
            self.run(&["update-ref", &step_ref, &step.cut])?;
        }

        Ok(blob_oid)
    }

    pub fn read_metadata(&self, id_or_name: &str) -> Result<StaircaseMetadata> {
        let name_ref = format!("refs/staircases/{}", id_or_name);
        let id_ref = format!("refs/staircase-state/{}/descriptor", id_or_name);

        let (ref_name, is_name) = if self.resolve_ref_opt(&name_ref)?.is_some() {
            (name_ref, true)
        } else if self.resolve_ref_opt(&id_ref)?.is_some() {
            (id_ref, false)
        } else {
            return Err(crate::error::StaircaseError::Other(format!(
                "Staircase not found: {}",
                id_or_name
            )));
        };

        let content = self.run(&["cat-file", "-p", &ref_name])?;
        let mut meta = self.parse_descriptor(&content)?;

        if is_name {
            meta.name = id_or_name.to_string();
        } else {
            // If we read from ID, we might need to find the name elsewhere
            // For now, let's see if we can find a ref in refs/staircases/ pointing to this descriptor
            let oid = self.resolve_ref(&ref_name)?;
            if let Ok(stdout) = self.run(&["for-each-ref", "--points-at", &oid, "refs/staircases/"])
            {
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

    fn parse_descriptor(&self, content: &str) -> Result<StaircaseMetadata> {
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
        &self,
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

        self.commit_json_data(&ref_name, &results, "verification.json", &commit_msg)
    }

    pub fn list_staircases(&self) -> Result<Vec<StaircaseMetadata>> {
        let mut staircases = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Check public names
        if let Ok(stdout) = self.run(&["for-each-ref", "--format=%(refname)", "refs/staircases/"]) {
            for line in stdout.lines() {
                let refname = line.trim();
                if refname.starts_with("refs/staircases/") {
                    let name = refname.strip_prefix("refs/staircases/").unwrap();
                    if !name.contains('/') {
                        if let Ok(meta) = self.read_metadata(name) {
                            seen_ids.insert(meta.id.clone());
                            staircases.push(meta);
                        }
                    }
                }
            }
        }

        // Check internal state for any missed ones (unnamed managed staircases)
        if let Ok(stdout) = self.run(&[
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
                            if let Ok(meta) = self.read_metadata(id) {
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

    pub fn update_branch(&self, branch_name: &str, oid: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{}", branch_name);
        self.run(&["update-ref", &ref_name, oid])?;
        Ok(())
    }

    pub fn update_step_ref(&self, id: &str, step_name: &str, cut: &str) -> Result<()> {
        let ref_name = format!("refs/staircase-state/{}/steps/{}", id, step_name);
        self.run(&["update-ref", &ref_name, cut])?;
        Ok(())
    }

    pub fn delete_step_ref(&self, id: &str, step_name: &str) -> Result<()> {
        let ref_name = format!("refs/staircase-state/{}/steps/{}", id, step_name);
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
        let args = ["symbolic-ref", "-q", "HEAD"];
        let output = self.exec(&args, None, false)?;
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

    pub fn delete_staircase_refs(&self, id: &str, name: &str) -> Result<()> {
        // Delete state refs
        let state_prefix = format!("refs/staircase-state/{}/", id);
        if let Ok(stdout) = self.run(&["for-each-ref", "--format=%(refname)", &state_prefix]) {
            for line in stdout.lines() {
                let refname = line.trim();
                self.run(&["update-ref", "-d", refname])?;
            }
        }
        // Delete public ref
        let ref_name = format!("refs/staircases/{}", name);
        self.run(&["update-ref", "-d", &ref_name])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_repo() -> (TempDir, GitRepo) {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().to_path_buf();

        let status = Command::new("git")
            .current_dir(&repo_path)
            .args(["init", "-b", "main"])
            .status()
            .unwrap();
        assert!(status.success());

        // Initial commit to have a HEAD
        fs::write(repo_path.join("init.txt"), "init").unwrap();
        let status = Command::new("git")
            .current_dir(&repo_path)
            .args(["add", "."])
            .status()
            .unwrap();
        assert!(status.success());

        let status = Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-m", "initial"])
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .status()
            .unwrap();
        assert!(status.success());

        (tmp, GitRepo::new(repo_path))
    }

    #[test]
    fn test_git_cmd_setup() {
        let (_tmp, repo) = setup_repo();
        let cmd = repo.git_cmd();
        assert_eq!(cmd.get_program(), "git");
        // We can't easily check current_dir and env from Command without unstable features or just running it.
        let output = repo.run(&["rev-parse", "--is-inside-work-tree"]).unwrap();
        assert_eq!(output.trim(), "true");
    }

    #[test]
    fn test_commit_json_data() {
        let (_tmp, repo) = setup_repo();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestData {
            foo: String,
        }

        let data = TestData {
            foo: "bar".to_string(),
        };
        let ref_name = "refs/test/json";
        let filename = "test.json";
        let commit_msg = "test commit";

        let commit_oid = repo
            .commit_json_data(ref_name, &data, filename, commit_msg)
            .unwrap();
        assert!(!commit_oid.is_empty());

        // Verify ref updated
        let resolved = repo.resolve_ref(ref_name).unwrap();
        assert_eq!(resolved, commit_oid);

        // Verify content
        let json = repo
            .run(&["cat-file", "-p", &format!("{}:{}", commit_oid, filename)])
            .unwrap();
        let read_data: TestData = serde_json::from_str(&json).unwrap();
        assert_eq!(read_data, data);

        // Verify parent
        let data2 = TestData {
            foo: "baz".to_string(),
        };
        let commit_oid2 = repo
            .commit_json_data(ref_name, &data2, filename, "second commit")
            .unwrap();

        let parents = repo
            .run(&["rev-list", "--parents", "-n", "1", &commit_oid2])
            .unwrap();
        let parts: Vec<&str> = parents.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], commit_oid);
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;
    use crate::error::StaircaseError;
    use tempfile::TempDir;

    #[test]
    fn test_git_command_failed_captures_info() {
        // ARRANGE
        let tmp = TempDir::new().unwrap();
        let repo = GitRepo::new(tmp.path().to_path_buf());
        let _ = std::process::Command::new("git")
            .current_dir(tmp.path())
            .arg("init")
            .status();

        // ACT
        let result = repo.run(&["rev-parse", "HEAD"]);

        // ASSERT
        match result {
            Err(StaircaseError::GitCommandFailed {
                command,
                stdout: _,
                stderr,
            }) => {
                assert!(command.contains("git rev-parse HEAD"));
                assert!(stderr.contains("fatal: ambiguous argument 'HEAD'"));
            }
            _ => panic!("Expected GitCommandFailed error, got {:?}", result),
        }
    }
}
