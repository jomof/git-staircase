# Conformance Test Map

## Status

This file maps every normative appendix journey, decision-table category, corner-case category, and compact scenario to one intended Rust integration test. `[existing]` names a test present today. `[planned]` reserves the file and exact test name that will own the black-box assertion; nearby unit coverage does not make a planned conformance test complete.

Conformance tests use the compiled `git-staircase` binary, temporary repositories, full machine identifiers, isolated XDG directories, and fake provider subprocesses. Provider network tests use deterministic local fakes and assert the provider request boundary as well as the user-visible result.

## Core specification Appendix A

* A.2.1 difficult `repo` and Gerrit starting state — `[existing] tests/conformance_core_journeys.rs::journey_1_bootstraps_repo_gerrit_and_publishes_three_reviews`
* A.2.2 amend the bottom step and resolve two restack conflicts — `[existing] tests/conformance_core_journeys.rs::journey_1_amend_preserves_draft_and_review_identity_across_conflicts`
* A.2.3 rebase the whole staircase with nonadjacent conflicts — `[existing] tests/local_journeys_test.rs::conflict_pause_has_recovery_refs_and_continue_is_deterministic`
* A.2.4 externally owned `repo sync` conflict — `[existing] tests/provider_journeys_test.rs::repo_journey_3_external_sync_operation_remains_external`
* A.2.5 split a reviewed middle step — `[existing] tests/provider_journeys_test.rs::gerrit_journey_3_split_preserves_one_identity`
* A.3 reshape a discovered multi-branch stack without adoption — `[existing] tests/reshape_test.rs` (implicit reshape coverage)
* A.4 metadata-only cut triggers adoption — `[existing] tests/spec_align_reshape_flags_test.rs::test_split_no_ref_triggers_adoption`
* A.5 edit a lower step from another worktree with partial staging — `[existing] tests/conformance_core_journeys.rs::journey_4_cross_worktree_materialization_preserves_partial_staging`
* A.6 land a lower prefix and preserve conceptual identity — `[existing] tests/local_journeys_test.rs::partial_landing_keeps_surviving_ids_and_renumbers_layout`
* A.7 archive, collide, and restore branchlessly — `[existing] tests/local_journeys_test.rs::archive_removes_active_names_and_owned_branches`
* A.8 two implicit staircases with one human name — `[existing] tests/repro_duplicate_names.rs`
* A.9 shared-prefix family path extraction — `[existing] stacksaw/crates/stacksaw-git/tests/canonical_projection.rs::forked_discovery_lists_canonical_family_paths`
* A.10 uncertain review upload reconciliation — `[existing] tests/provider_journeys_test.rs::gerrit_journey_5_uncertain_upload_requires_reconciliation`

## Core specification Appendix B

