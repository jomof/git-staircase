# Addendum L: Direct Archival of Implicit Staircases Without Adoption

## 1. Status and scope

This addendum modifies and supersedes conflicting archive behavior in the Git Staircase specification and prior addenda.

In particular, it supersedes any requirement that:

* Archive requires management.
* An implicit staircase must be adopted before archive.
* Archiving an implicit staircase performs an implicit-to-managed-to-archived transition.
* Every archived staircase must have a lineage ID or stable step IDs.
* `--no-adopt` prevents archiving an implicit staircase.

The governing requirement is:

> `git staircase archive <selector>` MUST archive either a managed staircase or a canonical implicit staircase. An implicit staircase MUST be archived directly, without first adopting it, unless the user explicitly requests adoption.

The following command is therefore required to work for both managed and unadopted implicit staircases:

```console
git staircase archive <name>
```

A command MUST NOT fail merely because the selected staircase is implicit.

Ordinary failures remain possible for genuine ambiguity, concurrent mutation, unsafe worktree state, ref collisions, corrupt objects, or another specific archive precondition defined below.

---

## 2. Core principle

Archive preserves enough exact state to inspect and restore work while removing it from active Staircase use.

That preservation does not inherently require a continuing staircase lineage.

Two archive paths therefore exist:

```text
managed active staircase
        ↓
managed archive

canonical implicit staircase
        ↓
implicit archive snapshot
```

The second path MUST NOT pass through managed active state.

An implicit archive snapshot is durable, but it is not an adopted or managed staircase.

It preserves one archived occurrence of one exact implicit structure. It does not assert that the staircase had a durable identity before archive or that it will retain conceptual identity after restoration.

---

## 3. Terminology

### 3.1 Managed archive

A **managed archive** is the archived lifecycle state of an existing managed staircase.

It preserves:

```text
lineage ID
stable step IDs
structure revision
metadata revision
review associations
policies
owned refs
lifecycle history
```

Existing managed archive semantics remain unchanged except where this addendum explicitly modifies common selection, suppression, or output behavior.

---

### 3.2 Implicit archive snapshot

An **implicit archive snapshot** is an archive-only persistent representation created directly from one canonical implicit staircase.

It preserves:

```text
archive ID
originating structural key
repository identity
object format
exact integration context
ordered cut OIDs
step ordinals
discovery provenance
display name and aliases
owned-ref restoration information
archive reason and event information
required object reachability
```

It does not contain or create:

```text
lineage ID
stable step IDs
active managed record
persistent staircase name
persistent staircase policy
persistent review association
lineage-relative verification history
pre-archive lifecycle history
```

The archive ID identifies the archive snapshot, not an evolving staircase.

---

### 3.3 Archive identity

Every implicit archive snapshot has an opaque lowercase UUID called its **archive ID**.

Canonical machine selector:

```text
archive@<archive-id>
```

Canonical typed selector:

```console
git staircase show --archive-id <archive-id>
git staircase unarchive --archive-id <archive-id>
git staircase delete --archived --archive-id <archive-id>
```

An archive ID:

* Is generated when the archive snapshot is created.
* Remains stable for that archive snapshot.
* Is not a lineage ID.
* Is not a step ID.
* Does not survive deletion of that archive snapshot.
* Does not imply identity continuity after unarchive.
* MUST NOT be used where a lineage ID is required.

---

### 3.4 Archived representation kinds

Every archived entry has one explicit representation kind:

```text
managed-lineage
implicit-snapshot
```

Human and machine output MUST distinguish them.

The generic word `archived` may describe both, but a command MUST NOT imply that an implicit snapshot has managed identity.

---

## 4. Representation and lifecycle model

### 4.1 Active representation

An active staircase is either:

```text
implicit
managed
```

An active implicit staircase remains reconstructible from current Git state and has no persistent staircase record.

---

### 4.2 Archived representation

An archived entry is either:

```text
archived managed lineage
archived implicit snapshot
```

An implicit archive snapshot is not an “active implicit staircase with archived lifecycle state.” Once archived, it is represented by the archive snapshot rather than by active implicit discovery.

---

### 4.3 Direct transition requirement

For an implicit staircase, archive MUST perform:

```text
canonical implicit discovery snapshot
        ↓
archive-only snapshot objects and refs
```

It MUST NOT perform:

```text
canonical implicit staircase
        ↓
managed active staircase
        ↓
managed archived staircase
```

unless `--adopt` was explicitly supplied.

---

### 4.4 No hidden adoption

When directly archiving an implicit staircase, the implementation MUST NOT:

* Allocate a lineage ID.
* Allocate stable step IDs.
* Create a ref under `refs/staircases/`.
* Create an active record ref under `refs/staircase-state/`.
* Publish an active managed record, even temporarily.
* Emit an adoption event.
* Report that adoption occurred.
* Retain a provisional managed record after failure.
* Describe the archive snapshot as adopted or managed.

Internal objects may be written before ref publication, but they MUST use the implicit archive snapshot schema and MUST NOT masquerade as an active managed record.

