# Addendum C: Sequential Primary-Branch Naming for Linear Staircases

## 1. Status and Scope

This addendum modifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**

It defines an optional, human-oriented naming layout for the local branches that materialize the cuts of a linear staircase.

For a staircase with branch-layout base `F`, the layout is:

```text
F-1
F-2
...
F-(N-1)
F
```

where:

* `F-1` names the bottom step, closest to the integration target.
* Numbered branches increase toward the tip.
* `F` names the final, child-most step.
* A one-step staircase uses only `F`.

This addendum defines:

* The distinction between step identity and branch-layout names.
* How the branch-layout base is selected and persisted.
* How primary step branches are assigned.
* How branch names change after split, join, reorder, insertion, deletion, and partial landing.
* How checked-out branches, branch configuration, reflogs, and unrelated refs are treated.
* How collisions are detected.
* How all affected refs are updated safely.
* When the layout may remain implicit and when it requires managed staircase state.

This naming layout applies only to **linear staircases**. It does not assign names to forked staircase families or other non-linear dependency structures.

---

## 2. Relationship to Existing Identities

### 2.1 Branch-layout names are not step identities

A name such as:

```text
feature-2
```

identifies the local branch currently assigned to the second staircase position.

It does not permanently identify the conceptual step occupying that position.

A structural operation may cause the same conceptual step to move from:

```text
feature-2
```

to:

```text
feature-3
```

while retaining the same stable step ID.

The naming model therefore distinguishes:

```text
step ID        Stable conceptual identity
step ordinal   Current position in the staircase
cut OID        Exact current cumulative commit
branch ref     Mutable local ref materializing the cut
layout name    Branch name derived from the current ordinal
```

The stable step-ID rules from Addendum B remain authoritative.

---

### 2.2 The layout names cuts through primary branches

Each step may have zero, one, or several refs pointing to its cut.

This addendum designates at most one local branch as the step’s **primary branch**.

Only primary branches participate in sequential renaming.

Other refs pointing to the same cut are aliases or independent refs. They are not automatically:

* Renamed.
* Deleted.
* Reassigned.
* Treated as owned by the staircase.

For a managed staircase, the mapping from step ID to primary branch ref is stored in the staircase descriptor.

For a conforming implicit staircase, the primary branches may be inferred from the complete sequential layout defined in this addendum.

---

### 2.3 The branch-layout base is not the staircase name

The branch-layout base `F` is independent of the canonical managed staircase name.

For example:

```text
staircase name:      authentication-redesign
branch-layout base:  users/auth
```

may produce:

```text
refs/staircases/authentication-redesign

refs/heads/users/auth-1
refs/heads/users/auth-2
refs/heads/users/auth
```

Renaming the managed staircase does not rename its primary branches unless branch-layout renaming is explicitly requested.

Renaming the branch-layout base does not change:

* Staircase lineage.
* Staircase revision history, except for the descriptor revision recording the new layout.
* Step IDs.
* Cut OIDs.
* The canonical staircase name.

---

## 3. Design Principles

### 3.1 The tip is bare

The final, child-most step receives the unsuffixed branch name:

```text
F
```

This branch ordinarily represents the complete staircase and is expected to be the most common workspace branch.

All lower steps receive numbered names.

---

### 3.2 Numbering begins at the integration boundary

The bottom-most step, closest to the integration boundary, receives:

```text
F-1
```

The next step receives:

```text
F-2
```

Numbering increases monotonically toward the tip.

---

### 3.3 Names describe current position

Sequential branch names describe the current shape of the staircase.

They are intentionally unstable under:

* Insertion.
* Split.
* Join.
* Drop.
* Reorder.
* Partial landing.
* Addition of a new tip.

Users and integrations that require stable identity must use step IDs rather than sequential branch names.

---

### 3.4 Rename only owned refs

The layout must never rename or delete a branch merely because its name resembles a sequential staircase branch.

A branch may be modified only when it is:

* Explicitly recorded as a primary branch of a managed staircase, or
* Unambiguously inferred as a primary branch of the selected implicit staircase under the recognition rules in this addendum.

