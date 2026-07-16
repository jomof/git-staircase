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
use super::types::WorktreeInfo;
use crate::error::Result;
use std::path::PathBuf;

impl GitRepo {
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
}
