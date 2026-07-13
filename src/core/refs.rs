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

pub const PUBLIC_PREFIX: &str = "refs/staircases/";
pub const STATE_PREFIX: &str = "refs/staircase-state/";
pub const ARCHIVE_PREFIX: &str = "refs/staircase-archive/";

pub struct StaircaseRefs;

impl StaircaseRefs {
    pub fn public(name: &str) -> String {
        format!("{}{}", PUBLIC_PREFIX, name)
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
