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

#[test]
fn revision_identity_remains_implicit_but_stable_identity_adopts() {
    // ARRANGE: Create a repository with a single commit.
    let ctx = TestContext::new();
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("feature.txt", "feature content", "feature commit");

    // ACT: Query its identity using `git staircase status --json`.
    let (success, stdout, _) = ctx.run_staircase(&["status", "feature", "--json"]);
    assert!(success);

    // ASSERT: Verify `is_implicit: true`.
    assert!(
        stdout.contains("\"is_implicit\": true"),
        "Initially should be implicit"
    );

    // ACT: Query revision identity.
    let (id_ok, stdout_id, _) =
        ctx.run_staircase(&["id", "--kind", "revision", "feature", "--json"]);
    assert!(id_ok);
    assert!(stdout_id.contains("\"id\":"));

    // ACT: Query lineage identity.
    let (id_ok2, stdout_id2, _) =
        ctx.run_staircase(&["id", "--kind", "lineage", "feature", "--json"]);
    assert!(id_ok2);
    assert!(stdout_id2.contains("\"id\": \"implicit@"));

    // Verify no managed refs created by read-only query.
    let refs = ctx.run_git(&["for-each-ref", "refs/staircase*"]);
    assert!(
        refs.is_empty(),
        "Querying status should not create managed refs, but found: {}",
        refs
    );

    // ACT: Run `git staircase adopt`.
    let (adopt_ok, _, _) = ctx.run_staircase(&["adopt", "feature", "feature"]);
    assert!(adopt_ok);

    // ASSERT: Verify the staircase is now reported as managed (not implicit).
    let (success2, stdout2, _) = ctx.run_staircase(&["status", "feature", "--json"]);
    assert!(success2);
    assert!(
        stdout2.contains("\"is_implicit\": false"),
        "Should NOT be implicit after adoption"
    );

    // ASSERT: Verify that a persistent record has been created in `refs/staircase-state/`.
    let refs_after = ctx.run_git(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/staircase-state/",
    ]);
    assert!(
        !refs_after.is_empty(),
        "Adoption should create managed records in refs/staircase-state/, but got: '{}'",
        refs_after
    );
}

#[test]
fn archive_always_adopts_implicit_selection() {
    // ARRANGE: Create a local branch `feature` ahead of `main` (an implicit staircase).
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

    // ACT: Run `git staircase archive feature`.
    let (archive_ok, stdout_archive, stderr) = ctx.run_staircase(&["archive", "feature"]);
    assert!(
        archive_ok,
        "Archive command failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout_archive, stderr
    );

    // ASSERT: Verify success.
    let (success2, stdout2, stderr2) = ctx.run_staircase(&["status", "feature", "--json"]);
    assert!(
        success2,
        "Status command failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout2, stderr2
    );

    // Check if it's no longer implicit.
    assert!(
        stdout2.contains("\"is_implicit\": false"),
        "Staircase should have been adopted during archive"
    );
    // Check if lifecycle state is archived.
    assert!(
        stdout2.contains("\"state\": \"archived\""),
        "Lifecycle state should be archived"
    );

    // Verify a record exists in `refs/staircase-archive/`.
    let refs = ctx.run_git(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/staircase-archive/",
    ]);
    assert!(
        !refs.is_empty(),
        "Archive records should exist in refs/staircase-archive/"
    );
}

#[test]
fn persistent_metadata_always_adopts() {
    // ARRANGE: Create a repository with an implicit staircase (local branch ahead of its anchor).
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

    // ACT: Use git staircase metadata set-title to set a title.
    let (set_ok, stdout_set, stderr_set) =
        ctx.run_staircase(&["metadata", "set-title", "feature", "My Feature"]);
    assert!(
        set_ok,
        "Metadata set-title command failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout_set, stderr_set
    );

    // ASSERT: Verify that git staircase status --json now reports is_implicit: false.
    let (success2, stdout2, stderr2) = ctx.run_staircase(&["status", "feature", "--json"]);
    assert!(
        success2,
        "Status command failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout2, stderr2
    );
    assert!(
        stdout2.contains("\"is_implicit\": false"),
        "Should be adopted after setting metadata. Output: {}",
        stdout2
    );

    // Verify that a record exists in refs/staircase-state/.
    let refs = ctx.run_git(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/staircase-state/",
    ]);
    assert!(
        !refs.is_empty(),
        "Adoption should create managed records in refs/staircase-state/"
    );
}

#[test]
fn no_adopt_fails_before_mutation_and_reports_reason() {
    // ARRANGE: Create a repository with an implicit staircase (e.g., a local branch ahead of its anchor).
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

    // ACT: Attempt to set metadata (e.g., `git staircase metadata set-title feature "My Title"`) with the `--no-adopt` flag.
    let (set_ok, _stdout_set, stderr_set) =
        ctx.run_staircase(&["--no-adopt", "metadata", "set-title", "feature", "My Title"]);

    // ASSERT: Verify the command fails with a non-zero exit code and a message indicating adoption was required but forbidden.
    assert!(!set_ok, "Command should have failed with --no-adopt");
    assert!(
        stderr_set.contains("adoption required") || stderr_set.contains("forbidden"),
        "Error message should mention adoption requirement/forbidden. Stderr: {}",
        stderr_set
    );

    // Verify that no managed refs (in `refs/staircase-state/`) were created.
    let refs = ctx.run_git(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/staircase-state/",
    ]);
    assert!(
        refs.is_empty(),
        "No managed refs should be created, but found: {}",
        refs
    );
}

#[test]
fn rebase_adopts_only_for_continuity_or_stale_state() {
    // ARRANGE: Create an implicit staircase.
    let ctx = TestContext::new();
    ctx.run_git(&["checkout", "-b", "feature"]);
    ctx.commit("feature.txt", "feature content", "feature commit");

    // Create a new branch to rebase onto.
    ctx.run_git(&["checkout", "main"]);
    ctx.run_git(&["checkout", "-b", "other"]);
    ctx.commit("other.txt", "other content", "other commit");

    // ACT: Perform a clean rebase onto 'other'.
    let (rebase_ok, stdout_rebase, stderr_rebase) =
        ctx.run_staircase(&["rebase", "--onto", "other", "refs/heads/feature"]);
    assert!(
        rebase_ok,
        "Rebase command failed:\nSTDOUT: {}\nSTDERR: {}",
        stdout_rebase, stderr_rebase
    );

    // ASSERT: Verify it remains implicit.
    let (success, stdout, _) = ctx.run_staircase(&["status", "feature", "--json"]);
    assert!(success);
    assert!(
        stdout.contains("\"is_implicit\": true"),
        "Should remain implicit after clean rebase. Output: {}",
        stdout
    );
}