All other refs are outside the mutation set.

---

### 3.5 Structural mutation and layout mutation form one logical operation

When a structural operation changes step positions, its branch-layout update is part of the same logical staircase operation.

The user must not be left with a successfully changed staircase whose owned branches silently retain an incorrect sequential layout.

The implementation must either:

* Complete the structural and branch-layout changes, or
* Abort before publication, or
* Leave a durable, explicitly reported interrupted operation that can be resumed or aborted.

---

### 3.6 Do not infer semantics from numeric suffixes alone

A branch ending in `-<digits>` is not necessarily a numbered staircase branch.

Examples include legitimate feature names such as:

```text
tls-1
api-v2-3
issue-1847
```

Therefore, the layout base must not be derived by unconditionally stripping a suffix matching:

```text
-[0-9]+$
```

Numeric stripping may be offered only as an explicit migration convenience. It is not the normative base-name algorithm.

---

## 4. Enabling the Sequential Layout

### 4.1 Layout profile

The sequential branch naming scheme is an optional layout profile.

A managed staircase may store:

```text
primary-branch-layout sequential-v1
branch-layout-base <F>
```

A staircase without this profile is not automatically renumbered by structural operations.

---

### 4.2 Explicit enablement

The layout may be enabled explicitly:

```console
git staircase layout set auth \
    --primary-branches=sequential \
    --base feature
```

This command:

1. Validates the requested base.
2. Determines the required final branch names.
3. Verifies ownership and collisions.
4. Assigns one primary branch to each step.
5. Renames or creates those branches transactionally.
6. Stores the layout policy and base.

---

### 4.3 Initial base selection

The branch-layout base `F` is selected using the following precedence:

1. An explicit `--base <F>` argument.
2. A previously stored branch-layout base.
3. For a recognized implicit sequential staircase, the exact name of its unsuffixed tip branch.
4. For a newly created staircase with a designated tip branch, the exact short name of that tip branch.
5. Otherwise, no base is inferred.

The exact tip branch name is used without removing a numeric suffix.

For example, if the designated tip branch is:

```text
refs/heads/feature-3
```

then the default base is:

```text
feature-3
```

and a three-step layout would be:

```text
feature-3-1
feature-3-2
feature-3
```

A user who intends `feature` as the base must state it explicitly:

```console
git staircase layout set auth \
    --primary-branches=sequential \
    --base feature
```

---

### 4.4 Optional suffix-stripping migration

An implementation may provide an explicit migration option:

```console
git staircase layout set auth \
    --infer-base=strip-numeric-suffix
```

This option may propose:

```text
feature-3      -> feature
feat/parser-12 -> feat/parser
```

Before applying the result, the tool must:

* Show or otherwise expose the proposed base.
* Validate all generated branch names.
* Check all destination collisions.
* Refuse ambiguous inference.
* Avoid changing unrelated refs.

Suffix stripping must never occur silently during ordinary discovery or mutation.

---

### 4.5 Base validation

For each possible staircase size, generated primary branches must be valid local branch refs.

At minimum, the implementation must validate:

```text
refs/heads/F
refs/heads/F-1
...
refs/heads/F-(N-1)
```

using Git refname validation.

If any generated refname is invalid, the operation must fail before modifying commits or refs.

The base must not be accepted if it produces:

* An invalid refname.
* A ref prefix conflict.
* A collision with an unowned branch.
* A collision with another reserved primary branch.
* A name outside `refs/heads/`.

---

## 5. Sequential Naming Function

Let a linear staircase contain (N) steps:

[
S_0, S_1, \dots, S_{N-1}
]

where:

* (S_0) is the bottom-most step.
* (S_{N-1}) is the tip step.
* Indices are zero-based internally.

Define the primary branch short name (B(i,N,F)) as:

[
B(i,N,F) =
\begin{cases}
F & \text{if } i=N-1 \
F\text{-}(i+1) & \text{if } 0 \leq i < N-1
\end{cases}
]

The corresponding full refname is:

```text
refs/heads/<B(i,N,F)>
```

