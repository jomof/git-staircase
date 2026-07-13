# Addendum H: Worktree Drafts, Index State, and Materialization

## 1. Status and Scope

This addendum modifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**
5. **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**
6. Applicable review and verification provider addenda

It defines how Staircase interacts with:

* Staged changes.
* Unstaged changes.
* Untracked files.
* Ignored files.
* Partially staged files.
* Unmerged index entries.
* Multiple Git worktrees.
* Detached worktrees.
* Draft verification.
* Materialization of staged changes into staircase commits and steps.
* Preservation of local changes during staircase rewrites.

This addendum introduces the concept of a **worktree draft**.

A worktree draft is first-class operational state, but it is not part of the committed staircase body until it is materialized as one or more commits.

---

## 2. Design Conclusion

The Staircase model distinguishes three layers:

```text
Committed staircase
        ↓
Index
        ↓
Worktree
```

These layers answer different questions.

### Committed staircase

The committed staircase represents durable, transportable, reviewable Git history.

It is identified through:

* Commit OIDs.
* Staircase revision OIDs.
* Stable staircase lineage.
* Stable managed step IDs.

### Index

The index represents a proposed next Git tree relative to the current `HEAD`.

It is the natural candidate for the next commit.

When fully resolved, it may be represented exactly by a Git tree OID.

### Worktree

The worktree contains additional filesystem state not represented by the index.

It may include:

* Unstaged tracked changes.
* Untracked files.
* Ignored files.
* Dirty submodules.
* Files transformed through Git filters.
* Sparse-checkout state.
* Filesystem-only metadata.

The worktree cannot always be represented as one canonical Git tree without an explicit inclusion and normalization policy.

The governing distinction is:

> Committed steps are staircase history. Staged and unstaged changes are worktree-scoped draft overlays that may later be materialized into staircase history.

---

## 3. Terminology

### 3.1 Worktree

A **worktree** is one Git working-tree instance, including its:

* `HEAD`.
* Index.
* Checked-out filesystem.
* Worktree-specific Git state.
* Active Git operation state.

Linked Git worktrees have independent `HEAD` and index state.

A worktree draft therefore belongs to one worktree, not merely to the shared repository.

---

### 3.2 Draft basis

The **draft basis** is the exact commit OID against which the current index and worktree changes are interpreted.

Normally:

```text
draft basis = current worktree HEAD OID
```

The literal name `HEAD` is not persistent identity.

The full resolved OID is the basis.

---

### 3.3 Index state

The **index state** is the complete current Git index for one worktree.

When the index contains only stage-zero entries and can be written as a tree, it defines an exact candidate tree:

[
T_I
]

The staged delta is conceptually:

[
\Delta_S = \operatorname{diff}(T_H,T_I)
]

where (T_H) is the tree of the draft-basis commit.

The index also contains operational metadata that is not part of the candidate tree, including flags and conflict stages.

---

### 3.4 Worktree state

The **worktree state** is the current checked-out filesystem interpreted through Git’s normal index, attributes, filters, and repository configuration.

The unstaged tracked delta is conceptually:

[
\Delta_U = \operatorname{diff}(I,W)
]

where (I) is the index state and (W) is the tracked worktree state.

The unstaged delta does not include untracked files unless explicitly stated.

---

### 3.5 Untracked set

The **untracked set** contains files present in the worktree that do not have corresponding tracked index entries.

Untracked files are not staged changes and are not ordinary unstaged tracked changes.

They are reported separately.

---

### 3.6 Ignored set

The **ignored set** contains untracked files excluded by active ignore rules.

Ignored files are excluded from Staircase draft operations by default.

---

### 3.7 Worktree draft

A **worktree draft** is the tuple:

[
D_W =
(B,I,W,U,G,A)
]

where:

* (B) is the exact draft-basis commit.
* (I) is the index state.
* (W) is the tracked worktree state.
* (U) is the untracked set.
* (G) is the ignored set.
* (A) is optional staircase attachment and intent.

A live worktree draft is mutable and ephemeral.

It is not a Git commit, staircase step, or staircase revision.

---

### 3.8 Draft attachment

A **draft attachment** records that a worktree draft is intended to affect a particular staircase or step.

An attachment contains:

```text
staircase lineage or structural selector
selected step ID or current ordinal
attachment mode
expected basis OID
worktree identity
```

Attachment does not change the content of the draft.

