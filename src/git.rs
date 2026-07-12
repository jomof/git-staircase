use crate::error::{Result, StaircaseError};
use crate::model::BranchInfo;
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

    pub fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.workdir);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd
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
    pub fn resolve_commit(&self, rev: &str) -> Result<String> {
        let stdout = self.run(&["rev-parse", "--verify", &format!("{}^{{commit}}", rev)])?;
        Ok(stdout.trim().to_string())
    }

    pub fn resolve_symbolic_full_name(&self, name: &str) -> Result<String> {
        let stdout = self.run(&["rev-parse", "--symbolic-full-name", name])?;
        let full_name = stdout.trim().to_string();
        if full_name.is_empty() {
            return Err(StaircaseError::Other(format!(
                "Could not resolve \"{}\" to a full refname",
                name
            )));
        }
        Ok(full_name)
    }

    pub fn resolve_commit_opt(&self, rev: &str) -> Result<Option<String>> {
        self.resolve_ref_opt(&format!("{}^{{commit}}", rev))
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

    pub fn local_branches(&self, pattern: Option<&str>) -> Result<Vec<BranchInfo>> {
        let stdout = self.run(&[
            "for-each-ref",
            "--format=%(refname)%09%(objectname)%09%(upstream)",
            pattern.unwrap_or("refs/heads/"),
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

    pub fn update_refs_transaction(&self, commands: &[String]) -> Result<()> {
        if commands.is_empty() {
            return Ok(());
        }
        let input = commands.join("\n") + "\n";
        self.run_with_stdin(&["update-ref", "--stdin"], &input)?;
        Ok(())
    }
}