---

### 5.1 One-step staircase

For (N=1):

```text
Step 0: F
```

Example:

```text
feature
```

---

### 5.2 Four-step staircase

For (N=4):

```text
Step 0: feature-1
Step 1: feature-2
Step 2: feature-3
Step 3: feature
```

---

### 5.3 Number parsing is contextual

A branch named:

```text
feature-3
```

is interpreted as ordinal 3 only when:

* The selected layout base is `feature`.
* The staircase contains at least four steps.
* The branch is the expected primary branch at index 2.
* Its cut matches the selected staircase structure.

The suffix alone does not establish membership or ordinal meaning.

---

## 6. Recognition of an Implicit Sequential Staircase

An implicit staircase conforms to this layout only when all of the following hold:

1. The staircase is linear and materialized.

2. It has a unique tip cut.

3. The tip cut has exactly one selected primary local branch named `F`.

4. Every lower cut has exactly one selected primary local branch named:

   ```text
   F-1, F-2, ..., F-(N-1)
   ```

5. Each expected branch points to the correct cut.

6. No two expected names resolve to different candidate staircases.

7. The base `F` is determined uniquely from the unsuffixed tip branch.

8. Any additional refs are treated as non-primary aliases and do not create a conflicting primary layout.

Such a staircase may be displayed as:

```text
feature  4 steps  sequential  (implicit)
```

Structural operations may preserve implicit status if:

* The current ownership mapping is unambiguous.
* The final staircase again conforms exactly to the sequential layout.
* No persistent overrides, stable branch ownership, or nonmaterialized state must be recorded.

If ownership or the intended base cannot be reconstructed unambiguously, enabling or preserving the layout requires adoption.

---

## 7. Primary-Branch Ownership

### 7.1 Managed ownership

For a managed staircase, each active step descriptor may contain:

```text
step-id <uuid>
primary-branch refs/heads/<name>
```

The staircase owns the right to update that ref only while the descriptor records it as the primary branch for that step.

Ownership does not imply ownership of:

* Tags.
* Remote-tracking refs.
* Other local branches at the same cut.
* Worktree-local refs.
* External review refs.
* Branches merely sharing a name prefix.

---

### 7.2 Implicit ownership

For a conforming implicit staircase, the operation may temporarily treat the exact refs:

```text
refs/heads/F-1
...
refs/heads/F-(N-1)
refs/heads/F
```

as the selected primary branches.

This inferred ownership exists only for the current command.

If the inference is ambiguous, the command must fail or adopt the staircase with an explicit mapping.

---

### 7.3 Ownership preconditions

Before mutation, every owned source branch must:

* Exist.
* Point to the expected cut OID.
* Still belong to the selected staircase revision.
* Match the expected old branch-layout mapping.

If any precondition fails, the operation must abort rather than renaming a branch that changed concurrently.

---

## 8. General Renumbering Rule

After any operation that changes the number or order of steps, the implementation must:

1. Compute the complete resulting ordered step list.
2. Compute the required primary branch name for every resulting step.
3. Map each surviving step ID to its required destination branch.
4. Determine which old primary branches are:

   * Unchanged.
   * Renamed.
   * Deleted.
   * Newly created.
5. Validate the complete destination layout.
6. Check collisions and checked-out branches.
7. Publish the resulting layout as one logical operation.

Renumbering is based on the final staircase shape, not a sequence of pairwise branch renames.

This avoids order-dependent behavior.

---

## 9. Split

### 9.1 Structural rule

Splitting step (S_i) at commit (C) replaces it with:

* A new lower child at index (i), ending at (C).
* The original upper child at index (i+1), ending at the original cut.

The step count changes from:

[
N \rightarrow N+1
]

Under Addendum B’s default step-identity rule:

* The upper child retains the original step ID.
* The new lower child receives a new step ID.

All steps previously above (S_i) shift upward by one position.

---

### 9.2 Branch-layout rule

After the split, every step receives the branch name determined by its new position.

The branch associated with the original step follows the original step ID to the upper child.

A new primary branch is created at the split cut.

