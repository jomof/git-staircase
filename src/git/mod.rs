pub mod cmd;
pub mod graph;
pub mod objects;
pub mod refs;

pub use cmd::GitCommand;
pub use objects::TreeEntry;

use crate::error::Result;
use crate::memoization::Memoizer;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub workdir: PathBuf,
    pub memoizer: Memoizer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
}

impl GitRepo {
    pub fn new(workdir: PathBuf) -> Self {
        GitRepo {
            workdir,
            memoizer: Memoizer::new(),
        }
    }

    pub fn with_memoizer(workdir: PathBuf, memoizer: Memoizer) -> Self {
        GitRepo { workdir, memoizer }
    }

    pub fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.workdir);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd.env("GIT_OPTIONAL_LOCKS", "0");
        cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
        cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
        cmd
    }

    pub fn command(&self) -> GitCommand<'_> {
        GitCommand::new(self)
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let trim = !args.first().map_or(false, |&cmd| cmd == "cat-file");
        self.command().args(args).trim(trim).run()
    }

    pub fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String> {
        self.command().args(args).stdin(stdin).run()
    }

    pub fn run_interactive(&self, args: &[&str]) -> Result<()> {
        self.command().args(args).interactive(true).run_output()?;
        Ok(())
    }

    pub fn get_object_format(&self) -> Result<String> {
        if let Some(fmt) = self.memoizer.get_object_format() {
            return Ok(fmt);
        }
        let fmt = self
            .command()
            .args(&["rev-parse", "--show-object-format"])
            .run()?;
        self.memoizer.set_object_format(&fmt);
        Ok(fmt)
    }

    pub fn repository_identity(&self) -> Result<String> {
        let object_directory = self
            .command()
            .args(&["rev-parse", "--git-path", "objects"])
            .run()?;
        let path = PathBuf::from(object_directory);
        let path = if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        };
        let canonical = path.canonicalize()?;
        Ok(canonical.to_string_lossy().into_owned())
    }

    pub fn git_dir(&self) -> Result<PathBuf> {
        let raw = self.command().args(["rev-parse", "--git-dir"]).run()?;
        let path = PathBuf::from(raw);
        Ok(if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        })
    }

    pub fn common_dir(&self) -> Result<PathBuf> {
        let raw = self
            .command()
            .args(["rev-parse", "--git-common-dir"])
            .run()?;
        let path = PathBuf::from(raw);
        Ok(if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        })
    }

    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self
            .command()
            .args(["worktree", "list", "--porcelain"])
            .run()?;
        let mut worktrees = Vec::new();
        let mut current: Option<WorktreeInfo> = None;
        for line in output.lines().chain(std::iter::once("")) {
            if let Some(path) = line.strip_prefix("worktree ") {
                if let Some(info) = current.take() {
                    worktrees.push(info);
                }
                current = Some(WorktreeInfo {
                    path: PathBuf::from(path),
                    head: None,
                    branch: None,
                });
            } else if let Some(head) = line.strip_prefix("HEAD ") {
                if let Some(info) = current.as_mut() {
                    info.head = Some(head.into());
                }
            } else if let Some(branch) = line.strip_prefix("branch ") {
                if let Some(info) = current.as_mut() {
                    info.branch = Some(branch.into());
                }
            } else if line.is_empty() {
                if let Some(info) = current.take() {
                    worktrees.push(info);
                }
            }
        }
        Ok(worktrees)
    }

    pub fn push(&self, remote: &str, refspecs: &[&str], atomic: bool, dry_run: bool) -> Result<()> {
        let mut cmd = self.command().arg("push");
        if atomic {
            cmd = cmd.arg("--atomic");
        }
        if dry_run {
            cmd = cmd.arg("--dry-run");
        }
        cmd.arg(remote).args(refspecs).run()?;
        Ok(())
    }

    pub fn fetch(&self, remote: &str, refspecs: &[&str], dry_run: bool) -> Result<()> {
        let mut cmd = self.command().arg("fetch");
        if dry_run {
            cmd = cmd.arg("--dry-run");
        }
        cmd.arg(remote).args(refspecs).run()?;
        Ok(())
    }
}