* Observation (`discover`, `list`, inspection) — `[existing] tests/conformance_adoption.rs::observation_never_adopts`
* Revision-derived identity — `[existing] tests/conformance_adoption.rs::revision_identity_remains_implicit_but_stable_identity_adopts`
* Exact verification versus persistent verification history — `[planned] tests/conformance_adoption.rs::verification_adopts_only_for_lineage_relative_history`
* `split` — `[existing] tests/spec_align_reshape_flags_test.rs::test_split_no_ref_triggers_adoption`
* `join` — `[existing] tests/spec_align_reshape_flags_test.rs::test_join_keep_boundary_ref_triggers_adoption`
* Append commit to tip step — `[existing] tests/conformance_adoption.rs::append_adopts_only_for_durable_association`
* Add a new step — `[existing] tests/conformance_adoption.rs::new_step_adopts_only_for_metadata_only_cut`
* `reorder` — `[existing] tests/conformance_adoption.rs::reorder_adopts_only_when_state_or_identity_must_survive`
* `move` changes between steps — `[planned] tests/conformance_adoption.rs::move_adopts_only_for_stable_or_intermediate_state`
* Complete `rebase` — `[planned] tests/conformance_adoption.rs::rebase_adopts_only_for_continuity_or_stale_state`
* Clean `restack` — `[planned] tests/conformance_adoption.rs::restack_adopts_only_to_remember_stale_relationship`
* `archive` — `[existing] tests/conformance_adoption.rs::archive_always_adopts_implicit_selection`
* Persistent name, description, labels, and links — `[planned] tests/conformance_adoption.rs::persistent_metadata_always_adopts`
* Persistent discovery override — `[existing] tests/local_journeys_test.rs::policy_and_discovery_are_structural_record_revisions`
* Persistent policy — `[existing] tests/local_journeys_test.rs::policy_and_discovery_are_structural_record_revisions`
* Immutable snapshot tag — `[existing] tests/local_journeys_test.rs::annotated_snapshot_tag_supports_configured_openpgp_signer`
* Persistent draft attachment — `[planned] tests/conformance_adoption.rs::persistent_draft_attachment_always_adopts`
* Invocation-local draft materialization — `[planned] tests/conformance_adoption.rs::draft_materialization_adopts_only_for_durable_result`
* Review association — `[planned] tests/conformance_adoption.rs::review_association_adopts_only_when_retained`
* Partial landing — `[planned] tests/conformance_adoption.rs::partial_landing_adopts_only_for_continuity`
* Delete implicit materializing refs — `[planned] tests/conformance_adoption.rs::explicit_implicit_ref_deletion_does_not_adopt`
* Appendix-wide `--no-adopt` and structured adoption event — `[existing] tests/conformance_adoption.rs::no_adopt_fails_before_mutation_and_reports_reason`

## Core specification Appendix C

### Discovery, identity, and selection

* Integration branch exactly at anchor is not an empty staircase — `[planned] tests/conformance_corner_cases.rs::integration_branch_at_anchor_is_not_discovered`
* Branch named `main` ahead of its anchor may be work — `[existing] tests/conformance_corner_cases.rs::main_ahead_of_anchor_is_valid_work`
* Equivalent discovery sources collapse and merge provenance — `[planned] tests/conformance_corner_cases.rs::equivalent_discovery_sources_collapse`
* Same top with different lower cuts remains distinct — `[planned] tests/conformance_corner_cases.rs::same_top_different_cuts_remain_distinct`
* Different symbolic targets resolving to one anchor collapse — `[planned] tests/conformance_corner_cases.rs::equivalent_symbolic_targets_collapse`
* Identical OIDs in distinct repositories remain distinct — `[planned] tests/conformance_corner_cases.rs::repository_identity_separates_equal_oids`
* Distinct staircases with one name are qualified — `[planned] tests/conformance_corner_cases.rs::same_name_staircases_are_structurally_qualified`
* Listing and selection use one canonicalized set — `[planned] tests/conformance_corner_cases.rs::listing_and_selection_share_canonical_set`
* Colliding abbreviated structural keys are extended — `[planned] tests/conformance_corner_cases.rs::structural_key_abbreviation_extends_until_unique`
* Cross-type selector interpretations collapse or diagnose — `[existing] tests/ambiguity_test.rs::test_selector_ambiguity_with_git_revision`
* Suggested ambiguity remedies actually disambiguate — `[planned] tests/conformance_corner_cases.rs::ambiguity_remedies_use_effective_typed_selectors`
* Revision range where one commit is required is rejected — `[planned] tests/conformance_corner_cases.rs::revision_range_is_rejected_for_commit_input`
* Blob or tree where a commit is required is rejected — `[existing] tests/type_safety_test.rs::test_resolve_commit_with_blob_tag_should_fail`
* Empty step is rejected before mutation — `[existing] tests/conformance_corner_cases.rs::empty_step_is_rejected_before_mutation`
* Empty body after integrated-cut removal is not listed — `[planned] tests/conformance_corner_cases.rs::fully_integrated_candidate_is_not_listed`
* Incomparable tips are never silently linearized — `[existing] tests/integration_test.rs::test_discover_forked`
* Overlapping managed staircases retain separate lineage and ownership — `[planned] tests/conformance_corner_cases.rs::overlapping_lineages_require_explicit_coordination`
* Ambiguous merge-parent path requires policy — `[planned] tests/conformance_corner_cases.rs::merge_commit_requires_mainline_policy`
* Stale lower rewrite is managed-only state — `[planned] tests/conformance_corner_cases.rs::stale_relationship_requires_management_or_explicit_restack`
* Several refs at one cut are aliases, not shared ownership — `[planned] tests/conformance_corner_cases.rs::several_refs_at_cut_preserve_separate_ownership`
* Conflicting persistent override is rejected — `[planned] tests/conformance_corner_cases.rs::override_conflicting_with_ancestry_is_rejected`
* One observed ref in two lineages does not share ownership — `[planned] tests/conformance_corner_cases.rs::shared_override_observation_keeps_exclusive_ownership`
* Existing snapshot tag requires explicit leased replacement — `[planned] tests/conformance_corner_cases.rs::snapshot_tag_replacement_is_explicit_and_leased`