---

### 9.3 Single-step example

Before:

```text
feature -> C_tip
```

Split at `C_split`:

```text
feature-1 -> C_split
feature   -> C_tip
```

The original step retains the bare `feature` branch because it remains the tip and retains the original terminal cut.

---

### 9.4 Middle-step example

Before:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

Split the step at index 1 at `C_split`:

```text
feature-1 -> C1
feature-2 -> C_split
feature-3 -> C2
feature   -> C3
```

The old conceptual step at `C2` retains its step ID but moves from:

```text
feature-2
```

to:

```text
feature-3
```

---

### 9.5 Split at an existing cut

A split point must be strictly inside the selected step.

It must not equal:

* The predecessor cut.
* The selected step’s current cut.
* A commit outside the step.
* A commit that would produce an empty child step.

A request that would create an empty step must fail.

---

## 10. Join

### 10.1 Structural rule

Joining adjacent steps (S_i) and (S_{i+1}) removes the boundary at the cut of (S_i).

The combined step ends at the former cut of (S_{i+1}).

The step count changes from:

[
N \rightarrow N-1
]

Under Addendum B’s default identity rule:

* The later step (S_{i+1}) survives.
* The earlier step (S_i) is retired.

---

### 10.2 Branch-layout rule

The primary branch at the removed lower cut is deleted.

The surviving later step is assigned the branch name corresponding to its new position.

All steps above the joined pair shift downward by one position.

Only the owned primary branch at the removed cut is deleted. Other refs pointing to that commit remain unchanged.

---

### 10.3 Join involving the tip

Before:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

Join steps at indices 1 and 2:

```text
feature-1 -> C1
feature   -> C3
```

The owned primary branch:

```text
feature-2
```

is deleted.

The tip branch remains:

```text
feature
```

---

### 10.4 Join below the tip

Before:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

Join steps at indices 0 and 1:

```text
feature-1 -> C2
feature   -> C3
```

The old branch:

```text
feature-1 -> C1
```

is deleted.

The surviving conceptual step formerly named:

```text
feature-2
```

is renamed to:

```text
feature-1
```

---

### 10.5 Nonadjacent join

A direct join operation may join only adjacent steps.

Joining nonadjacent steps would implicitly remove one or more intervening boundaries and must be expressed as:

* Several explicit joins, or
* A separate range-collapse operation with clearly defined identity and deletion semantics.

---

## 11. Reorder

### 11.1 Structural rule

A reorder changes the ordinal positions of existing conceptual steps.

Step IDs move with the conceptual steps.

---

### 11.2 Branch-layout rule

After a successful reorder:

* Each lower position receives its new numbered branch.
* The new tip receives the bare branch `F`.
* The former tip receives a numbered branch if it is no longer the tip.

Example:

Before:

```text
position 0: step A -> feature-1
position 1: step B -> feature-2
position 2: step C -> feature
```

Reorder to:

```text
A, C, B
```

After:

```text
position 0: step A -> feature-1
position 1: step C -> feature-2
position 2: step B -> feature
```

The bare branch name follows the tip position, not the former tip’s step identity.

---

### 11.3 Checked-out step preservation

Although the bare name follows the tip position, a worktree currently attached to an owned branch should continue to follow the same conceptual step when that step survives.

For example, if a worktree is attached to step C before the reorder, and step C moves from the tip to position 1, that worktree should become attached to:

```text
feature-2
```

It should not silently remain attached to `feature` and thereby switch to step B.

The worktree rules in Section 18 apply.

---

## 12. Append and Step Insertion

### 12.1 Append commits within the current tip

Adding commits to the existing tip step does not change step count or ordinal positions.

No primary branches are renamed.

The branch `F` moves to the new tip commit in the ordinary Git manner.

---

### 12.2 Append a new tip step

Adding a new step above the current tip changes:

[
N \rightarrow N+1
]

The old tip becomes the highest numbered lower step:

```text
F-N
```

The new tip receives:

```text
F
```

Example:

Before:

```text
feature-1
feature
```

After adding a new tip:

```text
feature-1
feature-2
feature
```

The previous tip’s conceptual step moves from `feature` to `feature-2`.

---

### 12.3 Insert a step at an arbitrary boundary

Inserting a new step at index (i) uses the same renumbering semantics as split:

* The new step occupies index (i).
* Existing steps at or above (i) shift upward.
* The final layout is recomputed globally.

---

## 13. Drop

Dropping step (S_i):

1. Retires its step ID.
2. Deletes its owned primary branch.
3. Restacks or otherwise rewrites surviving upper steps as required.
4. Shifts all surviving upper steps downward by one position.
5. Recomputes every resulting primary branch name.

Example:

Before:

```text
feature-1
feature-2
feature-3
feature
```

Drop the second step:

```text
feature-1
feature-2
feature
```

The resulting `feature-2` belongs to the conceptual step that previously occupied `feature-3`.

A drop that intentionally leaves descendants stale requires managed state under Addendum A. Sequential layout normalization must occur only after the surviving materialized order is known.

---

## 14. Partial Landing

When one or more bottom steps land and are removed from the active staircase, the remaining steps are renumbered from the new integration boundary.

Example:

Before landing:

```text
feature-1
feature-2
feature-3
feature
```

After the first two steps land:

```text
feature-1
feature
```

The conceptual step formerly named `feature-3` becomes `feature-1`.

The tip remains `feature`.

Discovery alone must never rename branches merely because commits appear to have landed.

Renumbering after partial landing occurs only when an explicit staircase operation:

* Confirms which steps landed.
* Advances the integration boundary.
* Updates the active step set.
* Publishes the new branch layout.

Preserving lineage across partial landing requires management as defined by Addendum A.

---

## 15. Operations That Do Not Renumber

The following operations do not change primary branch names when step count and ordering remain unchanged:

* Rebase of the complete staircase.
* Restack preserving step order.
* Amend within a step.
* Squash within a step.
* Commit-message edits.
* Verification.
* Review upload.
* Moving the target while preserving the same active step order.
* Moving commits between steps when the step set and order remain unchanged.

These operations may update the commits pointed to by the existing primary branches.

---

## 16. Renaming the Layout Base

The branch-layout base may be changed explicitly:

```console
git staircase layout rename auth \
    --base authentication
```

For a three-step staircase:

```text
feature-1       -> authentication-1
feature-2       -> authentication-2
feature         -> authentication
```

This is a complete layout replacement.

It must use the same collision, worktree, configuration, and transaction rules as structural renumbering.

Changing the canonical staircase name does not imply this operation.

---

## 17. Collision Detection

### 17.1 Complete destination planning

Before updating refs, the implementation must compute:

* The complete set of owned source refs.
* The complete set of required destination refs.
* The expected old OID of every source ref.
* The required new OID of every destination ref.
* Every source ref that will be deleted.
* Every destination ref that will be created or updated.

No ref mutation may begin before the complete mapping is valid.

---

### 17.2 Permutations within the owned source set

A destination name may currently be occupied by another owned source ref participating in the same renumbering operation.

For example:

```text
feature-2 -> feature-1
feature-3 -> feature-2
```

This is a valid permutation when performed in one reference transaction.

The implementation must not perform it as sequential high-level renames.

---

### 17.3 Unowned destination collision

If a required destination ref already exists and is not an owned source ref in the current operation, the operation must fail.

Example:

```text
required destination:
  refs/heads/feature-2

existing unowned branch:
  refs/heads/feature-2
```

The tool must not overwrite, repurpose, or delete that branch automatically, even if it points to the desired cut.

Possible explicit remedies include:

* Select a different base.
* Adopt the existing branch as the primary branch through an explicit operation.
* Remove or rename the colliding branch separately.
* Use an explicit force-and-lease takeover operation.

A generic `--force` must not bypass branch ownership checks.

---

### 17.4 Same-OID collisions

An unowned branch pointing to the same OID is still an independent ref.

OID equality does not grant permission to overwrite or adopt it.

---

### 17.5 Prefix conflicts