---

## 5. Command semantics

### 5.1 Default command

```console
git staircase archive <selector>
```

The command MUST accept:

* A managed staircase selector.
* A canonical implicit staircase display name or alias.
* An implicit structural key.
* A Git revision resolving unambiguously to one canonical staircase.
* Other selectors accepted by the core selector algorithm.

After selection:

```text
managed selection
    → managed archive path

implicit selection
    → direct implicit snapshot path
```

---

### 5.2 Required invariant

The following behavior is nonconforming:

```console
$ git staircase list
feature  3 steps  clean  (implicit)

$ git staircase archive feature
error: archive requires adoption
```

The conforming behavior is:

```console
$ git staircase archive feature
Archived implicit staircase 'feature'.
  archive ID: <ARCHIVE-ID>
  originating structural key: implicit@<DIGEST>
  adopted: no
```

---

### 5.3 Explicit `--adopt`

The user may explicitly request managed identity before archive:

```console
git staircase archive <implicit-selector> --adopt
```

This performs adoption and archive as one logical operation:

```text
implicit
    → managed
    → managed archive
```

Human output MUST say that explicit adoption occurred.

Machine output MUST include:

```json
{
  "source_representation": "implicit",
  "archive_kind": "managed-lineage",
  "adopted": true,
  "adoption_reason": "explicit-before-archive",
  "lineage_id": "<uuid>"
}
```

---

### 5.4 `--no-adopt`

The following MUST succeed when all ordinary archive preconditions are satisfied:

```console
git staircase archive <implicit-selector> --no-adopt
```

For direct implicit archive, `--no-adopt` confirms the default behavior.

For an already managed staircase, `--no-adopt` does not make it implicit and does not block archive.

---

### 5.5 Options requiring managed identity

An option that specifically requests durable staircase semantics unavailable to an implicit snapshot MUST fail unless `--adopt` is explicitly supplied.

Examples include requests to create or retain:

* Stable step identity.
* A persistent review mapping.
* A persistent staircase policy.
* Lineage-relative verification history.
* Metadata keyed by stable step ID.
* Continuity across later reshaping or partial landing.
* A managed canonical name independent of restored Git refs.

The failure MUST occur before mutation and MUST identify the exact requested feature:

```text
error: requested archive option requires managed identity

option:
  --preserve-review-mapping

the staircase is implicit and direct archive does not create lineage

use:
  git staircase archive feature --adopt --preserve-review-mapping
```

Plain archive and `--reason` MUST NOT require adoption.

---

## 6. Selection and ambiguity

### 6.1 Canonical selection first

Archive MUST resolve one canonical active staircase before planning archive state.

It MUST NOT archive raw discovery records independently.

Equivalent evidence from several refs, providers, caches, or graph traversals remains one staircase and produces at most one archive entry.

---

### 6.2 Managed and implicit interpretations

The normal selector algorithm applies.

If a managed staircase and an implicit candidate describe the same managed lineage and exact current structure, only the managed staircase is selected.

If a name identifies genuinely distinct managed or implicit staircases, archive MUST report ambiguity.

It MUST NOT prefer managed state merely because one candidate is managed.

---

### 6.3 Active and archived selection scopes

For `archive`, selector resolution proceeds as follows:

1. Resolve against active managed and active implicit staircases.
2. If one active result exists, archive it.
3. If several active results exist, report ambiguity.
4. If no active result exists, resolve against archived entries only to support an idempotent no-op.
5. An archived match MUST NOT hide an active match.

For `unarchive`, selection is restricted to archived entries.

---

### 6.4 Referential integrity

A unique name printed by `git staircase list` MUST remain usable by `git staircase archive` while relevant state remains unchanged.

Archive MUST use the same canonicalization and equivalence rules as listing.

Duplicate internal evidence MUST NOT become archive-time ambiguity.

---

## 7. Implicit archive snapshot schema

### 7.1 Snapshot descriptor

The snapshot descriptor MUST use a versioned deterministic schema containing at least:

```text
schema version
representation kind: implicit-snapshot
archive ID
repository identity
object format
originating implicit structural key
exact integration-context identity
ordered cut OIDs
step count
discovery provenance
canonical display name
aliases
materializing refs observed at planning time
```

The descriptor MUST NOT contain invented lineage or step IDs.

Step positions are represented by one-based ordinal in the archived snapshot.

---

### 7.2 Archive event data

The archive lifecycle data contains at least:

```text
state: archived
archive ID
archive event ID
archive time
optional actor
optional reason
source representation: implicit
name-reservation policy
retention policy
parent archive event, if explicitly retained
```

This lifecycle data belongs to the archive snapshot.

It does not imply that the active implicit staircase had a lifecycle record before archive.

---

### 7.3 Archive manifest

The archive manifest contains, as applicable:

```text
originating structural key
exact integration context
ordered cuts
operation-owned refs
unowned materializing refs
original refnames and OIDs
archive retention refnames
branch configuration snapshots
worktree observations
draft disposition
provider observations
snapshot-tag disposition
restoration policy
discovery configuration fingerprint
```

