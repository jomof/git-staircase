# Addendum A: Implicit Staircases and Automatic Adoption

## 1. Status and Scope

This addendum modifies the conceptual specification titled **Git Staircase: Conceptual Specification**.

It introduces **implicit staircases** as operationally usable staircases rather than merely discovery results.

An implicit staircase:

* Is discovered from the current Git commit graph, refs, integration boundary, and discovery rules.
* Has not been recorded as a managed staircase.
* May be used by most `git staircase` commands.
* Remains implicit unless an operation requires durable information that cannot be reconstructed from lower-level Git concepts.
* Is automatically adopted when such durable information becomes necessary.

This addendum narrows the purpose of adoption.

Adoption is not required merely to list, inspect, verify, reshape, rebase, restack, or land a currently discoverable staircase.

Adoption is required only when the staircase must preserve intent or identity beyond what the current commit graph and refs can express.

---

## 2. Terminology Changes

### 2.1 Implicit staircase

An **implicit staircase** is a currently materialized staircase that can be derived unambiguously from:

* The Git commit graph.
* An integration boundary.
* A set of candidate refs or explicitly supplied tips.
* The active discovery rules.
* The current repository state.

An implicit staircase has no persistent staircase-specific metadata.

Conceptually, an implicit staircase is:

[
\mathcal{S}_{implicit} =
(D, U; a_1, a_2, \dots, a_k)
]

where:

* (D) is the discovery context.
* (U) is the resolved integration boundary.
* (a_1, \dots, a_k) are the discovered cuts.

The discovery context (D) may include:

* Which ref namespaces are searched.
* Branch-name normalization rules.
* Whether remote-tracking refs are considered.
* Whether annotated review metadata is considered.
* User-supplied discovery constraints.

The discovery context is necessary because the same repository graph may admit several useful staircase views.

---

### 2.2 Managed staircase

A **managed staircase** is a staircase with persistent staircase-specific metadata.

A managed staircase may retain facts that are not derivable from the current Git graph, including:

* Stable lineage identity.
* Stable step identities.
* Intended step ordering when current ancestry no longer expresses it.
* Intended membership when refs no longer express it.
* Discovery overrides.
* Review-system associations.
* Verification and landing policies.
* Historical continuity across partial landing.
* User-assigned names independent of branch names.

The terms **managed staircase**, **tracked staircase**, and **adopted staircase** refer to the same state in this addendum.

The preferred conceptual distinction is:

* **Implicit:** structurally discoverable.
* **Managed:** structurally discoverable or persistently remembered.

---

### 2.3 Adoption

**Adoption** is the transition from an implicit staircase to a managed staircase.

Adoption creates persistent metadata, including at minimum:

* A stable lineage identity.
* A record of the current integration target.
* An ordered set of stable step identities.
* The current cut or ref associated with each step.
* The current exact staircase revision.

Adoption does not inherently modify commits or ordinary branch refs.

---

### 2.4 Automatic adoption

**Automatic adoption** occurs when a command invoked on an implicit staircase requires durable staircase-specific state.

The user need not run `git staircase adopt` first.

For example:

```console
git staircase id auth --kind=lineage
```

may automatically adopt `auth`, because an implicit staircase does not have a durable lineage identity.

Likewise:

```console
git staircase rebase auth --leave-upper-steps-stale
```

must adopt `auth` before rewriting the lower step, because the resulting intended relationship will no longer be recoverable from ancestry alone.

---

## 3. General Principle

An operation may preserve an implicit staircase if all information needed to interpret the staircase after the operation remains derivable from:

* Git commits.
* Git ancestry.
* Git refs.
* The integration boundary.
* The configured discovery rules.
* Explicit arguments to the current command.

An operation requires adoption if it must preserve information that would otherwise be lost or become ambiguous after the operation.

Formally, let:

[
R(G, U, D)
]

be the staircase information reconstructible from repository graph (G), integration boundary (U), and discovery context (D).

An operation (T) may preserve implicit status when the intended post-operation staircase is fully determined by:

[
R(T(G), U', D')
]

where (U') and (D') are the resulting integration boundary and discovery context.

Adoption is required when the intended post-operation staircase contains information not recoverable from:

[
R(T(G), U', D')
]

The distinction is therefore not based on whether a command is read-only or mutating.

The distinction is based on whether the staircase's intended meaning remains structurally reconstructible.

---

## 4. First-Class Behavior of Implicit Staircases

### 4.1 Listing

`git staircase list` must include both managed and implicit staircases by default.

Example:

```text
auth             3 steps  clean       (implicit)
payments         2 steps  stale
logging          1 step   clean       (implicit)
release-cleanup  4 steps  clean
```

Implicit staircases should be visibly marked.

Recommended marker:

```text
(implicit)
```

Alternative machine-readable fields may include:

```json
{
  "management": "implicit"
}
```

Supported filters should include:

```console
git staircase list --implicit
git staircase list --managed
git staircase list --all
```

`--all` is optional if listing all staircases is already the default.

---

### 4.2 Selection

An implicit staircase may be selected anywhere a managed staircase may be selected, subject to the operation restrictions in this addendum.

A selector may use:

* A uniquely derived nominal name.
* A top ref.
* A cut ref.
* An exact revision identity.
* An explicit chain of refs.
* A generated discovery identifier.

Examples:

```console
git staircase show auth
git staircase show feature/auth-tests
git staircase show --revision 6f281c4a
git staircase show \
    --steps feature/auth-core,feature/auth-ui,feature/auth-tests
```

A derived nominal name is not a durable identity.

If several discovered staircases have the same derived name, the tool must require disambiguation.

It must not silently choose one.

Example:

```text
error: implicit staircase name 'auth' is ambiguous

candidates:
  auth@6f281c4  feature/auth-core -> feature/auth-ui
  auth@29b108e  experimental/auth-core -> experimental/auth-ui
```

---

### 4.3 Inspection

The following operations must work on implicit staircases without adoption:

```console
git staircase show
git staircase status
git staircase log
git staircase diff
git staircase graph
git staircase steps
git staircase commits
```

The output should indicate that the staircase is implicit.

Example:

```text
auth (implicit)
  target: origin/main
  state: clean
  steps: 3
  lineage: none
```

---

### 4.4 Identity queries

An implicit staircase may have identities derived from its current structure.

The following identity kinds do not require adoption:

```console
git staircase id auth --kind=revision
git staircase id auth --kind=body
git staircase id auth --kind=decomposition
git staircase id auth --kind=outcome
git staircase id auth --kind=patch-series
git staircase id auth --kind=nominal
```

The following identity requires adoption:

```console
git staircase id auth --kind=lineage
```

The command should automatically adopt the staircase unless automatic adoption has been disabled.

Example:

```text
adopted implicit staircase 'auth'
lineage: 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

---

## 5. States Available to Implicit Staircases

Not every state defined by the base specification is meaningful for an implicit staircase.

Some states describe current structure. Other states describe a difference between current structure and persistent intent.

Only the former can exist without adoption.

---

### 5.1 Clean

An implicit staircase may be **clean**.

For an implicit staircase, clean means:

* Its cuts form a valid ancestry chain.
* Each step is nonempty.
* Its body is dependency-closed relative to the integration boundary.
* Its refs and commits satisfy the active discovery rules.

Every actionable implicit staircase is clean in this structural sense.

An implicit staircase may still have failing builds or tests. Structural cleanliness does not imply verification success.

---

### 5.2 Materialized

An implicit staircase is necessarily materialized.

If it is no longer materialized, it is no longer reconstructible as that same implicit staircase.

---

### 5.3 Verified

An implicit staircase may be verified for its current exact revision.

Verification evidence may be stored without adoption if it is keyed entirely by immutable or reconstructible values such as:

* Exact staircase revision.
* Resolved target revision.
* Verification profile identity.
* Environment identity.

For example:

[
E =
H(I_{\text{revision}}, I_{\text{target}}, I_{\text{profile}})
]

This evidence describes a revision, not a lineage.

Running:

```console
git staircase verify auth --profile presubmit
```

does not by itself require adoption.

---

### 5.4 Unverified

An implicit staircase may be reported as unverified when no current revision-keyed verification evidence exists.

---

### 5.5 Ambiguous discovery result

Ambiguity may exist during discovery, but an ambiguous candidate is not yet one actionable implicit staircase.

Examples include:

* Multiple possible cut chains.
* Multiple possible integration boundaries.
* Multiple staircases with the same derived name.
* A fork with several possible staircase paths.
* Multiple refs that imply conflicting decompositions.

Ambiguity does not itself require adoption.

It requires explicit disambiguation.

Once disambiguated, the selected chain may still remain implicit.

---

### 5.6 Implicit staircase family

A staircase family may be discovered implicitly.

For example:

```text
           ui
          /
