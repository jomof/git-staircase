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

mod common;

use common::*;

#[test]
fn observation_never_adopts() {
    // ARRANGE: Create a repository with an implicit staircase (e.g., local branch ahead of its anchor).
    let ctx = TestContext::new();
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("feature.txt", "feature content", "feature commit");

    // Verify it is discovered as implicit initially.
    let (success, stdout, _) = ctx.run_staircase(&["list", "--json"]);
    assert!(success);
    assert!(
        stdout.contains("\"is_implicit\": true"),
        "Initially should be implicit"
    );

    // ACT: Run git staircase list, git staircase show, and git staircase status.
    let (list_ok, _, _) = ctx.run_staircase(&["list"]);
    assert!(list_ok);
    let (show_ok, _, _) = ctx.run_staircase(&["show", "feature"]);
    assert!(show_ok);
    let (status_ok, _, _) = ctx.run_staircase(&["status", "feature"]);
    assert!(status_ok);

    // ASSERT: Verify that the staircase remains labeled as (implicit) and no persistent records are created.
    let (success, stdout, _) = ctx.run_staircase(&["list", "--json"]);
    assert!(success);
    assert!(
        stdout.contains("\"is_implicit\": true"),
        "Should remain implicit after observation"
    );

    // We check that no managed refs exist.
    // refs/staircase* includes refs/staircases/, refs/staircase-state/, refs/staircase-archive/
    let refs = ctx.run_git(&["for-each-ref", "refs/staircase*"]);
    assert!(
        refs.is_empty(),
        "Observation commands should not create managed refs, but found: {}",
        refs
    );
}