Provider observations copied into an implicit archive snapshot are non-authoritative observations unless separately represented by a managed provider association.

---

### 7.4 Archive record tree

The canonical implicit archive record is a Git tree such as:

```text
<implicit archive record>
├── kind
├── snapshot
├── lifecycle
└── archive-manifest
```

Its tree OID is the **archive record revision OID**.

It is not a managed staircase record revision.

Commands MUST label it accordingly.

---

## 8. Ref storage and reachability

### 8.1 Managed archive namespace

Existing managed archives may continue to use:

```text
refs/staircase-archive/<lineage-id>/
```

---

### 8.2 Implicit archive namespace

Implicit archive snapshots use:

```text
refs/staircase-archive/implicit/<archive-id>/
```

Required refs include:

```text
refs/staircase-archive/implicit/<archive-id>/record
refs/staircase-archive/implicit/<archive-id>/cuts/0001
refs/staircase-archive/implicit/<archive-id>/cuts/0002
...
refs/staircase-archive/implicit/<archive-id>/owned/<ref-entry-id>
```

Where:

* `record` points to the archive record tree.
* `cuts/<ordinal>` points directly to each retained cut commit.
* `owned/<ref-entry-id>` preserves the exact target of an archived ref.

The ordinal must be zero-padded sufficiently for lexical ordering or encoded through another versioned deterministic scheme.

---

### 8.3 Ref entry IDs

An implicit snapshot has no stable step IDs or lineage-owned ref IDs.

Archive-local ref entry IDs MAY be generated for manifest addressing.

They are scoped to one archive snapshot and MUST NOT be presented as conceptual staircase identity.

---

### 8.4 Object retention

Archive refs MUST preserve every object required to:

* Inspect all archived cuts.
* Reconstruct the archived step decomposition.
* Restore archived refs.
* Validate the archived integration context.
* Restore selected draft snapshots.
* Read the archive record and manifest.
* Complete or roll back an interrupted archive operation.

Textual OIDs inside blobs are not sufficient reachability roots.

---

## 9. Ownership and ref handling

### 9.1 Archive-operation ownership

An implicit staircase has no persistent ref ownership.

For direct archive, the implementation may derive **archive-operation ownership** for the current operation only.

Archive-operation ownership grants permission to move a ref during this archive transaction. It does not create continuing managed ownership.

---

### 9.2 Strictly inferred operation-owned refs

A local branch may be operation-owned when one of the following is true:

1. It is a primary branch in a complete, uniquely recognized sequential layout.
2. For a one-step staircase, it is the unique eligible local branch that supplies the canonical display name and the unique local primary materialization.
3. The user explicitly names it with an archive ref-selection option.
4. Another discovery profile explicitly defines an equally strict and unambiguous ownership convention.

OID equality, naming similarity, upstream configuration, reflog similarity, or provider association alone is insufficient.

---

### 9.3 Unowned aliases

Materializing refs not proven operation-owned remain unowned.

They MUST NOT be moved automatically.

Archive still succeeds unless another safety rule blocks it.

The command MUST report retained aliases:

```text
warning: archived staircase remains reachable through unowned refs

  refs/heads/feature-copy
  refs/remotes/user/feature

these refs were not moved
the exact archived structure is suppressed from active Staircase discovery
```

---

### 9.4 Ambiguous ownership does not imply selector ambiguity

If the staircase structure is unique but ref ownership is ambiguous:

* Archive MUST NOT report staircase-selector ambiguity.
* Archive MUST NOT move the ambiguous refs.
* Archive MAY archive other unambiguously owned refs.
* Archive MUST create the implicit archive snapshot.
* Archive MUST report which materializing refs remained.

An implementation MAY provide stricter options such as:

```console
git staircase archive feature --require-all-local-refs-hidden
git staircase archive feature --archive-ref refs/heads/feature
```

`--require-all-local-refs-hidden` fails before mutation unless every relevant local ref can be moved safely.

---

### 9.5 Overlap with other staircases

An operation-owned ref MUST NOT be moved when it is:

* Owned by another managed lineage.
* Required as an owned primary ref by another selected archive plan.
* Checked out in an incompatible worktree state.
* The only materializing ref of another distinct active staircase, unless an explicit multi-staircase operation is supported.

Shared commit OIDs alone do not block archive.

The diagnostic MUST distinguish structural overlap from ref ownership conflict.

---

## 10. Archived-structure suppression

### 10.1 Purpose

Unowned aliases, detached worktrees, remote-tracking refs, or tags may continue to expose commits after archive.

Without an additional rule, the same exact structure could immediately be rediscovered as a new implicit staircase, making archive appear ineffective.

Therefore archive records participate in active discovery suppression.

---

### 10.2 Suppression rule

After canonical implicit discovery, the implementation MUST compare each canonical implicit structural key with committed archive records.