---

### 3.9 Draft intent

A draft may have one of the following intents:

```text
unassigned
extend-step
new-step
rewrite-step
```

#### Unassigned

The worktree changes have no recorded staircase intent.

#### Extend-step

Materialization adds one or more commits to the selected existing step and advances that step’s cut.

#### New-step

Materialization creates a new step above the selected predecessor, normally above the current tip.

#### Rewrite-step

Materialization folds or otherwise rewrites the selected step rather than merely appending another commit.

Rewrite semantics must be specified by the materializing command, such as amend, fixup, or squash.

---

### 3.10 Draft snapshot

A **draft snapshot** is a durable recovery representation created explicitly from a live draft.

Unlike a live draft, a snapshot may have a persistent identifier.

A snapshot must record staged and unstaged state separately.

A snapshot is not automatically part of staircase history.

---

## 4. First-Classness

### 4.1 First-class operational state

Worktree drafts are first-class for:

* Status.
* Inspection.
* Diffing.
* Verification.
* Attachment.
* Preservation during structural operations.
* Materialization into commits.
* Recovery after interrupted operations.

Commands should not behave as though a dirty index or worktree is merely an exceptional nuisance.

---

### 4.2 Not first-class staircase membership

A live draft is not included in:

* The staircase body.
* A committed step.
* The staircase top.
* A staircase revision OID.
* A review-provider upload set.
* A commit-based outcome identity.

It becomes part of the staircase only after materialization into Git commits and publication of the resulting staircase revision.

---

### 4.3 No implicit commit identity

A draft has no commit OID.

A worktree filename or patch fingerprint must not be presented as though it were equivalent to a Git object identity.

---

### 4.4 Exact index-tree identity

When the index is fully resolved, the exact staged candidate tree may have a Git tree OID.

This OID identifies the candidate tree content.

It does not identify:

* A future commit.
* A staircase step.
* Commit metadata.
* Parentage.
* Author or committer.
* Commit message.
* Unstaged changes.
* Untracked files.

---

## 5. Automatic Draft Attachment

### 5.1 Exact-basis requirement

A draft may be automatically attached to a staircase step only when the worktree’s exact `HEAD` OID equals the selected step’s current cut OID.

For example:

```text
worktree HEAD: C3
staircase tip cut: C3
```

permits automatic attachment to the tip.

OID equality is required.

Similar branch names or equivalent patches are insufficient.

---

### 5.2 Primary-branch attachment

If the current worktree is attached to a primary staircase branch and:

```text
branch tip OID = selected cut OID = worktree HEAD OID
```

the draft may be inferred to belong to that step.

---

### 5.3 Detached attachment

A detached worktree at a staircase cut may also be attached by exact OID.

Detached state alone is not a barrier.

However, when the same OID is both:

* A possible workspace integration anchor, and
* A staircase cut,

the attachment must be selected explicitly or established by stronger staircase context.

---

### 5.4 No attachment from ancestry alone

A draft must not be automatically attached merely because its basis:

* Is a descendant of a staircase cut.
* Is an ancestor of a staircase cut.
* Has a nearby branch name.
* Contains a similar patch.
* Is reachable from the staircase tip.

Automatic attachment requires exact basis correspondence.

---

### 5.5 Several matching steps

Several staircases or steps may share the same cut OID.

If more than one materially distinct attachment is possible, the draft remains unassigned until the user selects one.

---

## 6. Persistent Attachment and Adoption

### 6.1 Invocation-local attachment

A user may select draft intent for one command without persistent attachment:

```console
git staircase draft show auth
git staircase draft materialize auth --new-step
```

Invocation-local selection does not inherently require adoption if the resulting staircase remains implicit and discoverable.

---

### 6.2 Persistent attachment

A persistent attachment may be created with:

```console
git staircase draft attach auth --mode=new-step
```

A persistent attachment records intent across commands.

Because it refers to stable staircase and step identity, it requires:

* A managed staircase.
* Stable step IDs when attached to an existing step.
* Worktree-scoped state.

Attaching persistently to an implicit staircase automatically adopts it.

---

### 6.3 Basis drift

A persistent attachment contains an expected basis OID.

If `HEAD` changes independently, the attachment becomes **detached from basis**.

The tool must not silently reinterpret the existing draft against the new `HEAD`.

Possible status:

```text
draft attachment: stale
expected basis: C3
current HEAD: D1
```

