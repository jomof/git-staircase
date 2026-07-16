use super::GitRepo;
use crate::error::{Result, StaircaseError};
use std::io::Write;
use std::process::Stdio;
use std::thread;

pub struct GitCommand<'a> {
    pub(super) repo: &'a GitRepo,
    pub(super) args: Vec<String>,
    pub(super) stdin: Option<String>,
    pub(super) interactive: bool,
    pub(super) check_status: bool,
    pub(super) trim: bool,
    pub(super) envs: std::collections::HashMap<String, String>,
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
            envs: std::collections::HashMap::new(),
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

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
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
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
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