core ----<
          \
           cli
```

This may be listed as:

```text
auth-family  2 paths  (implicit)
```

The individual paths may be used as implicit staircases.

Persistent family identity or persistent shared-prefix ownership requires management.

---

## 6. States That Require Management

The following states cannot be represented reliably by an implicit staircase because they depend on remembered intent rather than current graph structure.

---

### 6.1 Stale

A staircase is stale when current commits no longer satisfy its intended dependency relationship.

For example:

```text
C_old---D---E   feature/auth-ui
   \
    C_new      feature/auth-core
```

An implicit discovery process sees two structures that no longer form one chain.

Without persistent metadata, it cannot establish that:

* `feature/auth-ui` was intended to follow `feature/auth-core`.
* `C_old` and `C_new` represent successive revisions of the same lower step.
* `feature/auth-ui` should be replayed onto `C_new`.

Therefore:

> A stale staircase must be managed.

A staircase may become stale only if it was previously managed or if a command automatically adopts it before creating the stale state.

---

### 6.2 Diverged

Divergence means multiple current objects claim to represent the same intended step or lineage position.

This concept requires stable step identity.

Therefore:

> A diverged staircase must be managed.

An implicit discovery process may report competing candidates, but it cannot call them divergent versions of the same step without persistent intent.

---

### 6.3 Incomplete

An incomplete staircase has intended steps whose current cuts or refs are missing.

Missing intent cannot be derived from current structure.

Therefore:

> An incomplete staircase must be managed.

An implicit staircase with a deleted step ref simply becomes a different discovered staircase or ceases to be discoverable.

---

### 6.4 Interrupted persistent reshape

A staircase may be left intentionally or accidentally between shapes.

Examples include:

* Only lower steps have been rebased.
* A reorder has updated some cuts but not others.
* A drop has removed a step but descendants have not yet been replayed.
* Conflict resolution is deferred across command invocations.

A command may use transient operation state without adopting the staircase, provided:

* The operation owns all necessary temporary refs.
* The operation can be resumed or aborted.
* The transient state is not presented as a durable staircase state.
* Completion or abortion restores a fully discoverable structure.

If an interrupted state is to survive as a recognized staircase after the operation relinquishes control, the staircase must be managed.

---

### 6.5 Verification-stale as lineage state

A current implicit revision can be verified or unverified.

It cannot reliably be called a newer revision of a previously verified staircase without lineage information.

Therefore:

> Verification-stale, when it means that an earlier revision of the same staircase was verified, requires management.

Revision-keyed verification evidence may still exist without management, but unmatched older evidence is not attached to an implicit lineage.

---

### 6.6 Partially landed with continuity

The remaining upper steps of a partially landed implicit staircase may be rediscovered as a shorter staircase relative to the advanced target.

For example, a three-step implicit staircase may become a newly discovered two-step implicit staircase.

This does not require adoption.

However, the claim that the two-step staircase is the continuing identity of the original three-step staircase requires lineage metadata.

Therefore:

* Partial landing does not inherently require adoption.
* Preserving staircase continuity across partial landing does require adoption.

---

## 7. Operations That Preserve Implicit Status

This section defines when commands may operate on an implicit staircase without adopting it.

---

### 7.1 Discovery and listing

The following commands never require adoption:

```console
git staircase discover
git staircase list
git staircase list --implicit
git staircase list --families
```

They observe current repository structure.

---

### 7.2 Showing and inspecting

The following commands do not require adoption:

```console
git staircase show
git staircase status
git staircase graph
git staircase log
git staircase diff
git staircase steps
git staircase commits
```

They may report only facts derivable from the current repository state.

For an implicit staircase, they must not claim:

* Stable lineage.
* Historical continuity.
* Staleness relative to a prior shape.
* Stable step identity across rewrites.
* Persistent review associations.

---

### 7.3 Revision-derived identities

The following operations preserve implicit status:

```console
git staircase id --kind=revision
git staircase id --kind=body
git staircase id --kind=decomposition
git staircase id --kind=outcome
git staircase id --kind=patch-series
git staircase id --kind=nominal
```

---

### 7.4 Verification of the current revision

The following operation preserves implicit status:

```console
git staircase verify auth
```

This remains true when verification evidence is stored by exact revision and target.

The operation requires adoption only if it also attaches persistent policy or lineage-relative state.

Examples requiring adoption:

```console
git staircase verify auth --set-default-profile presubmit
git staircase policy set auth verification=each-prefix
git staircase verify auth --record-against-lineage
```

---

### 7.5 Split

A split may preserve implicit status when the new cut remains structurally discoverable.

Example:

```console
git staircase split auth:2 \
    --at D \
    --branch feature/auth-model