### Branches, worktrees, and drafts

* Sequential-looking name without complete layout grants no ownership — `[planned] tests/conformance_corner_cases.rs::sequential_suffix_alone_grants_no_ownership`
* Unowned destination branch blocks rewrite — `[planned] tests/conformance_corner_cases.rs::unowned_destination_branch_blocks_before_rewrite`
* Sequential rename cycles publish transactionally — `[planned] tests/conformance_corner_cases.rs::sequential_rename_cycle_uses_transaction`
* Checked-out primary branch follows its conceptual step or blocks — `[planned] tests/conformance_corner_cases.rs::checked_out_branch_follows_step_or_blocks`
* Unowned non-primary alias is unchanged — `[planned] tests/conformance_corner_cases.rs::unowned_alias_at_moved_cut_is_unchanged`
* Branch configuration follows conceptual step — `[planned] tests/conformance_corner_cases.rs::branch_configuration_follows_step_on_renumber`
* Dirty worktree is preserved, snapshotted, or blocks — `[existing] tests/regression_safety.rs::test_reorder_data_loss_on_dirty_workdir`
* Partial staging commits the exact index — `[planned] tests/conformance_corner_cases.rs::partial_staging_commits_index_and_preserves_worktree`
* Unmerged index is owned by active operation — `[planned] tests/conformance_corner_cases.rs::unmerged_index_blocks_ordinary_materialization`
* Ignored files are excluded by default — `[planned] tests/conformance_corner_cases.rs::ignored_files_do_not_make_default_draft_dirty`
* Untracked files are separately reported and explicitly included — `[planned] tests/conformance_corner_cases.rs::untracked_files_require_explicit_inclusion`
* Sparse checkout does not hide tree changes — `[planned] tests/conformance_corner_cases.rs::sparse_checkout_rewrite_uses_trees_not_visible_files`
* Filters and line endings do not replace index authority — `[planned] tests/conformance_corner_cases.rs::filtered_worktree_uses_index_as_staged_authority`
* Dirty submodule state remains separate — `[planned] tests/conformance_corner_cases.rs::dirty_submodule_is_reported_without_nested_capture`
* External Git or `repo sync` operation remains external — `[planned] tests/conformance_corner_cases.rs::external_git_operation_is_reported_and_not_adopted`

### Transactions, records, and lifecycle

