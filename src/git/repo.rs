/*
 * Copyright (C) 2024 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use super::command::GitCommand;
use crate::error::Result;
use crate::memoization::Memoizer;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub workdir: PathBuf,
    pub memoizer: Memoizer,
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
}