A canonical implicit candidate whose exact structural key is represented by an archived entry is suppressed from normal active Staircase results unless:

* It is being restored by the current unarchive transaction.
* An active managed record explicitly claims the same current structure.
* The user explicitly requests diagnostic inclusion of archived materializations.

This applies to both managed and implicit archive records when their archive manifest records the corresponding exact structural key.

---

### 10.3 Scope of suppression

Suppression is exact.

It covers:

```text
repository identity
object format
integration-context identity
ordered cut OIDs
```

It MUST NOT suppress a distinct staircase merely because it has:

* The same display name.
* The same top commit.
* Overlapping commits.
* The same branch names.
* The same cuts under a different integration context.

---

### 10.4 Changed structures

If a retained branch moves to a new cut, the resulting staircase has a different structural key and is not suppressed by the old archive snapshot.

That new candidate may be listed as active.

Diagnostics SHOULD mention its overlap with an archived snapshot when useful.

---

### 10.5 Diagnostic visibility

The core SHOULD support:

```console
git staircase list --diagnostics
git staircase list --include-archived-materializations
```

Diagnostics should distinguish:

```text
canonical active staircases
canonical candidates suppressed by managed archives
canonical candidates suppressed by implicit archive snapshots
```

Suppression MUST NOT occur at raw-evidence level.

---

## 11. Worktrees and drafts

### 11.1 Worktree inspection

Archive MUST inspect every linked worktree before publication.

---

### 11.2 Clean worktree on an operation-owned branch

A clean worktree symbolically attached to an operation-owned branch is detached at the same exact commit before that branch is removed.

The worktree files and index remain unchanged.

---

### 11.3 Dirty worktree on an operation-owned branch

A dirty worktree attached to an operation-owned branch blocks default archive.

Existing explicit options remain applicable:

```console
git staircase archive <selector> --snapshot-drafts
git staircase archive <selector> --detach-dirty-worktrees
git staircase archive <selector> --leave-worktrees
```

`--leave-worktrees` is invalid when a symbolic `HEAD` would refer to a branch being removed.

Snapshotting a draft for an implicit archive stores it as archive-local recovery state. It does not create persistent attachment to a lineage or stable step ID.

---

### 11.4 Worktree on an unowned branch

A worktree attached to an unowned branch does not block archive merely because the branch points into the archived structure.

The branch and worktree remain unchanged.

The command MUST warn that a live worktree continues to expose or modify related commits.

---

### 11.5 Detached worktree at an archived cut

A clean detached worktree at an archived cut MAY remain at that commit.

The exact archived structural key remains suppressed from active Staircase discovery.

A dirty detached worktree whose basis is an archived cut blocks by default unless its disposition is explicitly selected.

---

### 11.6 Active Git operations

Archive MUST refuse when an affected worktree or ref participates in an active:

```text
merge
rebase
cherry-pick
revert
sequencer operation
incomplete Staircase operation
unresolved provider mutation
unsupported conflicted operation
```

Read-only observation of an unrelated operation is insufficient to block archive.

---

## 12. Branch configuration, reflogs, and tags

### 12.1 Branch configuration

Configuration under `branch.<name>.*` is captured and removed only for branches actually moved by the archive operation.

Configuration for retained unowned branches remains untouched.

Configuration changes MUST be journaled because they may not share a physical transaction with ref updates.

---

### 12.2 Reflogs

Archive guarantees reachability through archive refs.

It does not guarantee byte-for-byte reflog migration.

Archive and restoration refs SHOULD receive descriptive reflog entries where supported.

Recovery MUST NOT depend solely on reflogs.

---

### 12.3 Tags

Unowned tags are never moved.

Immutable snapshot tags remain visible by default.

An active-layout tag may be moved only when:

* The discovery profile classifies it as operation-owned.
* The user explicitly requests inclusion where required.
* Its expected OID is protected by a lease.

---

### 12.4 Remote-tracking refs

Remote-tracking refs are observations and are not moved by default.

They may continue to expose related names through ordinary Git remote-branch commands.

Archive remains local and performs no fetch, push, or remote deletion by default.

---

## 13. Archive transaction

### 13.1 Planning snapshot

After selecting an implicit staircase, archive binds to an immutable discovery snapshot containing:

```text
originating structural key
repository identity
object format
integration context
ordered cuts
materializing refs
operation-owned refs
expected ref OIDs
worktree observations
configuration fingerprint
discovery configuration fingerprint
```

---

### 13.2 Prepublication validation

Immediately before publication, archive MUST verify:

* Every cut still exists and has the expected object type.
* Every operation-owned ref still has its expected OID.
* The integration context remains valid.
* The canonical structural key remains unchanged.
* Relevant worktree state remains compatible.
* No conflicting managed record appeared.
* No committed archive already suppresses the same selected active object unexpectedly.
* Every destination archive ref does not exist.
* Required objects were written successfully.

A mismatch aborts before ref publication.

---

### 13.3 Publication sequence

Direct implicit archive MUST:

1. Generate an archive ID.
2. Construct the snapshot descriptor, lifecycle data, and archive manifest.
3. Write the archive record tree and retention objects.
4. Create a durable operation journal keyed by archive ID.
5. Prepare affected worktrees and branch configuration.
6. Begin a ref transaction.
7. Verify every source ref using expected old OIDs.
8. Create archive record, cut, and owned-ref retention refs.
9. Delete only operation-owned active refs selected by the plan.
10. Commit the ref transaction.
11. Complete configuration and worktree finalization.
12. Mark the journal complete.

No active managed record is created during these steps.

---

### 13.4 Failure and recovery

If failure occurs before the ref transaction commits:

* The active implicit staircase remains authoritative.
* No committed archive entry exists.
* Unreferenced objects are recovery artifacts only.
* No managed record may remain.

If failure occurs after the ref transaction commits:

* The implicit archive snapshot is authoritative.
* The journal MUST permit deterministic completion or rollback.
* Recovery is keyed by archive ID, not lineage ID.
* The command MUST NOT create a managed lineage merely to recover.

---

## 14. Name behavior

### 14.1 Implicit display names are not managed names

The display name of an implicit staircase is derived from discovery.

Storing that display name in an archive snapshot does not turn it into a managed canonical staircase name.

---

### 14.2 Default reservation

A managed archive continues to reserve its canonical managed name by default.

An implicit archive snapshot does not reserve its derived display name by default.

This permits another branch or implicit staircase to later use the same spelling without falsely implying lineage conflict.

---

### 14.3 Optional reservation

An implementation MAY support:

```console
git staircase archive feature --reserve-name
```

This creates an archive-scoped name reservation associated with the archive ID.

It does not create lineage or managed identity.

A reservation collision must be resolved before publication.

---

### 14.4 Duplicate archived display names

Several implicit archive snapshots may share the same display name.

`git staircase list --archived` MUST qualify them with archive IDs or unique archive-ID prefixes.

A bare archived name is accepted only when unique in the archived selection scope.

---

## 15. Listing and inspection

### 15.1 Active listing

```console
git staircase list
```

shows:

* Active managed staircases.
* Active canonical implicit staircases.
* No archived entries.
* No exact candidates suppressed by archive records.

---

### 15.2 Archived listing

```console
git staircase list --archived
```

shows managed archives and implicit archive snapshots.

Example:

```text
feature  3 steps  archived implicit snapshot
  archive ID: 7e86f350-55d3-4af1-8ca4-65a8c739bc13
  origin: implicit@63f4d1c9...
  lineage: none
```

---

### 15.3 All states

```console
git staircase list --all
```

shows active and archived entries with explicit representation and lifecycle markers.

---

### 15.4 Inspection limitations

Inspection of an implicit archive snapshot may show:

```text
archive ID
originating structural key
integration context
ordered cuts
step ordinals
archived refs
retained aliases
reason
archive time
```

It MUST NOT claim:

```text
lineage identity
stable historical step identity
managed canonical name
persistent review mapping
managed structure revision
```

---

## 16. Unarchive semantics

### 16.1 Managed archive

Unarchiving a managed archive retains existing managed lifecycle behavior.

---

### 16.2 Implicit archive default

```console
git staircase unarchive <implicit-archive-selector>
```

restores the archive snapshot as an implicit staircase.

It MUST NOT adopt by default.

The operation attempts to:

1. Recreate archived operation-owned refs.
2. Restore compatible branch configuration.
3. Remove the archive suppression.
4. Remove archive refs after active restoration is safely published.
5. Rediscover the resulting canonical implicit staircase.
6. Confirm that the restored staircase is uniquely selectable.

No lineage or stable step IDs are created.

---

### 16.3 Exact restoration requirement

By default, unarchive of an implicit snapshot MUST prove that the post-operation state will contain one canonical implicit staircase with the archived exact structural identity.

The resulting structural key must equal the originating structural key.

If current integration-context resolution no longer reconstructs that exact structure, default unarchive fails before mutation.

Example:

```text
error: archived implicit structure is not reconstructible under the current integration context

archived integration anchor:
  19aa07d...

current integration anchor:
  61b3e42...

the archived refs have not been restored

available choices:
  unarchive with --accept-current-context
  unarchive with --adopt
```

---

### 16.4 Accepting a changed integration context

An explicit option may permit restoration as a newly interpreted implicit staircase:

```console
git staircase unarchive <selector> --accept-current-context
```

The command MUST show:

```text
archived structural key
new structural key
archived integration context
current integration context
changes to active step decomposition
cuts that are now integrated
```

It must refuse if the resulting candidate is empty, invalid, ambiguous, or dependency-incomplete.

This operation does not create lineage. It restores work as a new implicit interpretation.

---

### 16.5 Explicit adoption during unarchive

The user may request:

```console
git staircase unarchive <selector> --adopt
```

This creates a new managed staircase from the archived exact structure.

For an implicit archive snapshot:

* A new lineage ID is allocated.
* New stable step IDs are allocated.
* The archive ID is retained as provenance.
* The new lineage does not retroactively exist before unarchive.
* The operation MUST NOT claim continuity with a nonexistent pre-archive lineage.

This option can restore branchlessly when supported:

```console
git staircase unarchive <selector> --adopt --branches=none
```

---

### 16.6 Ref destination collisions

For each archived ref destination:

* Missing destination may be recreated.
* Existing destination at the exact expected OID may be treated as already restored when configuration and worktree state are compatible.
* Existing destination at another OID is a collision and MUST NOT be overwritten.
* Existing ref of another type is a collision.
* Prefix conflicts are collisions.
* A generic `--force` MUST NOT bypass these rules.

Sequential layouts may be restored under a new explicit base after complete planning.

---

### 16.7 Configuration collisions

Missing configuration may be restored.

Identical existing configuration may be reused.

Conflicting configuration MUST NOT be merged automatically.

The archive remains intact after a collision failure.

---

### 16.8 Worktrees during unarchive

Unarchive does not reattach worktrees by default.

Reattachment requires an explicit option and is allowed only when:

* The worktree still exists.
* It is detached or otherwise compatible.
* It remains at the expected cut.
* Its index and worktree state are compatible.
* The destination branch follows the same archived ordinal.
* No active operation blocks attachment.

---

### 16.9 Archive removal order

Archive refs and suppression MUST remain in place until active restoration succeeds.

The operation must not first delete the archive and then attempt restoration.

When physical atomicity across refs, configuration, and worktrees is unavailable, a journal MUST make the operation resumable or rollback-capable.

---

## 17. Idempotency

### 17.1 Archive

Archiving an already archived entry is a successful no-op when the archived entry is selected unambiguously.

No new archive ID or lifecycle event is created.

---

### 17.2 Unarchive

Unarchiving an already restored implicit snapshot is a successful no-op only when:

* The archive journal proves successful restoration.
* The expected restored refs still match.
* The resulting active structural key remains the expected key.
* No conflicting active or archived entry exists.

Otherwise the command reports the changed state and performs no silent reinterpretation.

---

### 17.3 Re-archive after restoration

Because an implicit staircase has no lineage continuity, restoring an implicit archive and later archiving it again normally creates a new archive ID.

The new archive MAY record the earlier archive ID as provenance when that relationship is known.

It MUST NOT describe the two archive IDs as one continuing lineage.

Users requiring stable identity across repeated archive and unarchive cycles should explicitly use `--adopt`.

---

## 18. Provider behavior

### 18.1 Local and offline default

Direct implicit archive performs no provider-side or network mutation by default.

It does not:

* Close or modify a pull request.
* Abandon or modify a Gerrit change.
* Delete a remote branch.
* Cancel verification.
* Push archive refs.
* Query remote state merely to complete archive.

---

### 18.2 Provider observations

Recomputable provider observations may be copied into the archive manifest for diagnostics.

They MUST be labeled as observations.

They MUST NOT become durable review associations merely because archive persisted them.

---

### 18.3 Durable provider identity

Preserving an authoritative review association across archive and later restoration requires managed identity.

Such behavior requires explicit adoption:

```console
git staircase archive feature --adopt --preserve-review-associations
```

Archive without that option remains direct and non-adopting.

---

## 19. Verification

Verification evidence for exact immutable commits may remain independently available.

An implicit archive snapshot may reference exact revision verification evidence where the verification store already supports revision-derived identity.

It MUST NOT convert that evidence into lineage-relative verification history.

After unarchive under a changed integration context, evidence whose applicability includes the integration anchor may become stale.

---

## 20. Delete and garbage collection

### 20.1 Delete remains separate

Deleting an active implicit staircase record remains nonsensical because no active managed record exists.

This rule is unchanged:

```console
git staircase delete <active-implicit-selector>
```

must fail unless an explicit ref-deletion operation was requested.

Deleting an implicit archive snapshot is different because an archive record exists:

```console
git staircase delete --archived --archive-id <archive-id>
```

---

### 20.2 Delete diagnostics

Before deleting an implicit archive snapshot, the command MUST report:

```text
archive record refs to be removed
cut-retention refs to be removed
owned-ref retention refs to be removed
objects still reachable elsewhere
objects that may later become unreachable
retained active or remote aliases
```

---

### 20.3 No automatic expiration

An implicit archive snapshot MUST NOT expire merely because it has no lineage.

No automatic retention period is implied.

Explicit retention policies may be supported but must be visible and independently configured.

---

## 21. Transport

Implicit archive snapshots MAY be transported as archive artifacts.

Transport must include:

```text
archive record
cut-retention refs
owned-ref retention refs
referenced objects
archive manifest
required draft snapshots
```

Importing an implicit archive snapshot does not create a managed lineage.

Unarchive in another repository requires:

* Compatible object format.
* Availability of all required objects.
* A compatible repository identity policy.
* Destination-ref collision checks.
* Valid current integration-context resolution, or explicit `--adopt`.