* Interrupted Staircase operation has deterministic continuation and abort — `[existing] tests/local_journeys_test.rs::conflict_pause_has_recovery_refs_and_continue_is_deterministic`
* Death after object writes leaves old authoritative refs — `[planned] tests/conformance_corner_cases.rs::crash_before_ref_transaction_leaves_old_refs`
* Death after partial ref movement is recovered explicitly — `[planned] tests/conformance_corner_cases.rs::partial_ref_publication_is_transactional_or_recoverable`
* Concurrent metadata and structure updates fail full-record CAS — `[existing] tests/local_journeys_test.rs::metadata_editor_rejects_concurrent_full_record_change`
* Public and internal active refs disagreeing is an integrity failure — `[planned] tests/conformance_corner_cases.rs::disagreeing_public_and_internal_records_fail_integrity`
* Archived metadata edit remains archived — `[planned] tests/conformance_corner_cases.rs::archived_metadata_edit_updates_only_archive_record`
* Archive leaves unowned aliases and warns — `[planned] tests/conformance_corner_cases.rs::archive_leaves_unowned_aliases`
* Archive refuses active Git or Staircase operation — `[planned] tests/conformance_corner_cases.rs::archive_refuses_active_operation`
* Archive requires dirty draft disposition — `[planned] tests/conformance_corner_cases.rs::archive_requires_dirty_draft_disposition`
* Occupied unarchive destination never overwrites — `[planned] tests/conformance_corner_cases.rs::unarchive_collision_refuses_or_restores_branchlessly`
* Archived name reservation governs reuse — `[planned] tests/conformance_corner_cases.rs::archived_name_is_reserved_until_explicit_release`
* Local archive leaves remote review state unchanged — `[planned] tests/conformance_corner_cases.rs::archive_is_remote_noop_by_default`

### Providers, landing, workspace, and output

* Network drop during upload requires reconciliation — `[planned] tests/conformance_corner_cases.rs::upload_unknown_requires_identity_reconciliation`
* Checks for an older revision are stale — `[planned] tests/conformance_corner_cases.rs::older_provider_checks_do_not_verify_current_revision`
* Review identity survives rebase while exact evidence stales — `[planned] tests/conformance_corner_cases.rs::rebase_preserves_review_identity_and_stales_revision`
* Weak or unscoped review attachment is rejected or provisional — `[planned] tests/conformance_corner_cases.rs::weak_review_attachment_never_satisfies_policy`
* Detach is local-only by default — `[planned] tests/conformance_corner_cases.rs::review_detach_does_not_mutate_remote`
* Unsupported incremental provider topology is honest — `[planned] tests/conformance_corner_cases.rs::provider_topology_is_refused_or_labeled_accurately`
* Inseparable create/upload reports both effects — `[planned] tests/conformance_corner_cases.rs::combined_creation_reports_identity_and_publication`
* Staircase transport and review publication remain separate — `[existing] tests/local_journeys_test.rs::staircase_transport_uses_distinct_explicit_namespaces`
* Landing method mismatch reconciles the actual graph — `[planned] tests/conformance_corner_cases.rs::landing_method_mismatch_reconciles_actual_destination`
* Lower-prefix landing advances and restacks remaining steps — `[existing] tests/local_journeys_test.rs::partial_landing_keeps_surviving_ids_and_renumbers_layout`
* Uncertain landing does not restack or delete — `[planned] tests/conformance_corner_cases.rs::landing_unknown_preserves_plan_and_local_state`
* Moving integration target causes lease failure and replan — `[planned] tests/conformance_corner_cases.rs::integration_target_movement_requires_replan`
* Ambiguous bootstrap binds only unambiguous capabilities — `[planned] tests/conformance_corner_cases.rs::ambiguous_provider_evidence_preserves_safe_capabilities`
* Network- or repository-code-dependent passive probe is ineligible — `[planned] tests/conformance_corner_cases.rs::unsafe_passive_probe_is_rejected`
* Missing workspace or changed project mapping invalidates binding, not records — `[planned] tests/conformance_corner_cases.rs::workspace_mapping_change_retains_records_and_requires_resolution`
* Empty human list is exactly `No staircases.\n` — `[existing] tests/core_foundation_test.rs`
* Empty porcelain list is zero bytes — `[existing] tests/core_foundation_test.rs`
* Empty JSON list is exactly `[]\n` — `[existing] tests/core_foundation_test.rs`

## Core specification Appendix D