The user must:

* Reattach.
* Rebase the draft explicitly.
* Restore the expected basis.
* Detach the draft intent.

---

### 6.4 Worktree scope

A persistent draft attachment applies only to the worktree in which it was created.

It must not automatically attach drafts in another linked worktree.

---

## 7. Draft State Classification

A worktree draft may be classified as follows.

### 7.1 Clean

```text
index tree = HEAD tree
tracked worktree = index
no relevant untracked files
no unmerged entries
```

Ignored files do not make a worktree draft dirty unless explicitly included by policy.

---

### 7.2 Staged only

The index differs from `HEAD`, but tracked worktree content matches the index.

---

### 7.3 Unstaged only

The index tree matches `HEAD`, but tracked worktree content differs from the index.

---

### 7.4 Partially staged

Both of the following are nonempty:

```text
HEAD → index
index → worktree
```

This may include different hunks of the same file.

The tool must preserve this distinction exactly.

---

### 7.5 Untracked

One or more untracked files exist.

This state is reported separately from staged and unstaged tracked content.

---

### 7.6 Conflicted

The index contains unmerged entries at stages 1, 2, or 3.

A conflicted index does not define one candidate tree.

---

### 7.7 Transient-operation draft

The worktree participates in an active Git operation such as:

* Merge.
* Rebase.
* Cherry-pick.
* Revert.
* Bisect.
* Sequencer replay.
* An interrupted Staircase operation.

This state belongs first to the active operation.

It must not be attached automatically as an ordinary staircase draft.

---

### 7.8 Submodule-dirty

At least one tracked submodule has local state not represented solely by the superproject’s index entry.

Dirty submodule state is nested repository state.

It is not automatically captured by the containing repository’s draft.

---

## 8. Status and Inspection

### 8.1 Staircase status

`git staircase status` should report current-worktree draft state when relevant.

Example:

```text
auth
  state: clean
  steps: 3

current worktree draft:
  attached to: auth step 3
  intent: extend-step
  basis: c05a881
  staged: 4 paths
  unstaged: 2 paths
  untracked: 1 path
  conflicts: none
```

---

### 8.2 Listing

`git staircase list` may mark staircases with attached drafts:

```text
auth       3 steps  clean  draft
payments   2 steps  clean
```

The marker refers only to the current worktree unless `--all-worktrees` is requested.

A draft marker must not imply that uncommitted content is part of the staircase revision.

---

### 8.3 Draft commands

Recommended commands include:

```console
git staircase draft status
git staircase draft show
git staircase draft diff --staged
git staircase draft diff --unstaged
git staircase draft diff --combined
git staircase draft attach auth --mode=extend-step
git staircase draft detach
git staircase draft snapshot
git staircase draft restore <snapshot>
git staircase draft materialize
```

Ordinary Git commands such as `git add`, `git restore`, and `git reset` remain the canonical low-level tools for editing index state.

Staircase need not replace Git staging porcelain.

---

## 9. Diff Semantics

### 9.1 Staged diff

The staged diff is computed from:

```text
draft basis commit tree
    → current index
```

It must use the exact index, including partial staging.

---

### 9.2 Unstaged diff

The unstaged diff is computed from:

```text
current index
    → tracked worktree state
```

It excludes untracked files unless explicitly requested.

---

### 9.3 Combined working diff

The combined tracked diff is computed from:

```text
draft basis commit tree
    → tracked worktree state
```

This view does not preserve which changes are staged.

It is therefore unsuitable as the sole input to a command that promises to preserve staging boundaries.

---

### 9.4 Untracked files

Untracked content must be shown through an explicit category or option.

Example:

```console
git staircase draft diff --untracked
```

The command must not silently treat every untracked file as intended review content.

---

### 9.5 Ignored files

Ignored files are excluded unless explicitly requested:

```console
git staircase draft diff --ignored
```

Inspection does not imply later inclusion in a snapshot or commit.

---

## 10. Materialization

### 10.1 Definition

**Materialization** converts staged draft state into one or more commits and updates the staircase model accordingly.

By default, only the current index is materialized.

Unstaged and untracked content remain in the worktree.

---

### 10.2 Index as authoritative input

Materialization must commit the exact index state.

It must not reconstruct staged content by reading entire worktree files.

This preserves:

* Partial staging.
* File-mode changes.
* Deletions.
* Renames as represented by content and tree state.
* Staged submodule gitlink updates.
* Attribute and clean-filter behavior already reflected in the index.

---

### 10.3 Commit metadata

Before materialization, the command must obtain or derive:

* Parent commit.
* Tree OID.
* Commit message.
* Author information.
* Committer information.
* Optional signing policy.
* Provider-required commit trailers.

These values affect the resulting commit OID.

A tree OID alone is insufficient.

---

### 10.4 Extend an existing step

For `extend-step` intent:

1. The selected step’s cut must equal the draft basis.
2. The staged index becomes a new commit above that cut.
3. The selected step’s cut advances to the new commit.
4. Upper dependent steps, if any, must be restacked.
5. The unstaged worktree delta remains unstaged relative to the new index and `HEAD` as closely as ordinary Git commit semantics permit.
6. A new staircase revision is published.

If the selected step is not the tip, the operation must either:

* Restack all upper steps before completion, or
* Adopt the staircase before leaving those steps stale.

---

### 10.5 Create a new step

For `new-step` intent:

1. The selected predecessor must equal the draft basis.
2. The staged index becomes the first commit of a new step.
3. A new step ID is created when the staircase is managed.
4. A new cut is introduced at the resulting commit.
5. Sequential branch layout, when active, is recomputed under Addendum C.
6. The new step becomes the tip unless another explicit topology is requested.

For a one-step sequential staircase:

```text
before:
  feature -> C1
```

materializing a new step may produce:

```text
feature-1 -> C1
feature   -> C2
```

The current worktree should remain attached to the conceptual new tip when branch attachment is supported.

---

### 10.6 Rewrite an existing step

For `rewrite-step` intent, the command must explicitly state the rewrite operation.

Examples include:

```console
git staircase draft materialize --amend
git staircase draft materialize --fixup
git staircase draft materialize --fold-into auth:2
```

The command must define:

* Which commit or commits are replaced.
* Which step ID survives.
* How commit messages are formed.
* How upper steps are replayed.
* How review identities are preserved or invalidated.
* How the current worktree’s unstaged content is retained.

A generic `rewrite-step` operation without a concrete rewrite policy is invalid.

---

### 10.7 No staged changes

Materialization with an unchanged index must fail unless the requested operation explicitly permits an empty commit.

An empty commit must be opt-in.

---

### 10.8 Conflicted index

An index containing unmerged entries cannot be materialized as an ordinary commit.

The command must fail until the conflicts are resolved.

---

## 11. Unstaged and Untracked Materialization

### 11.1 No implicit staging

Staircase must not silently stage unstaged or untracked changes merely because a materialization command is run.

Default behavior follows:

```text
commit the index
leave other worktree changes uncommitted
```

---

### 11.2 Explicit combined materialization

A command may explicitly request a temporary combined candidate:

```console
git staircase draft materialize --all-tracked
```

This command must:

* Use a temporary alternate index.
* Stage tracked worktree changes into that alternate index.
* Leave the user’s real index unchanged until publication succeeds.
* Clearly define treatment of deletions and intent-to-add entries.
* Preserve the user’s original staged/unstaged split if the operation fails.

It must not be described as equivalent to ordinary staged-only materialization.

---

### 11.3 Untracked inclusion

Untracked files are included only through an explicit option or path selection:

```console
git staircase draft materialize --include-untracked
git staircase draft materialize -- path/to/new-file
```

The command must still honor explicit exclusions and safety checks.

---

### 11.4 Ignored inclusion

Ignored files require a stronger explicit option:

```console
git staircase draft materialize --include-ignored
```

A generic `--all` must not silently include ignored content.

---

## 12. Draft Verification

### 12.1 Verification subjects

Verification evidence must identify its subject type.

Valid draft subjects include:

```text
draft-index
draft-tracked-worktree
draft-snapshot
```

These are distinct from:

```text
commit
staircase-revision
review-patch-set
test-merge
merge-group
```

---

### 12.2 Index verification

When the index defines a candidate tree, verification may run against a temporary checkout of that tree.

Evidence records:

```text
draft basis OID
index tree OID
verification profile
environment identity
result
```

This evidence does not verify unstaged changes.

---

### 12.3 Working-draft verification

Verification of combined working state must record:

* Draft basis OID.
* Exact inclusion policy.
* Tracked-worktree snapshot identity.
* Included untracked paths.
* Excluded ignored paths.
* Repository attributes and filter context when relevant.
* Verification profile.
* Environment identity.

