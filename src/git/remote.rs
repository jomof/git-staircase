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

use super::GitRepo;
use crate::error::Result;

impl GitRepo {
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