Archive ID collisions with unequal content MUST be treated as corruption or namespace conflict.

---

## 22. Machine output

### 22.1 Direct implicit archive

Machine output MUST clearly state that no adoption occurred:

```json
{
  "command": "archive",
  "source_representation": "implicit",
  "archive_kind": "implicit-snapshot",
  "archive_id": "7e86f350-55d3-4af1-8ca4-65a8c739bc13",
  "archive_record_oid": "<full-typed-oid>",
  "originating_structural_key": "implicit@<full-digest>",
  "adopted": false,
  "lineage_id": null,
  "stable_step_ids": false,
  "moved_refs": [
    {
      "ref": "refs/heads/feature",
      "old_oid": "<full-typed-oid>"
    }
  ],
  "retained_materializing_refs": [],
  "suppression_installed": true,
  "state": "archived"
}
```

---

### 22.2 Managed archive

Managed archive output continues to report:

```json
{
  "source_representation": "managed",
  "archive_kind": "managed-lineage",
  "adopted": false,
  "lineage_id": "<uuid>",
  "stable_step_ids": true
}
```

---

### 22.3 Explicit adoption

When `--adopt` is supplied for an implicit staircase:

```json
{
  "source_representation": "implicit",
  "archive_kind": "managed-lineage",
  "adopted": true,
  "adoption_reason": "explicit-before-archive",
  "lineage_id": "<uuid>",
  "stable_step_ids": true
}
```

---

## 23. Stable error codes

Recommended stable core error codes include:

```text
archive-selector-ambiguous
archive-source-changed
archive-ref-ownership-conflict
archive-ref-collision
archive-worktree-dirty
archive-active-operation
archive-record-conflict
archive-object-missing
archive-record-corrupt
archive-option-requires-management
implicit-unarchive-context-changed
implicit-unarchive-not-discoverable
implicit-unarchive-ambiguous
concurrent-archive-update
```

Provider-specific details remain subordinate to stable core codes.

---

## 24. Required corner-case behavior

| Case                                                                | Required behavior                                                                           |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Unique one-step implicit staircase materialized by one local branch | Archive directly; infer that branch as operation-owned; no adoption                         |
| Complete sequential implicit layout                                 | Archive all recognized primary branches directly; no adoption                               |
| Several raw discovery paths describe the same structure             | Collapse first; create one archive snapshot                                                 |
| Several aliases point at the same cuts                              | Archive only proven operation-owned refs; retain and warn about other aliases               |
| Structure is unique but ownership is ambiguous                      | Archive the snapshot; do not move ambiguous refs; do not report selector ambiguity          |
| Two distinct structures share one name                              | Fail with complete structural-key diagnostics                                               |
| A managed and implicit interpretation are equivalent                | Select the managed staircase only                                                           |
| A managed and implicit interpretation are distinct                  | Treat them as distinct candidates; no hidden preference                                     |
| Detached `HEAD` is the only materializer                            | Create an implicit archive snapshot; retain detached checkout; suppress the exact structure |
| Dirty detached worktree at a cut                                    | Block unless explicit draft disposition is supplied                                         |
| Clean worktree is attached to a branch being moved                  | Detach at the same commit before deleting the branch                                        |
| Dirty worktree is attached to a branch being moved                  | Block by default                                                                            |
| Unowned branch is checked out                                       | Leave it and its worktree untouched; warn                                                   |
| Remote-tracking ref exposes the same commits                        | Leave it; suppress only the exact archived structural key from Staircase discovery          |
| Tag exposes an archived cut                                         | Leave it unless explicitly classified and selected as operation-owned                       |
| Integration provider changes after archive                          | Default unarchive fails if the exact archived key cannot be reconstructed                   |
| User accepts current integration context                            | Restore under `--accept-current-context` and report old and new keys                        |
| User wants stable identity after restoration                        | `unarchive --adopt` creates a new lineage                                                   |
| Destination branch already exists at expected OID                   | Reuse as already restored when other state is compatible                                    |
| Destination branch exists at another OID                            | Fail without overwriting                                                                    |
| Another managed lineage owns a ref archive would move               | Fail with ownership conflict                                                                |
| Staircases overlap only through immutable commits                   | Do not block solely because of commit overlap                                               |
| Ref moves after planning                                            | Abort before publication                                                                    |
| Archive record is written but ref transaction does not commit       | Active implicit state remains authoritative; no managed residue                             |
| Ref transaction commits but configuration finalization fails        | Archive snapshot remains authoritative; journal requires recovery                           |
| Same archived display name is used by several snapshots             | Qualify with archive ID                                                                     |
| Implicit display name is reused after archive                       | Allowed unless an explicit archive-scoped reservation exists                                |
| Same exact structure is still exposed by an unowned alias           | Hide it from active Staircase discovery through exact suppression                           |
| An alias moves to a new commit                                      | New structural key may appear as a new active staircase                                     |
| Archive is invoked with `--no-adopt`                                | Succeed through the direct implicit path                                                    |
| Archive is invoked with `--adopt`                                   | Adopt explicitly, then create a managed archive                                             |
| Plain archive includes `--reason`                                   | Store reason in archive lifecycle without adoption                                          |
| Requested option needs stable step identity                         | Fail before mutation unless `--adopt` was explicitly supplied                               |
| Archived implicit snapshot is deleted                               | Remove archive artifacts only through explicit archived delete                              |
| Restored implicit staircase is archived again                       | Create a new archive ID; do not invent lineage continuity                                   |
| Objects required for restoration are missing                        | Report corruption or incomplete transport; retain all remaining archive refs                |
| Bare repository has no worktrees                                    | Apply the same ref, snapshot, suppression, and collision rules without worktree steps       |
| SHA-1 and SHA-256 repositories contain matching textual OIDs        | Keep archives distinct through repository identity and object format                        |