---

### 12.4 Promotion of evidence

Draft verification does not automatically become commit verification.

It may be promoted only when the materialized commit is proven to contain the exact verified tree under the same relevant verification policy.

Even then, commit metadata or provider policy may require separate verification.

---

### 12.5 Review providers

Review providers upload commits or provider-native review revisions.

They do not upload a live worktree draft by default.

A review-plan command may preview draft effects, but remote review creation requires materialization unless a provider explicitly defines a separate draft-snapshot protocol.

---

## 13. Draft Snapshots

### 13.1 Purpose

A draft snapshot provides durable recovery for:

* Long-running staircase rewrites.
* Worktree switching.
* Interrupted materialization.
* User-requested checkpoints.
* Preservation before destructive operations.

---

### 13.2 Required separation

A snapshot must preserve separately:

```text
basis commit
index candidate state
unstaged tracked overlay
selected untracked content
selected ignored content
submodule policy
draft attachment and intent
```

A snapshot that collapses staged and unstaged changes into one patch is not lossless.

---

### 13.3 Snapshot identity

A snapshot may be assigned:

* A stable snapshot UUID.
* Immutable Git object OIDs for captured trees and blobs.
* An operation-scoped ref.

A live draft does not acquire that identity merely because a snapshot exists.

---

### 13.4 No ordinary stash assumption

An implementation may use Git stash machinery internally only if it can preserve the required semantics.

It must not assume that one ordinary stash entry always preserves:

* Partial staging.
* Untracked policy.
* Ignored policy.
* Dirty submodule state.
* Worktree attachment.
* Staircase intent.
* Exact restoration behavior.

---

### 13.5 Restore collision

If restoration would overwrite changed or newly created files:

* Restoration must stop.
* Captured objects and recovery refs must remain available.
* The tool must report the conflicting paths.
* It must not discard either version silently.

---

## 14. Structural Operations with a Dirty Worktree

### 14.1 Default rule

A staircase operation that would modify any of the following must inspect the current worktree draft:

```text
current HEAD
current branch ref
current index
checked-out files
commits underlying the attached draft
```

If the operation cannot prove that the draft will remain valid and untouched, it must refuse by default.

---

### 14.2 Read-only operations

Read-only commands may operate with any draft state.

They must not refresh, rewrite, or normalize the index.

---

### 14.3 Ref-only operations

An operation affecting only refs not attached to the current worktree may proceed when it cannot alter the interpretation or safety of the current draft.

For example, renaming staircase branches in another detached worktree may be possible.

Worktree and branch-attachment rules from Addendum C still apply.

---

### 14.4 Rewrite operations

Rebase, restack, reorder, split with history rewriting, join with history rewriting, and drop commonly affect the basis of a draft.

Their default is:

```text
refuse when the affected worktree has staged or unstaged changes
```

unless an explicit draft-preservation mode is used.

---

### 14.5 Preserve-draft mode

A command may offer:

```console
--preserve-draft
```

This mode must:

1. Capture the exact staged state.
2. Capture the unstaged tracked overlay.
3. Capture only explicitly selected untracked content.
4. Refuse unsupported dirty submodule state unless recursive preservation is requested.
5. Record an operation journal.
6. Perform the staircase operation.
7. Reapply the draft to the corresponding surviving conceptual step or basis.
8. Restore staged and unstaged separation.
9. Leave recovery state if exact restoration cannot complete.

The command must not claim successful preservation if the staged boundary was lost.

---

### 14.6 Automatic preservation

Automatic preservation without an explicit option is permitted only when:

* The operation is designed to own the current draft.
* Lossless capture is supported.
* No untracked or ignored overwrite risk exists.
* No dirty nested repository exists.
* The behavior is documented for that operation.

A generic mutation must not invent an implicit autostash policy.

---

## 15. Multiple Worktrees

### 15.1 Independent drafts

Each worktree has an independent:

* `HEAD`.
* Index.
* Worktree draft.
* Persistent attachment.
* Active operation state.

The shared repository may therefore have several simultaneous drafts.

---

### 15.2 Listing scope

By default, draft information is shown for the current worktree.

An explicit option may show all known worktrees:

```console
git staircase status --all-worktrees
```

Example:

```text
auth
  worktree /work/auth:
    attached to step 3
    staged: 2 paths

  worktree /work/auth-tests:
    attached to step 2
    unstaged: 1 path
```

---

### 15.3 Conflicting attachments

Two worktrees may draft changes against the same cut.

This is permitted.

They are independent competing drafts.

The tool must not merge them or assign one shared draft identity.

Materializing one may make the other attachment stale.

---

### 15.4 Branch constraints

Git may prevent the same local branch from being checked out in several worktrees.

Detached worktrees may still share the same basis OID.

Staircase attachment is determined from exact worktree state, not merely branch exclusivity.

---

## 16. Index Corner Cases

### 16.1 Partial staging

Partially staged files must retain their exact index/worktree split.

Commands must not use whole-file worktree content when committing the index.

---

### 16.2 Intent-to-add

Intent-to-add entries are reported separately.

They may affect diff presentation without contributing normal staged blob content.

Materialization must follow Git’s actual index-writing rules and fail if the index cannot produce the requested commit tree.

---

### 16.3 Unmerged index stages

Stages 1, 2, and 3 are conflict state, not three ordinary draft layers.

The draft is conflicted until stage-zero entries are restored.

---

### 16.4 Skip-worktree

Sparse-checkout and `skip-worktree` entries must be interpreted through Git’s index semantics.

A file absent from the worktree is not automatically a deletion.

Staircase must not implement draft discovery by blindly scanning the filesystem and comparing it with `HEAD`.

---

### 16.5 Assume-unchanged

`assume-unchanged` is an index optimization hint, not review intent.

It must not be used to decide staircase membership or draft identity.

---

### 16.6 File modes and symbolic links

Candidate trees must preserve Git-relevant file modes and symbolic-link representation.

Filesystem metadata not represented by Git is outside committed staircase identity unless a separate provider explicitly handles it.

---

### 16.7 Filters and line endings

Worktree content may differ from canonical Git blob content because of:

* Clean and smudge filters.
* End-of-line conversion.
* Working-tree encoding.
* Attributes.

Materialization and snapshotting must use Git’s normal conversion semantics rather than hashing raw worktree bytes and calling the result a Git tree.

The relevant repository configuration is part of the provenance of a worktree snapshot.

---

## 17. Submodules and Nested Repositories

### 17.1 Superproject state

The containing repository records a submodule through a gitlink entry.

A staged change to that gitlink may be part of the superproject index draft.

---

### 17.2 Dirty submodule worktree

Uncommitted changes inside the submodule belong to the submodule repository’s own worktree draft.

They are not captured by staging the superproject gitlink.

---

### 17.3 Default behavior

A dirty submodule blocks automatic lossless draft preservation for an operation affecting the containing worktree.

The user may:

* Materialize or snapshot the nested draft separately.
* Request an explicit recursive operation.
* Exclude the submodule when safe.
* Proceed with a documented nonrecursive policy.

---

### 17.4 Multi-repository workspace

A workspace-level review may eventually coordinate drafts across several repositories.

That is a higher-level aggregate.

One repository-local staircase must not absorb another repository’s live draft into its own body.

---

## 18. Active Git Operations

### 18.1 Operation ownership

During an active merge, rebase, cherry-pick, revert, or sequencer operation, the index and worktree primarily belong to that operation.

Staircase must not reinterpret them automatically as a normal draft.

---

### 18.2 Inspection

Read-only Staircase commands may report:

```text
draft state: controlled by active rebase
```

They may also show staircase relationships when this can be done without disturbing the active operation.

---

### 18.3 Mutation

A Staircase mutation must:

* Refuse.
* Resume the owning Staircase operation.
* Or explicitly integrate with the active Git operation.

It must not begin an independent rewrite over an unresolved operation.

---

### 18.4 Conflict resolution

When a Staircase-managed restack stops for conflicts, the resulting index and worktree are operation-resolution state.

After resolution and continuation, remaining user draft state must be restored separately from conflict-resolution state.

The two must not be conflated.

---

## 19. Review Workflow Integration

### 19.1 Local draft before review

A common lifecycle is:

```text
edit worktree
stage selected changes
materialize commit
update staircase
verify committed revision
upload review revision
```

Staircase should support this lifecycle without requiring that all editing occur through Staircase-specific commands.

---

### 19.2 Draft-aware review planning

A provider may show how current staged changes would affect review topology:

```console
git staircase review plan auth --include-draft
```

