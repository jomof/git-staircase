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
use super::types::TreeEntry;
use crate::error::{Result, StaircaseError};
use serde::Serialize;

impl GitRepo {
    pub fn get_tree_id(&self, rev: &str) -> Result<String> {
        let commit_oid = self.resolve_commit(rev)?;
        if let Some(tree) = self.memoizer.get_tree_id(&commit_oid) {
            return Ok(tree);
        }
        let tree = self
            .command()
            .args(&["rev-parse", &format!("{}^{{tree}}", commit_oid)])
            .run()?;
        self.memoizer.set_tree_id(&commit_oid, &tree);
        Ok(tree)
    }

    pub fn write_blob(&self, content: &str) -> Result<String> {
        self.command()
            .args(&["hash-object", "-w", "--stdin"])
            .stdin(content)
            .run()
    }

    pub fn write_json<T: Serialize>(&self, data: &T) -> Result<String> {
        let json = serde_json::to_string_pretty(data)?;
        self.write_blob(&json)
    }

    pub fn write_tree(&self, entries: &[TreeEntry]) -> Result<String> {
        let mut input = String::new();
        for entry in entries {
            input.push_str(&format!(
                "{} {} {}\t{}\0",
                entry.mode, entry.kind, entry.oid, entry.name
            ));
        }
        self.command().args(&["mktree", "-z"]).stdin(input).run()
    }

    pub fn hash_data(&self, data: &str) -> Result<String> {
        if let Some(hash) = self.memoizer.get_hash_data(data) {
            return Ok(hash);
        }
        let hash = self
            .command()
            .args(&["hash-object", "--stdin"])
            .stdin(data)
            .run()?;
        self.memoizer.set_hash_data(data, &hash);
        Ok(hash)
    }

    pub fn get_object_type(&self, oid: &str) -> Result<String> {
        Ok(self.run(&["cat-file", "-t", oid])?.trim().to_string())
    }

    pub fn cat_file(&self, oid: &str) -> Result<String> {
        self.command().args(&["cat-file", "-p", oid]).run()
    }

    pub fn cat_file_bytes(&self, oid: &str) -> Result<Vec<u8>> {
        self.command().args(&["cat-file", "-p", oid]).run_bytes()
    }

    pub fn ls_tree(&self, oid: &str) -> Result<Vec<TreeEntry>> {
        let output = self.run(&["ls-tree", oid])?;
        let mut entries = Vec::new();
        for line in output.lines() {
            let (metadata, name) = line.split_once("\t").ok_or_else(|| {
                StaircaseError::Other(format!("invalid ls-tree entry in {}", oid))
            })?;
            let fields: Vec<_> = metadata.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(StaircaseError::Other(format!(
                    "invalid ls-tree metadata in {}",
                    oid
                )));
            }
            entries.push(TreeEntry {
                mode: fields[0].to_string(),
                kind: fields[1].to_string(),
                oid: fields[2].to_string(),
                name: name.to_string(),
            });
        }
        Ok(entries)
    }

    pub fn commit_tree(&self, tree_oid: &str, parents: &[&str], message: &str) -> Result<String> {
        let mut cmd = self
            .command()
            .arg("commit-tree")
            .arg(tree_oid)
            .arg("-m")
            .arg(message);
        for parent in parents {
            cmd = cmd.arg("-p").arg(*parent);
        }
        cmd.run()
    }

    pub fn read_tree_file(&self, rev: &str, path: &str) -> Result<String> {
        self.cat_file(&format!("{}:{}", rev, path))
    }
}