---

## 25. Examples

### 25.1 One-step implicit staircase

```console
$ git staircase list
feature  1 step  clean  (implicit)

$ git staircase archive feature
Archived implicit staircase 'feature'.
  archive ID: 7e86f350-55d3-4af1-8ca4-65a8c739bc13
  originating structural key: implicit@63f4d1c9...
  adopted: no
  archived branches:
    refs/heads/feature
  retained aliases: none
```

Afterward:

```console
$ git staircase list
No active staircases.

$ git branch
* main

$ git staircase list --archived
feature  1 step  archived  (implicit snapshot)
  archive ID: 7e86f350...
```

---

### 25.2 Sequential implicit staircase

```console
$ git branch
  feature-1
  feature-2
  feature
* main

$ git staircase archive feature
Archived implicit staircase 'feature'.
  archive ID: 43214756-287f-46de-a952-a04de48fe322
  steps: 3
  adopted: no
  archived branches:
    refs/heads/feature-1
    refs/heads/feature-2
    refs/heads/feature
```

No lineage or stable step IDs are created.

---

### 25.3 Unowned alias remains

```console
$ git staircase archive feature
Archived implicit staircase 'feature'.
  archive ID: 95b5eb6e-e7bc-4235-b994-72427e687288
  adopted: no
  archived branches:
    refs/heads/feature

warning: unowned refs still expose archived commits
  refs/heads/feature-copy

the archived structural key is suppressed from active Staircase discovery
```

`git branch` may still show `feature-copy`, but `git staircase list` does not rediscover the exact archived staircase.

---

### 25.4 Explicit managed archive

```console
$ git staircase archive feature --adopt
Adopted implicit staircase 'feature' by explicit request.
  lineage: e6b76cd6-40e1-47a8-8951-c522ed9b03cd

Archived managed staircase 'feature'.
  lineage: e6b76cd6-40e1-47a8-8951-c522ed9b03cd
  archive kind: managed lineage
```

---

### 25.5 Restore as implicit

```console
$ git staircase unarchive --archive-id \
    7e86f350-55d3-4af1-8ca4-65a8c739bc13
Restored implicit staircase 'feature'.
  structural key: implicit@63f4d1c9...
  restored branches:
    refs/heads/feature
  adopted: no
```

---

### 25.6 Restore as managed

```console
$ git staircase unarchive --archive-id \
    7e86f350-55d3-4af1-8ca4-65a8c739bc13 \
    --adopt
Restored archive snapshot as a managed staircase.
  archive provenance: 7e86f350-55d3-4af1-8ca4-65a8c739bc13
  new lineage: 544bf1bc-2067-4aee-8106-bd296f82bd5e
  stable step IDs allocated: 1
```

The new lineage begins at unarchive. It did not exist before archive.

---

## 26. Normative invariants

A conforming implementation MUST preserve all of the following:

1. Default archive accepts both managed and canonical implicit staircases.
2. An implicit staircase is not adopted merely because it is archived.
3. Direct implicit archive allocates an archive ID, not a lineage ID.
4. Direct implicit archive allocates no stable step IDs.
5. No active managed ref is created during direct implicit archive.
6. `--no-adopt` does not block direct implicit archive.
7. Only explicit `--adopt` selects the managed archive path for an implicit staircase.
8. Archive selection operates on canonical staircases, not raw discovery records.
9. Ownership ambiguity is not staircase ambiguity.
10. Ambiguous refs are not moved automatically.
11. Retained aliases do not immediately resurrect the exact archived staircase in active discovery.
12. Suppression is keyed to exact canonical structure, not name or top commit.
13. Managed and implicit archive representations remain distinguishable.
14. Unarchive of an implicit snapshot is non-adopting by default.
15. Default implicit unarchive restores the exact archived structural identity or fails before mutation.
16. Restoring under changed interpretation requires explicit consent.
17. Explicit adoption during unarchive creates a new lineage beginning at unarchive.
18. Archive remains separate from delete.
19. Archive remains local and offline by default.
20. Machine output states unambiguously whether adoption occurred.

Any implementation that automatically adopts an implicit staircase during plain:

```console
git staircase archive <implicit-selector>
```

is nonconforming.
