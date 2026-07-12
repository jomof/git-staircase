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

pub struct GitCommand<'a> {
    repo: &'a GitRepo,
    args: Vec<String>,
    stdin: Option<String>,
    interactive: bool,
    check_status: bool,
    trim: bool,
}

impl<'a> GitCommand<'a> {
    pub fn new(repo: &'a GitRepo) -> Self {
        Self {
            repo,
            args: Vec::new(),
            stdin: None,
            interactive: false,
            check_status: true,
            trim: true,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<S: AsRef<str>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        for arg in args {
            self.args.push(arg.as_ref().to_string());
        }
        self
    }

    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn check_status(mut self, check: bool) -> Self {
        self.check_status = check;
        self
    }

    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    pub fn run(self) -> Result<String> {
        let trim = self.trim;
        let output = self.run_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if trim {
            Ok(stdout.trim().to_string())
        } else {
            Ok(stdout.into_owned())
        }
    }

    pub fn run_output(self) -> Result<std::process::Output> {
        let mut cmd = self.repo.git_cmd();
        cmd.args(&self.args);

        let output = if self.interactive {
            let status = cmd.status()?;
            std::process::Output {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        } else if let Some(input) = self.stdin {
            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut child_stdin = child.stdin.take().ok_or_else(|| {
                StaircaseError::Other("Failed to open stdin for git command".into())
            })?;

            thread::scope(|s| {
                s.spawn(move || {
                    let _ = child_stdin.write_all(input.as_bytes());
                });
                child.wait_with_output()
            })?
        } else {
            cmd.output()?
        };

        if self.check_status && !output.status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", self.args.join(" ")),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(output)
    }
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

    pub fn command(&self) -> GitCommand<'_> {
        GitCommand::new(self)
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        self.command().args(args).run()
    }

    pub fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String> {
        self.command().args(args).stdin(stdin).run()
    }

    pub fn run_interactive(&self, args: &[&str]) -> Result<()> {
        self.command().args(args).interactive(true).run_output()?;
        Ok(())
    }

    pub fn resolve_ref(&self, rev: &str) -> Result<String> {
        self.command().args(&["rev-parse", "--verify", rev]).run()
    }

    pub fn resolve_commit(&self, rev: &str) -> Result<String> {
        self.command()
            .args(&["rev-parse", "--verify", &format!("{}^{{commit}}", rev)])
            .run()
    }

    pub fn resolve_symbolic_full_name(&self, name: &str) -> Result<String> {
        let full_name = self
            .command()
            .args(&["rev-parse", "--symbolic-full-name", name])
            .run()?;
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
        let result = self
            .command()
            .args(&["rev-parse", "--verify", rev])
            .check_status(false)
            .run_output()?;

        if result.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&result.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let output = self
            .command()
            .args(&["merge-base", "--is-ancestor", ancestor, descendant])
            .check_status(false)
            .run_output()?;

        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => {
                if !output.status.success() {
                    return Err(StaircaseError::GitCommandFailed {
                        command: format!(
                            "git merge-base --is-ancestor {} {}",
                            ancestor, descendant
                        ),
                        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    });
                }
                Ok(false)
            }
        }
    }

    pub fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        self.command().args(&["merge-base", a, b]).run()
    }

    pub fn commits_between(&self, base: &str, tip: &str) -> Result<Vec<String>> {
        let stdout = self
            .command()
            .args(&["rev-list", "--reverse", &format!("{}..{}", base, tip)])
            .run()?;
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }

    pub fn update_branch(&self, branch_name: &str, oid: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{}", branch_name);
        self.command().args(&["update-ref", &ref_name, oid]).run()?;
        Ok(())
    }

    pub fn update_step_ref(&self, id: &str, step_name: &str, cut: &str) -> Result<()> {
        let ref_name = format!("refs/staircase-state/{}/steps/{}", id, step_name);
        self.command().args(&["update-ref", &ref_name, cut]).run()?;
        Ok(())
    }

    pub fn delete_step_ref(&self, id: &str, step_name: &str) -> Result<()> {
        let ref_name = format!("refs/staircase-state/{}/steps/{}", id, step_name);
        self.command()
            .args(&["update-ref", "-d", &ref_name])
            .run()?;
        Ok(())
    }

    pub fn get_object_format(&self) -> Result<String> {
        self.command()
            .args(&["rev-parse", "--show-object-format"])
            .run()
    }

    pub fn get_tree_id(&self, rev: &str) -> Result<String> {
        self.command()
            .args(&["rev-parse", &format!("{}^{{tree}}", rev)])
            .run()
    }

    pub fn hash_data(&self, data: &str) -> Result<String> {
        self.command()
            .args(&["hash-object", "--stdin"])
            .stdin(data)
            .run()
    }

    pub fn get_patch_id(&self, base: &str, tip: &str) -> Result<String> {
        let diff = self.command().args(&["diff-tree", "-p", base, tip]).run()?;
        let stdout = self.command().args(&["patch-id"]).stdin(diff).run()?;
        Ok(stdout.split_whitespace().next().unwrap_or("").to_string())
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = self
            .command()
            .args(&["symbolic-ref", "-q", "HEAD"])
            .check_status(false)
            .run_output()?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn local_branches(&self, pattern: Option<&str>) -> Result<Vec<BranchInfo>> {
        let stdout = self
            .command()
            .args(&[
                "for-each-ref",
                "--format=%(refname)%09%(objectname)%09%(upstream)",
                pattern.unwrap_or("refs/heads/"),
            ])
            .run()?;
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
        self.command()
            .args(&["update-ref", "--stdin"])
            .stdin(input)
            .run()?;
        Ok(())
    }
}