```

The new branch ref materializes the inserted cut.

The resulting staircase remains implicit if discovery can reconstruct:

```text
feature/auth-core
feature/auth-model
feature/auth-ui
```

A split requires adoption when the new cut is intentionally not represented by a discoverable ref or rule.

Example:

```console
git staircase split auth:2 --at D --no-ref
```

This must automatically adopt the staircase because the new cut exists only as staircase metadata.

---

### 7.6 Join

A join may preserve implicit status if the removed cut also ceases to be recognized by discovery.

For example:

```console
git staircase join auth:2 auth:3 --delete-boundary-ref
```

or:

```console
git staircase join auth:2 auth:3 \
    --rename-boundary-ref refs/archive/auth-model
```

The resulting current refs must describe the joined staircase unambiguously.

A join requires adoption when:

* The old boundary ref is retained.
* The discovery rules would still treat that ref as a cut.
* The staircase must remember that the ref is intentionally not a step boundary.

Example:

```console
git staircase join auth:2 auth:3 --keep-boundary-ref
```

This must automatically adopt the staircase and record a discovery override.

---

### 7.7 Append

Appending commits within the final step does not require adoption.

Creating a new step does not require adoption if the new cut is represented by a discoverable ref.

Examples:

```console
git staircase append auth --commits HEAD~2..HEAD

git staircase append auth \
    --new-step \
    --branch feature/auth-tests
```

Appending an unnamed or metadata-only step requires adoption.

---

### 7.8 Reorder

Reordering may preserve implicit status if:

1. The staircase is clean and unambiguous when the command starts.
2. The command knows all existing cuts.
3. The command performs the complete rewrite.
4. All affected refs are updated.
5. The final result is again discoverable as one clean staircase.
6. No durable identity across the rewrite is requested.

Example:

```console
git staircase reorder auth --steps 1,3,2
```

The tool may capture the current structure, rewrite the commits, and update all cut refs without creating a managed staircase.

Reordering requires adoption when the operation intentionally leaves:

* Upper steps stale.
* Missing step tips.
* Conflicting candidate cuts.
* A metadata-only step order.
* Stable step identities that must survive the reorder.

Examples:

```console
git staircase reorder auth --steps 1,3,2 --no-restack
git staircase reorder auth --stop-after 2
```

---

### 7.9 Move

Moving commits or patch fragments between steps may preserve implicit status if the complete resulting staircase is rewritten and all cut refs remain discoverable.

Example:

```console
git staircase move auth \
    --from 3 \
    --to 2 \
    <commit>