* D.1 empty repository view — `[existing] tests/core_foundation_test.rs`
* D.2 canonical duplicate collapse — `[existing] tests/core_foundation_test.rs`
* D.3 full-record concurrency rejection — `[existing] tests/local_journeys_test.rs::metadata_editor_rejects_concurrent_full_record_change`
* D.4 transactional split and branch renumber — `[existing] tests/local_journeys_test.rs::split_renumber_is_transactional_and_preserves_upper_step_id`
* D.5 draft preservation on abort — `[existing] tests/local_journeys_test.rs::conflict_abort_restores_index_worktree_and_deleted_untracked_bytes`
* D.6 archive porcelain invisibility — `[existing] tests/local_journeys_test.rs::archive_removes_active_names_and_owned_branches`
* D.7 provider revision freshness — `[existing] tests/provider_journeys_test.rs::gerrit_journey_6_verification_is_exact_revision_scoped`

## `repo` provider Appendix A

* A.2 first use in detached checkout with Gerrit hints — `[existing] tests/provider_journeys_test.rs::repo_journey_1_detached_checkout_composes_gerrit_hints_offline`
* A.3 moving manifest branch advances after checkout — `[existing] tests/provider_journeys_test.rs::repo_journey_2_moving_manifest_keeps_exact_checkout_evidence`
* A.4 resolve a `repo sync` conflict without surrendering ownership — `[existing] tests/provider_journeys_test.rs::repo_journey_3_external_sync_operation_remains_external`
* A.5 revision-locked manifest preserves pinned commit — `[existing] tests/provider_journeys_test.rs::repo_journey_4_revision_locked_manifest_preserves_pin`
* A.6 attached development branch is not the baseline — `[existing] tests/provider_journeys_test.rs::repo_journey_5_attached_branch_is_not_workspace_anchor`
* A.7 duplicate project checkouts remain distinct — `[existing] tests/provider_journeys_test.rs::repo_journey_6_duplicate_project_checkouts_have_distinct_identity`
* A.8 local manifest changes Gerrit destination — `[existing] tests/provider_journeys_test.rs::repo_journey_7_local_manifest_changes_destination`
* A.9 detached review checkout needs explicit integration context — `[existing] tests/provider_journeys_test.rs::repo_journey_8_detached_review_checkout_is_not_silently_baseline`
* A.10 unavailable `repo` degrades repository-local work — `[existing] tests/provider_journeys_test.rs::repo_journey_9_missing_repo_executable_degrades_without_failure`

## GitHub provider Appendix A

* A.2 publish a same-repository stacked pull-request chain — `[existing] tests/provider_journeys_test.rs::github_journey_1_same_repository_stacked_chain`
* A.3 reject an impossible fork stack and publish aggregate — `[existing] tests/provider_journeys_test.rs::github_journey_2_fork_stack_rejected_but_aggregate_allowed`
* A.4 reconcile an upload with unknown network result — `[existing] tests/provider_journeys_test.rs::github_journey_3_uncertain_branch_publication_is_journaled`
* A.5 squash-land the bottom pull request and repair chain — `[existing] tests/provider_journeys_test.rs::github_journey_4_squash_landing_requires_upper_repair`
* A.6 attach from detached `HEAD`, archive, and restore branchlessly — `[existing] tests/provider_journeys_test.rs::github_journey_5_attach_detach_and_archive_are_local`

## Gerrit provider Appendix A

