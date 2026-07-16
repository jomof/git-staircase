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

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: String,
    pub oid: String,
    pub name: String,
}

impl TreeEntry {
    pub fn blob(oid: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            mode: "100644".to_string(),
            kind: "blob".to_string(),
            oid: oid.into(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
}