```

It requires adoption if the command leaves an intentionally incomplete or stale shape.

---

### 7.10 Drop

Dropping a step may preserve implicit status if:

* The command removes that step’s changes.
* It rewrites all dependent upper steps.
* It updates all cut refs.
* The final staircase remains discoverable.

Example:

```console
git staircase drop auth:2 --restack
```

Dropping requires adoption when descendants are intentionally left on the former step.

Example:

```console
git staircase drop auth:2 --leave-descendants-stale
```

The intended relationship between the surviving steps would otherwise be lost.

---

### 7.11 Rebase

Rebasing a complete implicit staircase may preserve implicit status if the command performs the rebase as one staircase-aware operation.

Example:

```console
git staircase rebase auth --onto origin/main
```

The command may:

1. Capture the current cuts.
2. Rebase or replay the body.
3. Reconstruct each cut.
4. Update the corresponding refs.
5. Produce a clean discoverable staircase.

No permanent lineage metadata is required.

Rebase requires adoption when:

* Only part of the staircase is rebased.
* Upper steps are intentionally left stale.
* Some cut refs are unavailable after the operation.
* Persistent mapping from old steps to new steps is requested.
* External review identities must be preserved.
* The resulting staircase is not immediately discoverable.

---

### 7.12 Restack

Restacking does not inherently require adoption.

A clean implicit staircase may be restacked as part of a command that rewrites one step and all descendants atomically.

Example:

```console
git staircase restack auth --onto origin/main
```

An explicit chain may also be restacked without adoption:

```console
git staircase restack \
    feature/auth-core \
    feature/auth-ui \
    feature/auth-tests
```

However, a previously untracked staircase that has already become stale cannot generally be rediscovered as the same staircase.

The following command may fail:

```console
git staircase restack auth
```

if `auth` no longer resolves to one chain and no managed metadata exists.

The user may still supply the intended chain explicitly:

```console
git staircase restack \
    --steps feature/auth-core,feature/auth-ui,feature/auth-tests
```

If the explicit operands fully specify the intended relationship and the operation returns the repository to a discoverable clean state, adoption is not required.

If the user wants the stale relationship retained before completion, adoption is required.

---

### 7.13 Landing

Aggregate or stepwise landing may operate on an implicit staircase.

Examples:

```console
git staircase land auth --aggregate
git staircase land auth --stepwise
```

The staircase may remain implicit if:

* The operation uses only the current discovered structure.
* The remaining unlanded work is discoverable afterward.
* No persistent landing history or lineage continuity is required.
* No persistent review mapping is required.

Landing requires adoption when the system must retain:

* Which landed reviews belonged to the staircase.
* Staircase identity across partial landing.
* Landing policy.
* Approval requirements.
* A durable landing ledger.
* Recovery information after a non-atomic partial landing.

---

### 7.14 Rename

Renaming ordinary refs may preserve implicit status if the resulting names and ancestry still allow discovery.

Example:

```console
git staircase rename-step auth:2 feature/auth-model
```

Assigning a stable staircase name independent of refs requires management.

Example:

```console
git staircase name auth authentication-redesign
```

This must adopt the staircase unless the command merely renames branches according to a reversible convention.

---

### 7.15 Delete

An implicit staircase has no management record to delete.

Therefore:

```console
git staircase delete auth
```

must not pretend to delete an implicit staircase entity unless its ref effects are explicit.

Recommended behavior:

```text
error: 'auth' is implicit and has no managed record to delete

use:
  git staircase delete auth --delete-step-refs