The output must label the result as hypothetical.

Example:

```text
current committed staircase:
  3 steps

if staged draft is materialized as a new step:
  4 steps
  1 new review would be created
```

---

### 19.3 No live-draft upload

Unless a provider explicitly defines otherwise, upload commands operate only on immutable commits.

A command may combine materialization and upload as one high-level operation, but it must:

1. Materialize locally.
2. Publish a new staircase revision.
3. Construct an upload plan from exact commit OIDs.
4. Perform remote review operations.
5. Retain recovery state if remote outcome is uncertain.

---

### 19.4 Provider trailers and metadata

A review provider may require commit metadata such as trailers.

Such metadata is applied during materialization, not by mutating a live draft identity.

If adding provider metadata rewrites a commit, the resulting commit OID is the review revision.

---

## 20. Identity and Fingerprints

### 20.1 Live draft generation

A live draft may have a process-local or worktree-local generation token used to detect change during an operation.

This token is not a persistent public identity.

---

### 20.2 Index tree OID

When available:

```text
index tree OID
```

is an exact Git identity for staged tree content.

It must be labeled as a tree OID, not a commit OID.

---

### 20.3 Worktree fingerprint

An implementation may compute a worktree fingerprint for caching or change detection.

The fingerprint must be typed and scoped:

```text
worktree-draft:<algorithm>:<digest>
```

It must not be advertised as:

* Portable across repositories.
* Equivalent to a Git OID.
* Stable across attribute or filter changes.
* A staircase lineage identity.

---

### 20.4 Snapshot ID

A durable snapshot may have a stable snapshot UUID and immutable object references.

Snapshot identity remains distinct from the eventual commit created from it.

---

## 21. Adoption Rules

The following operations do not inherently require staircase adoption:

* Showing the current draft.
* Diffing staged or unstaged state.
* Verifying an index tree.
* Invocation-local materialization into a final discoverable implicit staircase.
* Temporary preservation within one successfully completed operation.

The following require management or automatic adoption:

* Persistent attachment to a staircase or stable step.
* Durable draft intent across commands.
* Leaving upper steps stale after materializing a lower-step change.
* Recording stable draft-to-review associations.
* Retaining a draft through an interrupted multi-command staircase operation.
* Associating snapshots with staircase lineage.
* Coordinating drafts across worktrees using stable step IDs.

---

## 22. Failure and Recovery Rules

### 22.1 No silent loss

No Staircase operation may silently discard:

* Staged changes.
* Unstaged tracked changes.
* Selected untracked content.
* Draft attachment.
* An operation recovery snapshot.

---

### 22.2 No silent staging-boundary collapse

If an operation cannot restore the exact staged/unstaged split, it must:

* Report incomplete restoration.
* Preserve recovery state.
* Avoid claiming full success.
* Provide the paths or hunks requiring reconciliation.

---

### 22.3 Concurrent worktree changes

Before publication, a draft-consuming operation must verify that:

* `HEAD` still equals the planned basis.
* The index still equals the planned index state.
* Relevant worktree content has not changed.
* The staircase revision has not changed.
* Owned refs retain expected old values.

If any condition fails, the operation must replan or abort.

---

### 22.4 Uncertain materialization

If a crash occurs after creating commits but before publishing staircase refs:

* The created commits remain immutable recovery objects.
* Operation refs or a journal must identify them.
* The command may resume or abort.
* The original draft must not be discarded until publication is confirmed.

---

## 23. Recommended Command Behavior

### 23.1 `git staircase list`

* Does not fail because the worktree is dirty.
* May mark relevant staircases as having drafts.
* Does not include draft content in staircase commit counts.

### 23.2 `git staircase show`

* Shows committed staircase structure.
* May include a separately labeled draft section.
* Does not render a draft as an ordinary committed step unless explicitly requested as a hypothetical view.

### 23.3 `git staircase verify`

By default verifies the committed staircase revision.

Explicit options select draft subjects:

```console
git staircase verify auth --draft=index
git staircase verify auth --draft=working
```

### 23.4 `git staircase upload`

By default ignores uncommitted changes and uploads the selected committed staircase revision.

If uncommitted changes are attached, it should warn that they are not included.

A combined materialize-and-upload mode must be explicit.

### 23.5 `git staircase rebase` and `restack`

Refuse when they would invalidate an unpreserved current draft.

Offer an explicit preservation mode.