The operation must reject layouts in which one required refname conflicts with another refname as a path prefix.

For example, a repository cannot safely use both:

```text
refs/heads/feature
refs/heads/feature/substep
```

when the ref backend prohibits the prefix relationship.

All generated refs must be validated as a set, not only one at a time.

---

### 17.6 Concurrent modification

Each source and destination ref must be protected by an expected-old-value condition.

The transaction must fail if:

* A source ref moved.
* A destination ref appeared.
* A destination ref disappeared when an expected value was required.
* The selected staircase revision changed.
* The primary-branch ownership mapping changed.

The operation must then be retried from a newly resolved staircase state.

---

## 18. Checked-Out Branches and Worktrees

### 18.1 Worktree-aware planning

Before renaming or deleting any primary branch, the implementation must inspect all linked worktrees.

For each worktree attached to an affected primary branch, the implementation must determine:

* The conceptual step currently checked out.
* Whether that step survives the operation.
* The destination branch assigned to that step.
* Whether the worktree has uncommitted or conflicting state relevant to the operation.

---

### 18.2 Surviving steps

If the checked-out conceptual step survives, the worktree’s symbolic `HEAD` should follow that step to its new primary branch.

Example:

```text
before:
  checked-out step: B
  branch: feature-2

after reorder:
  step B becomes tip
  branch: feature
```

The worktree should become attached to:

```text
feature
```

---

### 18.3 Retired steps

If a checked-out step is retired by join or drop, the operation must define the successor explicitly.

Defaults are:

* **Join:** attach the worktree to the surviving joined step.
* **Drop:** refuse if the dropped step is checked out, unless the command explicitly specifies a surviving destination or detached-HEAD behavior.
* **Partial landing:** attach to the corresponding surviving step when one exists; otherwise require explicit handling.

The implementation must not silently attach a worktree to an unrelated step merely because that step inherits the old branch spelling.

---

### 18.4 Unsupported atomic worktree updates

If the implementation cannot safely update all affected worktree `HEAD` references as part of the logical operation, it must:

* Refuse the operation, or
* Use a durable operation journal with explicit resume and abort behavior.

It must not leave a worktree attached to a deleted or semantically reassigned branch without reporting an interrupted state.

---

## 19. Branch Configuration

### 19.1 Configuration follows the conceptual step

Git branch configuration commonly uses keys under:

```text
branch.<short-name>.*
```

Examples include:

```text
branch.<name>.remote
branch.<name>.merge
branch.<name>.pushRemote
branch.<name>.rebase
branch.<name>.description
```

When a primary branch is renamed, its branch-specific configuration should follow the conceptual step, not remain attached to the old ordinal name.

---

### 19.2 Configuration permutation

For a renumbering operation, the implementation must:

1. Read all affected branch configuration sections.
2. Associate each section with the current step ID.
3. Compute the destination short name for that step.
4. Validate that no unrelated destination section would be overwritten.
5. Rewrite the affected configuration as one planned permutation.

Configuration must not be moved through a sequence of naive pairwise section renames because names may overlap.

---

### 19.3 Configuration collision

If a destination branch configuration section exists for an unowned branch, the operation must fail unless an explicit merge or takeover policy is supplied.

The implementation must not merge arbitrary branch configuration automatically.

---

### 19.4 Configuration and ref atomicity

Git ref transactions and repository configuration updates do not necessarily share one physical transaction.

Therefore, the operation must provide logical transactional behavior through:

* Prevalidation.
* A repository-level staircase-operation lock.
* A durable operation journal.
* A staged configuration update.
* Resume and abort support.
* Explicit interrupted-state reporting after a crash or partial external failure.

The tool must not claim physical atomicity across refs, worktree `HEAD` state, and configuration when the underlying Git interfaces cannot provide it.

The required guarantee is:

> The operation either completes consistently or leaves sufficient durable information to recover deterministically.

---

## 20. Reflogs and Recovery

### 20.1 Reflog entries

Every created, updated, or deleted primary branch should receive a descriptive reflog reason where supported.

Examples:

```text
staircase: renumber after splitting step 2
staircase: renumber after joining steps 1 and 2
staircase: assign bare tip branch after reorder
staircase: rename branch layout from feature to authentication
```

---

### 20.2 Reflog continuity

An implementation should preserve branch reflog continuity across renames when the Git ref backend provides a supported mechanism.

Sequential layout semantics must not depend on reflog-file relocation.

The implementation must not directly manipulate presumed files under:

```text
.git/logs/refs/heads/
```

Recovery must primarily rely on:

* Managed staircase state refs.
* Descriptor history.
* Operation recovery refs.
* Transaction journals.
* Ordinary Git object retention.

---

### 20.3 Recovery refs

Before publishing a destructive branch-layout mutation, a managed staircase operation should retain the previous cuts under operation-scoped refs such as:

```text
refs/staircase-operations/<operation-id>/before/steps/<step-id>
```

These refs:

* Preserve object reachability.
* Support abort and repair.
* Are not public staircase names.
* Must be cleaned up after successful completion according to retention policy.

---

## 21. Ref Transaction Requirements

### 21.1 Single ref transaction

All ordinary branch-ref changes for one renumbering operation must be submitted as one reference transaction where supported.

The transaction should include:

* Deletion of obsolete owned primary branches.
* Creation of new primary branches.
* Updates of retained branch names to new cut OIDs.
* Updates of managed step refs.
* Update of the staircase descriptor ref.
* Update of the public staircase-name ref.
* Creation or cleanup of recovery refs as appropriate.

---

### 21.2 No temporary namespace is needed for an atomic permutation

When Git can update all affected refs in one transaction, the implementation should not first rename branches into a temporary namespace.

The reference transaction can directly express:

* Deletes.
* Creates.
* Updates.
* Expected old values.

A branch name occupied by another source ref in the same transaction is not a collision when the complete permutation is valid.

---

### 21.3 Temporary namespace fallback

A temporary namespace may be used only when the implementation cannot express the complete operation in one supported reference transaction.

The namespace must be operation-scoped, for example:

```text
refs/staircase-operations/<operation-id>/branches/<encoded-original-ref>
```

It must not use:

```text
refs/staircases-temp/
```

because that resembles a public staircase namespace and does not encode operation ownership clearly.

A temporary-namespace fallback:

* Is a multi-phase migration.
* Is not inherently atomic.
* Requires a durable operation journal.
* Requires a repository-level operation lock.
* Must support resume and abort.
* Must preserve the expected source OIDs.
* Must never expose temporary refs as ordinary primary branches.
* Must clearly report an interrupted operation.

If these guarantees cannot be provided, the implementation must refuse the operation rather than perform unsafe sequential renames.

---

## 22. Interaction with Implicit and Managed Staircases

### 22.1 Implicit preservation

A structural mutation may preserve implicit status when:

* The starting staircase conforms exactly to the sequential layout.
* Its primary branches are unambiguous.
* The base is the exact unsuffixed tip branch name.
* The operation completes in one invocation.
* The resulting staircase again conforms exactly.
* No stable ownership, interrupted state, or layout override must be retained.

---

### 22.2 Adoption triggers

The operation requires adoption when any of the following is true:

* The layout base cannot be inferred uniquely.
* Primary-branch ownership is ambiguous.
* A step has no materializing primary branch.
* A step has a metadata-only cut.
* The operation intentionally leaves the staircase stale or incomplete.
* Worktree or recovery state must persist across invocations.
* A nonconforming existing branch must be retained but excluded from the sequential layout.
* The layout policy must remain active for future mutations.
* Stable step-to-branch configuration must be retained.
* Partial landing must preserve lineage.
* The command must recover after an interrupted multi-phase rename.

Automatic adoption follows Addendum A.

---

## 23. Layout Status

A managed staircase using this profile has a branch-layout status independent of its structural status.

### 23.1 Clean layout

The layout is clean when every active step has the expected primary branch and every such branch points to the expected cut.

---

### 23.2 Dirty layout

The layout is dirty when the staircase remains structurally valid but one or more primary branch names do not match their required sequential names.