```

The following may operate without adoption:

```console
git staircase delete auth --delete-step-refs
```

This deletes the refs that currently materialize the implicit staircase, subject to ordinary Git safety rules.

It does not first adopt the staircase.

After deletion, the staircase ceases to be discoverable unless other refs still expose its cuts.

---

## 8. Operations That Require Adoption

The following operations or requested semantics require persistent staircase metadata.

---

### 8.1 Requesting lineage identity

```console
git staircase id auth --kind=lineage
```

A lineage identity cannot be derived from current commit content alone.

---

### 8.2 Stable step identity

Positions such as `auth:2` are not stable under insertion, deletion, or reordering.

If the user requests an identifier meaning:

> This same conceptual step across rewrites and position changes

the staircase must be managed.

Example:

```console
git staircase step-id auth:2
```

This may automatically adopt the staircase.

---

### 8.3 Persistent name independent of refs

A display name derived from branch names may remain implicit.

A stable name that survives arbitrary ref renaming requires adoption.

---

### 8.4 Discovery overrides

Adoption is required to record statements such as:

* Include this ref even though discovery would exclude it.
* Exclude this ref even though discovery would include it.
* Treat this ref as an alias, not a step boundary.
* Treat this unnamed commit as a cut.
* Assign these incomparable refs to a declared order.
* Treat these refs as members of one named family.
* Preserve this branch as belonging to the staircase after ancestry changes.

Examples:

```console
git staircase include auth feature/special-case
git staircase ignore-cut auth feature/auth-model
git staircase split auth:2 --at D --no-ref
```

---

### 8.5 Persistent policy

Attaching policy requires management.

Examples include:

```console
git staircase policy set auth verification=each-prefix
git staircase policy set auth landing=stepwise
git staircase policy set auth target=origin/main
git staircase policy set auth require-review=true
```

A command-line option used for one invocation does not require adoption.

For example:

```console
git staircase verify auth --each-prefix
```

does not attach policy and may preserve implicit status.

---

### 8.6 External review associations

Persistent associations with Gerrit changes, pull requests, or other review records require management.

Examples:

```console
git staircase review attach auth:1 Iabc123
git staircase upload auth --remember-reviews
```

A one-time upload that does not retain the mapping may avoid adoption, though such behavior may be less useful.

---

### 8.7 Preserving nonmaterialized states

Any command that intentionally leaves the staircase stale, incomplete, or diverged must adopt it before creating that state.

Examples:

```console
git staircase rebase auth:1 --leave-upper-steps-stale
git staircase drop auth:2 --no-restack
git staircase reorder auth --stop-after 2
```

Without management, the staircase would cease to exist as one discoverable entity.

---

### 8.8 Historical continuity

The following claims require management:

* This staircase is the successor of an earlier staircase revision.
* This remaining upper stack is the same staircase after partial landing.
* This rebased chain is the same staircase as the previous chain.
* This restored chain is the same staircase after branch deletion.
* These new commits replace the former version of step 2.

These are lineage claims rather than current structural facts.

---

### 8.9 Persistent verification state by lineage

Storing evidence for one exact revision does not require management.

Storing statements such as the following does:

* This staircase was previously verified.
* The current revision invalidates an earlier successful verification.
* Step 2 has not been reverified since it changed.
* Verification history belongs to this evolving staircase.

---

### 8.10 Persistent transport metadata

An implicit staircase may be transported by pushing its ordinary refs and rediscovering it in another repository.

Adoption is required when the system must transport:

* Stable lineage identity.
* Stable step identities.
* Discovery overrides.
* Review associations.
* Policies.
* Historical records.
* Metadata-only cuts.

---

## 9. Operation Matrix

The following matrix is normative.

| Operation or state                                      |      Allowed on implicit |  May remain implicit | Adoption trigger                                                    |
| ------------------------------------------------------- | -----------------------: | -------------------: | ------------------------------------------------------------------- |
| `discover`                                              |                      Yes |                  Yes | Never                                                               |
| `list`                                                  |                      Yes |                  Yes | Never                                                               |
| `show`, `graph`, `log`, `diff`                          |                      Yes |                  Yes | Never                                                               |
| Current structural `status`                             |                      Yes |                  Yes | Never                                                               |
| Revision, body, outcome, decomposition, patch identity  |                      Yes |                  Yes | Never                                                               |
| Lineage identity                                        |                      Yes |                   No | Always                                                              |
| Verify current exact revision                           |                      Yes |                  Yes | Only if lineage-relative evidence or policy is requested            |
| Split with discoverable ref                             |                      Yes |                  Yes | No                                                                  |
| Split with metadata-only cut                            |                      Yes |                   No | Always                                                              |
| Join and remove discoverable boundary                   |                      Yes |                  Yes | No                                                                  |
| Join while retaining a ref that discovery sees as a cut |                      Yes |                   No | Always                                                              |
| Append commits to existing step                         |                      Yes |                  Yes | No                                                                  |
| Add step with discoverable ref                          |                      Yes |                  Yes | No                                                                  |
| Add unnamed or metadata-only step                       |                      Yes |                   No | Always                                                              |
| Reorder completely and atomically                       |                      Yes |                  Yes | No                                                                  |
| Reorder while preserving incomplete intermediate state  |                      Yes |                   No | Always                                                              |
| Move changes and fully restack                          |                      Yes |                  Yes | No                                                                  |
| Drop and fully restack                                  |                      Yes |                  Yes | No                                                                  |
| Drop while leaving descendants stale                    |                      Yes |                   No | Always                                                              |
| Rebase complete staircase                               |                      Yes |                  Yes | No                                                                  |
| Rebase lower prefix only and retain stale upper steps   |                      Yes |                   No | Always                                                              |
| Restack from a currently known clean chain              |                      Yes |                  Yes | No                                                                  |
| Recover an already stale chain by managed name          |                       No |       Not applicable | Management must already exist                                       |
| Recover an already stale chain from explicit operands   |                      Yes |                  Yes | No, if the final state is clean and discoverable                    |
| Aggregate landing                                       |                      Yes |                  Yes | Only if durable history, review mapping, or continuity is requested |
| Stepwise landing                                        |                      Yes |                  Yes | Only if durable history, policy, or recovery state is requested     |
| Partial landing as a newly discovered shorter staircase |                      Yes |                  Yes | No                                                                  |
| Partial landing preserving original lineage             |                      Yes |                   No | Always                                                              |
| Derived nominal name                                    |                      Yes |                  Yes | No                                                                  |
| Stable name independent of refs                         |                      Yes |                   No | Always                                                              |
| Attach review record                                    |                      Yes |                   No | Always                                                              |
| Attach landing or verification policy                   |                      Yes |                   No | Always                                                              |
| Delete managed record                                   | No managed record exists |       Not applicable | Must not auto-adopt                                                 |
| Delete refs materializing implicit staircase            |                      Yes | Staircase disappears | Never                                                               |
| Clean state                                             |                      Yes |                  Yes | No                                                                  |
| Stale state                                             |                       No |                   No | Management required before entering state                           |
| Diverged state                                          |                       No |                   No | Management required                                                 |
| Incomplete state                                        |                       No |                   No | Management required                                                 |
| Verified current revision                               |                      Yes |                  Yes | No                                                                  |
| Verification-stale lineage state                        |                       No |                   No | Management required                                                 |
| Implicit family discovered from current graph           |                      Yes |                  Yes | No                                                                  |
| Persistent family identity                              |                      Yes |                   No | Always                                                              |

---

## 10. Automatic Adoption Semantics

### 10.1 Default behavior

When a command requires adoption, the tool should adopt automatically.

Example:

```console
git staircase rebase auth:1 --leave-upper-steps-stale
```

Possible output:

```text
adopting implicit staircase 'auth'
reason: preserving intended upper-step dependencies after ancestry is broken
lineage: 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