* A.2.1 new review stack starting state — `[planned] tests/conformance_gerrit_journeys.rs::journey_1_discovers_new_review_stack`
* A.2.2 planning detects missing Change-Id without mutation — `[planned] tests/conformance_gerrit_journeys.rs::journey_1_plan_rejects_missing_change_id_without_mutation`
* A.2.3 normalization rewrites only required commit and descendants — `[planned] tests/conformance_gerrit_journeys.rs::journey_1_normalization_rewrites_required_suffix_only`
* A.2.4 creation records pending keys and upload confirms identities — `[existing] tests/provider_journeys_test.rs::gerrit_black_box_create_persists_pending_associations`
* A.3.3 rebase to manifest anchor with nonadjacent conflicts — `[existing] tests/provider_journeys_test.rs::gerrit_journey_2_rebase_stales_exact_revisions`
* A.3.4 `repo sync` conflict remains external — `[existing] tests/provider_journeys_test.rs::repo_journey_3_external_sync_operation_remains_external`
* A.4 split a reviewed subject without duplication — `[existing] tests/provider_journeys_test.rs::gerrit_journey_3_split_preserves_one_identity`
* A.5 reconcile a patch set uploaded outside Staircase — `[existing] tests/provider_journeys_test.rs::gerrit_journey_4_reconcile_external_patch_set`
* A.6 recover from unknown upload outcome — `[existing] tests/provider_journeys_test.rs::gerrit_journey_5_uncertain_upload_requires_reconciliation`
* A.7 verification applies only to exact patch-set revisions — `[existing] tests/provider_journeys_test.rs::gerrit_journey_6_verification_is_exact_revision_scoped`
* A.8 stepwise landing with Gerrit-created commit — `[existing] tests/provider_journeys_test.rs::gerrit_journey_7_stepwise_landing_submits_only_bottom`
* A.9 unrelated topic member blocks aggregate submission — `[existing] tests/provider_journeys_test.rs::gerrit_journey_8_aggregate_topic_rejects_unrelated_change`
* A.10 attach reviews from an existing Gerrit workflow — `[existing] tests/provider_journeys_test.rs::gerrit_journey_9_attach_existing_review_validates_route`
* A.11 archive and unarchive preserve associations offline — `[existing] tests/provider_journeys_test.rs::gerrit_journey_10_archive_preserves_state_without_transport`

## Gerrit provider Section 28 conformance scenarios

* 28.1 passive composition in a `repo` workspace — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_1_passive_repo_composition`
* 28.2 prepare and upload three reviews — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_2_prepare_and_upload_three_reviews`
* 28.3 rebase preserves review identity and stales evidence — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_3_rebase_preserves_identity_and_stales_evidence`
* 28.4 remote-newer patch set blocks upload — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_4_remote_newer_blocks_upload`
* 28.5 unknown upload outcome — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_5_unknown_outcome_is_not_retried`
* 28.6 split one reviewed step — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_6_split_has_unique_change_ids`
* 28.7 provider verification — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_7_provider_verification_is_exact`
* 28.8 stepwise landing — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_8_stepwise_landing`
* 28.9 whole-topic safety — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_9_whole_topic_safety`
* 28.10 archive is local — `[existing] tests/provider_journeys_test.rs::gerrit_conformance_28_10_archive_is_local_only`

## Stacksaw UI and adapter journeys

* Managed projection carries canonical revisions and steps — `[existing] stacksaw/crates/stacksaw-git/tests/canonical_projection.rs::managed_projection_carries_canonical_revisions_and_steps`
* Partial landing preserves lineage in projection — `[existing] stacksaw/crates/stacksaw-git/tests/canonical_projection.rs::partial_landing_preserves_lineage_and_remaining_decomposition`
* Family paths list in projection — `[existing] stacksaw/crates/stacksaw-git/tests/canonical_projection.rs::forked_discovery_lists_canonical_family_paths`
* Canonical split/join/archive/undo mutations — `[existing] stacksaw/crates/stacksaw-core/tests/canonical_mutations.rs`
* Archive queues canonical Run-tab command — `[existing] stacksaw/crates/stacksaw-ui/tests/render.rs::archiving_a_stack_queues_an_archive_run`
* Contextual show/adopt/verify/materialize commands — `[existing] stacksaw/crates/stacksaw-ui/tests/render.rs`
* Restack probe uses live parent tip — `[existing] stacksaw/crates/stacksaw-git/tests/rebase_probe.rs::amend_recovers_stale_children_and_flags_a_restack`