### 23.6 `git staircase delete`

Deleting a staircase with attached drafts requires explicit disposition:

```text
detach draft
retarget draft
snapshot draft
abort deletion
```

The tool must not delete worktree content.

---

## 24. Example: Partially Staged New Step

Committed state:

```text
feature-1 -> C1
feature   -> C2
```

Current worktree:

```text
HEAD: C2
index:
  part of parser change
worktree:
  remaining parser change
  unrelated logging change
```

Attachment:

```text
staircase: feature
intent: new-step
basis: C2
```

Materialization creates:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

where `C3` contains only the staged parser changes.

The remaining parser and logging changes remain unstaged in the worktree.

---

## 25. Example: Draft Against a Lower Step

Committed staircase:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

A second worktree is attached to:

```text
feature-1 -> C1
```

It contains staged changes intended to extend step 1.

Materialization:

1. Creates `C1'`.
2. Advances step 1 to `C1'`.
3. Restacks steps 2 and 3 as `C2'` and `C3'`.
4. Updates stable step IDs.
5. Recomputes sequential branch refs.
6. Preserves any unstaged changes in the lower-step worktree.
7. Marks provider review revisions stale until republished.

---

## 26. Example: Detached Workspace Baseline

Current worktree:

```text
HEAD detached at workspace anchor B
```

Staircase branches:

```text
feature-1 -> C1
feature   -> C2
```

Edits made while detached at `B` are not automatically attached to `feature`.

The user may:

* Create a new staircase from the draft.
* Explicitly attach and rebase the draft onto a staircase cut.
* Materialize a separate commit.
* Discard or snapshot the draft.

The tool must not assume that every edit in a detached workspace belongs to the nearest staircase branch.

---

## 27. Example: Review Planning with Draft State

Committed staircase:

```text
3 steps
GitHub pull requests: 3
```

Current staged draft:

```text
intent: new-step
```

Command:

```console
git staircase review plan auth --include-draft
```

Possible output:

```text
committed review topology:
  3 pull requests

hypothetical after materializing staged draft:
  4 steps
  1 new pull request
  3 existing pull-request identities preserved

not included:
  2 unstaged paths
  1 untracked path
```

No branch is pushed and no pull request is created.

---

## 28. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 28.1 Drafts are worktree-scoped

A draft belongs to one worktree and one exact basis OID.

### 28.2 The index and worktree remain distinct

Staged and unstaged changes are never collapsed silently.

### 28.3 A draft is not a staircase step

Only commits participate in the committed staircase body.

### 28.4 The index is the default materialization source

Staircase does not silently stage the worktree.

### 28.5 Untracked and ignored files require explicit policy

They are not ordinary unstaged tracked content.

### 28.6 Exact basis equality governs automatic attachment

Ancestry or naming similarity is insufficient.

### 28.7 Conflicted indexes do not define candidate trees

They cannot be materialized as ordinary commits.

### 28.8 Review providers consume immutable revisions

Live worktree state is not uploaded by default.

### 28.9 Draft verification is typed

Verification of an index or worktree snapshot is not silently treated as commit verification.

### 28.10 Structural rewrites protect drafts

An affected dirty worktree is refused, preserved losslessly, or left with deterministic recovery state.

### 28.11 Worktree fingerprints are not Git OIDs

Derived draft fingerprints remain typed and scoped.

### 28.12 Nested repository state remains nested

Dirty submodule or child-repository state is not absorbed silently into the parent staircase.

### 28.13 Active Git operations retain ownership of their conflict state

Staircase does not reinterpret an in-progress rebase or merge as an ordinary draft.

### 28.14 No silent loss or reassignment

Draft content and intent are never discarded or attached to a different conceptual step without an explicit operation.

---

## 29. Summary

The Staircase model should treat uncommitted changes as a first-class **draft plane** above committed history:

```text
unstaged tracked changes
untracked and ignored content
            ↓
index candidate tree
            ↓
committed staircase cut
```

The index is the strongest draft-level object because it can often identify an exact candidate Git tree.

The worktree is a further mutable overlay whose meaning depends on explicit inclusion, filter, sparse-checkout, and nested-repository policies.

A draft may be attached to a staircase and given intent, but it does not become part of the staircase body until it is materialized as commits.

The governing rule is:

> The index proposes history; the worktree proposes changes to that proposal; only commits become staircase history.