The operation then proceeds against the managed staircase.

---

### 10.2 User control

The following options are recommended:

```console
--no-adopt
--adopt
```

`--no-adopt` means:

> Fail rather than create persistent staircase metadata.

Example:

```console
git staircase split auth:2 --at D --no-ref --no-adopt
```

Result:

```text
error: this operation requires adoption because the new cut would not be discoverable from refs
```

`--adopt` means:

> Adopt before executing, even if the operation could otherwise preserve implicit status.

This is useful for:

* Scripts that need lineage identity.
* Users who want explicit tracking.
* Operations expected to be followed by ordinary Git commands that may break structural discovery.

---

### 10.3 Atomicity

Automatic adoption and the triggering operation should behave as one logical transaction.

Before changing commits or refs, the tool should establish enough recovery information to preserve staircase intent.

If the operation fails before any durable mutation:

* Provisional adoption metadata should be removed.
* The staircase should remain implicit.

If the operation fails after leaving a state that requires persistent intent:

* The adoption record must remain.
* The staircase should be marked interrupted, stale, or incomplete as appropriate.
* Recovery commands should use the retained managed state.

---

### 10.4 Visibility

Automatic adoption must not be completely silent.

Human-readable output should identify:

* That adoption occurred.
* Why it was required.
* The new lineage identity or managed name.

Machine-readable output should include:

```json
{
  "adopted": true,
  "adoption_reason": "persistent-stale-state",
  "lineage": "3d7d16d1-14c8-4d86-a55d-9ce54094bc25"
}
```

---

## 11. Transient Operation State

A distinction must be maintained between:

* Persistent staircase management.
* Temporary state used to complete one operation.

A staircase-aware rebase may create temporary refs such as:

```text
refs/staircase-operations/<operation-id>/original/1
refs/staircase-operations/<operation-id>/original/2
refs/staircase-operations/<operation-id>/rewritten/1
```

