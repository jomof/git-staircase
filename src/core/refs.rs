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

use crate::model::LifecycleState;
use crate::model::StaircaseMetadata;
use std::collections::BTreeMap;

pub const PUBLIC_PREFIX: &str = "refs/staircases/";
pub const STATE_PREFIX: &str = "refs/staircase-state/";
pub const ARCHIVE_PREFIX: &str = "refs/staircase-archive/";

pub struct StaircaseRefs;

impl StaircaseRefs {
    pub fn local(branch: &str) -> String {
        if branch.starts_with("refs/heads/") {
            branch.into()
        } else {
            format!("refs/heads/{}", branch)
        }
    }

    pub fn owned_branches(metadata: &StaircaseMetadata) -> BTreeMap<String, String> {
        metadata
            .steps
            .iter()
            .filter_map(|step| {
                step.branch
                    .as_ref()
                    .map(|branch| (Self::local(branch), step.cut.clone()))
            })
            .collect()
    }

    pub fn public(name: &str) -> String {
        format!("{}{}", PUBLIC_PREFIX, name)
    }

    pub fn public_optional(name: Option<&str>, state: LifecycleState) -> Option<String> {
        match (state, name) {
            (LifecycleState::Active, Some(name)) if !name.is_empty() => Some(Self::public(name)),
            _ => None,
        }
    }

    pub fn record(id: &str, state: LifecycleState) -> String {
        match state {
            LifecycleState::Active => Self::state_record(id),
            LifecycleState::Archived => Self::archive_record(id),
        }
    }

    pub fn step(id: &str, step_key: &str, state: LifecycleState) -> String {
        match state {
            LifecycleState::Active => Self::state_step(id, step_key),
            LifecycleState::Archived => Self::archive_step(id, step_key),
        }
    }

    pub fn state_record(id: &str) -> String {
        format!("{}{}/record", STATE_PREFIX, id)
    }

    pub fn state_descriptor(id: &str) -> String {
        format!("{}{}/descriptor", STATE_PREFIX, id)
    }

    pub fn state_step(id: &str, step_key: &str) -> String {
        format!("{}{}/steps/{}", STATE_PREFIX, id, step_key)
    }

    pub fn archive_record(id: &str) -> String {
        format!("{}{}/record", ARCHIVE_PREFIX, id)
    }

    pub fn archive_step(id: &str, step_key: &str) -> String {
        format!("{}{}/steps/{}", ARCHIVE_PREFIX, id, step_key)
    }

    pub fn archive_owned(id: &str, ref_id: &str) -> String {
        format!("{}{}/owned/{}", ARCHIVE_PREFIX, id, ref_id)
    }

    pub fn verification(id: &str) -> String {
        format!("{}{}/verification", PUBLIC_PREFIX, id)
    }

    pub fn revision_verification(rev: &str) -> String {
        format!("{}by-revision/{}/verification", PUBLIC_PREFIX, rev)
    }
}