Examples include:

* A user manually renamed `feature-2`.
* A branch was deleted.
* A partial landing was recorded without completing branch normalization.
* An external tool changed the primary branch mapping.

A dirty layout does not necessarily make the staircase stale.

---

### 23.3 Blocked layout

The layout is blocked when normalization cannot proceed because of:

* An unowned destination collision.
* A checked-out retired step requiring explicit handling.
* Invalid generated refnames.
* Concurrent ref changes.
* Conflicting branch configuration.
* Unsupported worktree update semantics.

---

### 23.4 Inspection and normalization

Commands should include:

```console
git staircase layout show auth
git staircase layout check auth
git staircase layout normalize auth
```

Inspection commands must not mutate refs.

Normalization applies the complete final layout using the transaction rules in this addendum.

---

## 24. Command Examples

### 24.1 Enable the layout

```console
git staircase layout set auth \
    --primary-branches=sequential \
    --base feature
```

---

### 24.2 Preview renumbering

```console
git staircase split auth:2 --at <commit> --dry-run
```

Possible output:

```text
new step:
  <new-step-id> -> refs/heads/feature-2

branch changes:
  refs/heads/feature-2 -> refs/heads/feature-3
  refs/heads/feature   -> refs/heads/feature

unchanged:
  refs/heads/feature-1

collision check:
  clear

worktrees:
  main worktree follows <existing-step-id> to refs/heads/feature-3
```

---

### 24.3 Normalize after manual branch changes

```console
git staircase layout normalize auth
```

---

### 24.4 Change the layout base

```console
git staircase layout rename auth \
    --base auth-redesign
```

---

### 24.5 Disable automatic sequential renaming

```console
git staircase layout unset auth --primary-branches
```

Disabling the policy does not rename existing branches unless an explicit target layout is requested.

---

## 25. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 25.1 One primary branch per active step

Under the sequential layout, every active materialized step has exactly one owned primary local branch.

---

### 25.2 One bare tip

Exactly one primary branch uses the unsuffixed base `F`, and it belongs to the current tip step.

---

### 25.3 Contiguous numbering

For (N>1), the lower primary branches are exactly:

```text
F-1 through F-(N-1)
```

There are no gaps, duplicates, or zero-based names.

---

### 25.4 Names follow positions

Sequential names follow current ordinals.

Stable step IDs follow conceptual steps.

---

### 25.5 Only owned branches are mutated

No unowned branch is renamed, deleted, overwritten, or adopted merely because its name or OID resembles an expected layout branch.

---

### 25.6 Numeric suffixes are not stripped implicitly

The base is not inferred by blindly removing `-[0-9]+`.

---

### 25.7 Complete planning precedes mutation

The full post-operation branch map is computed and validated before any ref is changed.

---

### 25.8 Ref changes are lease-protected

Every affected source and destination ref is checked against its expected old state.

---

### 25.9 Ref permutations are transactional

Overlapping branch renames are expressed as one reference transaction, not as unsafe sequential renames.

---

### 25.10 Worktrees follow conceptual steps

A worktree must not silently switch conceptual steps merely because a branch spelling is reassigned.

---

### 25.11 Configuration follows conceptual steps

Branch-specific configuration moves with the stable step ID, subject to explicit collision handling.

---

### 25.12 Recovery is deterministic

When physical atomicity across all repository state is unavailable, an interrupted operation must be detectable, resumable, or abortable.

---

## 26. Summary

The sequential primary-branch layout for a linear staircase with base `F` is:

```text
F-1, F-2, ..., F-(N-1), F
```

The bottom step receives `F-1`.

The tip receives `F`.

These names communicate current order, not stable identity.

The stable identity of a step remains its step ID. Its exact current materialization remains its cut OID.

Every shape-changing operation recomputes the complete final branch layout. It renames only owned primary branches, rejects unowned collisions, preserves worktree attachment by conceptual step, and publishes the ref permutation through a lease-protected transaction.

The governing rule is:

> Step IDs follow the work. Sequential branch names follow the shape.