This does not necessarily constitute adoption.

Transient state preserves the mechanics of one active operation.

Managed state preserves the identity and intent of a staircase beyond that operation.

Transient operation state must:

* Be scoped to one operation.
* Be removable on successful completion or abortion.
* Not assign durable lineage unless adoption occurs.
* Not cause the staircase to appear as managed after successful completion.
* Support resume and abort semantics where practical.

---

## 12. Behavior of `git staircase status`

For an implicit staircase:

```console
git staircase status auth
```

may report:

```text
auth (implicit)
  target: origin/main
  state: clean
  steps: 3
  materialized: yes
  lineage: none
  verification: passed for current revision
```

It must not report `stale` based only on branch naming similarity or historical guesses.

If discovery finds related but non-chain branches, it may report:

```text
no actionable implicit staircase named 'auth'

possible related refs:
  feature/auth-core
  feature/auth-ui

reason:
  neither tip is currently an ancestor of the other

use:
  git staircase restack \
      --steps feature/auth-core,feature/auth-ui

or create persistent intent with:
  git staircase adopt \
      --steps feature/auth-core,feature/auth-ui
```

The second command explicitly declares the intended relationship and creates a managed staircase.

---

## 13. Revised Meaning of `git staircase adopt`

Explicit adoption remains useful, but it is no longer a prerequisite for ordinary staircase operations.

Its revised purpose is:

> Persist the current staircase's identity, membership, step identities, and intended structure before those facts become unrecoverable from Git structure.

Examples:

```console
git staircase adopt auth
```

or:

```console
git staircase adopt auth \
    --onto origin/main \
    feature/auth-core \
    feature/auth-ui \
    feature/auth-tests
```

Valid reasons for explicit adoption include:

* The user expects to amend lower branches using ordinary Git commands.
* The user wants stable lineage identity.
* The user wants stable step identities.
* The staircase will be synchronized with a review system.
* Policies will be attached.
* The staircase may temporarily become stale or incomplete.
* The user wants continuity across partial landing.
* The user wants to transport metadata across repositories.

Explicit adoption should be optional in normal clean workflows.

The command may be renamed to:

```console
git staircase track
```

if `track` better communicates that the staircase already exists and is merely acquiring persistent management.

`adopt` and `track` may be aliases.

---

## 14. Can the Adoption Concept Be Removed?

The requirement for explicit prior adoption can be removed.

The requirement for persistent managed state cannot be removed without losing important semantics.

The following concepts fundamentally require information beyond current Git ancestry and refs:

* Stable lineage identity.
* Stable step identity across rewrites.
* Stale staircase recognition.
* Diverged staircase recognition.
* Incomplete staircase recognition.
* Discovery overrides.
* Metadata-only cuts.
* Durable review associations.
* Durable landing and verification policy.
* Continuity across partial landing.
* Recovery of intended structure after arbitrary ordinary Git operations.

Therefore the specification should retain the distinction between implicit and managed staircases.

However, adoption should not be treated as a ceremony the user must perform before using staircase functionality.

The recommended model is:

1. A discoverable staircase is immediately usable as an implicit staircase.
2. Commands preserve implicit status whenever the resulting staircase remains reconstructible.
3. Commands automatically create managed state only when durable intent becomes necessary.
4. Users may force or forbid that transition.
5. Explicit `adopt` or `track` remains available but is optional.

---

## 15. Revised Summary Definition

A staircase may exist in either of two representation modes.

### Implicit staircase

A staircase reconstructed from current Git commits, ancestry, refs, target boundary, and discovery rules.

It is fully usable for operations whose inputs and results remain structurally discoverable.

It has revision-derived identities but no durable lineage.

### Managed staircase

A staircase with persistent metadata recording facts that may not remain structurally discoverable.

It can survive stale, diverged, incomplete, partially landed, or metadata-defined states.

### Transition rule

An implicit staircase becomes managed precisely when an operation or requested state requires preserving information that cannot be reconstructed from lower-level Git concepts.

This transition should normally occur automatically.

In compact form:

> Prefer structure over metadata. Introduce metadata only when intent would otherwise be lost.
