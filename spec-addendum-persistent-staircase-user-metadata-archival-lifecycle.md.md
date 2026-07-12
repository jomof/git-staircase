
# Addendum I: Persistent Staircase Records, User Metadata, and Archival Lifecycle

## 1. Status and Scope

This addendum modifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**
5. **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**
6. **Addendum H: Worktree Drafts, Index State, and Materialization**
7. Applicable review, verification, and transport provider addenda

It defines:

* Persistent user-facing staircase metadata.
* Overall and per-step descriptions.
* The separation of structural, metadata, and lifecycle revisions.
* Active and archived staircase states.
* Exact archive and unarchive semantics.
* How archiving hides Staircase-owned branches and other active refs from ordinary Git porcelain.
* Branch ownership, worktree, draft, configuration, reflog, collision, provider, transport, and recovery behavior.
* Commands for inspecting and editing persistent state.

This addendum uses **archive** to mean:

> Preserve the staircase and its identity while removing it from active development namespaces and ordinary active listings.

Archive is not equivalent to deletion, review abandonment, remote-branch deletion, or garbage collection.

---

## 2. Design Principles

### 2.1 Persistent state has separate layers

A staircase has several kinds of persistent state with different identity and invalidation semantics:

```text
Structural state
User-facing metadata
Lifecycle state
Provider and operational state
```

These layers must not be collapsed into one undifferentiated revision.

For example:

* Editing the description must not invalidate a successful build of unchanged commits.
* Archiving must not change the staircase body or outcome identity.
* Rebasing must change the structural revision but need not change the description.
* Renaming must not change the staircase lineage.
* Closing an external review must not change the staircase’s commit graph.

---

### 2.2 Archive is reversible deactivation

Archiving preserves:

* Staircase lineage.
* Structural revision.
* Step IDs.
* Cut OIDs.
* Descriptions and labels.
* Review associations.
* Branch restoration information.
* Required object reachability.
* Recovery information selected for retention.

It removes active visibility and active mutability.

---

### 2.3 Archive hides names, not objects

Archiving removes Staircase-owned branch refs from:

```text
refs/heads/
```

and moves their retention refs into a Staircase-specific archive namespace.

This makes them disappear from commands such as:

```console
git branch
git branch --all
git switch <former-branch-name>
```

because Git branch heads live under `refs/heads/`. Tags similarly live under `refs/tags/`.

Archive does not make the underlying Git objects secret or undiscoverable.

Archived refs may still be found through explicitly broad or namespace-aware commands such as:

```console
git for-each-ref
git rev-parse --all
git log --all
git show-ref refs/staircase-archive/...
```

Archive therefore provides **porcelain invisibility**, not confidentiality.

---

### 2.4 Only owned entities are moved

A staircase may be associated with many refs and external entities, but Staircase may automatically move or delete only entities it owns.

OID equality does not establish ownership.

A branch pointing to a staircase cut is not necessarily a Staircase-owned branch.

---

### 2.5 Archive is local by default

Archiving performs no network operation by default.

It does not automatically:

* Delete remote branches.
* Delete remote-tracking refs.
* Close GitHub pull requests.
* Abandon Gerrit changes.
* Disable auto-merge.
* Remove reviewers.
* Cancel presubmit.
* Modify workspace manifests.
* Push archive refs.

Provider-side lifecycle actions require separate, explicit options.

---

## 3. Persistent State Model

### 3.1 Structural revision

The **structural revision** identifies the exact Git structure of the staircase.

It includes, as applicable:

* Lineage ID.
* Integration context.
* Ordered step IDs.
* Cut commit OIDs.
* Active body.
* Structural state.
* Materializing refs.
* Landing and verification policy when those policies affect interpretation.
* Review mappings when those mappings are part of the managed structure.

Its identifier is the OID of the canonical structural descriptor blob:

```text
structure revision ID = <structural descriptor blob OID>
```

A structural revision changes when the staircase’s semantic structure changes.

It does not change when only user-facing metadata or lifecycle state changes.

---

### 3.2 Metadata revision

The **metadata revision** identifies one exact version of user-facing metadata.

It includes:

* Title.
* Description.
* Labels.
* Links.
* Per-step metadata.
* Namespaced extension metadata.
* Creation and modification provenance.

Its identifier is the OID of the canonical metadata blob:

```text
metadata revision ID = <metadata blob OID>
```

---

### 3.3 Lifecycle revision

The **lifecycle revision** identifies one exact version of lifecycle state.

It includes:

* Active or archived state.
* Archive events.
* Unarchive events.
* Archive reason.
* Archived branch-restoration manifest.
* Name reservation.
* Retention policy.
* Lifecycle provenance.

Its identifier is the OID of the canonical lifecycle blob:

```text
lifecycle revision ID = <lifecycle blob OID>
```

---

### 3.4 Record revision

The **record revision** binds one structural revision, one metadata revision, and one lifecycle revision into one coherent staircase record.

The initial representation is a canonical Git tree:

```text
<record tree>
├── structure
├── metadata
├── lifecycle
└── archive-manifest      # archived records only
```

Each entry points to a canonical blob.

The record-tree OID is the:

```text
record revision ID
```

This gives four distinct identities:

```text
lineage ID
    same evolving staircase

structure revision ID
    same exact staircase graph and structural semantics

metadata revision ID
    same exact user-facing metadata

record revision ID
    same exact combined persistent record
```

---

### 3.5 Ref targets

For a named active staircase:

```text
refs/staircases/<name>
    -> current record tree
```

The internal active record ref is:

```text
refs/staircase-state/<lineage-id>/record
    -> current record tree
```

Both refs point to the same record tree.

For an archived staircase:

```text
refs/staircase-archive/<lineage-id>/record
    -> archived record tree
```

The active public and internal record refs do not exist while the staircase is archived.

---

### 3.6 Compatibility with earlier descriptor blobs

A managed staircase created under the earlier blob-only representation remains valid.

It is interpreted as:

```text
structure:
  existing descriptor blob

metadata:
  empty metadata revision

lifecycle:
  active lifecycle revision
```

The first metadata or lifecycle mutation upgrades the record to the tree-based representation.

The original structural descriptor OID remains the structure revision ID.

---

## 4. Identity and Invalidation Rules

| Change                                  | Lineage | Structure revision |                            Metadata revision |              Lifecycle revision |    Record revision |
| --------------------------------------- | ------: | -----------------: | -------------------------------------------: | ------------------------------: | -----------------: |
| Edit description                        |    Same |               Same |                                      Changes |                            Same |            Changes |
| Add label or link                       |    Same |               Same |                                      Changes |                            Same |            Changes |
| Rename canonical staircase ref          |    Same |               Same |                                         Same |                            Same |               Same |
| Amend or rebase a step                  |    Same |            Changes |                                         Same |                            Same |            Changes |
| Split or join steps                     |    Same |            Changes | May change if step metadata is redistributed |                            Same |            Changes |
| Archive                                 |    Same |               Same |                                         Same |                         Changes |            Changes |
| Unarchive                               |    Same |               Same |                                         Same |                         Changes |            Changes |
| Change archive reason                   |    Same |               Same |                                         Same |                         Changes |            Changes |
| Change verification policy              |    Same |            Changes |                                         Same |                            Same |            Changes |
| Change only cached provider observation |    Same |               Same |                                         Same | Same unless persisted in record | Possibly unchanged |

Verification of source content is keyed to the structure revision and exact integration anchor, not the record revision.

A description edit therefore does not make successful source verification stale.

---

## 5. User-Facing Metadata

### 5.1 Overall title

A staircase may have an optional human-facing title:

```text
Authentication redesign
```

The title is distinct from:

* Canonical staircase name.
* Branch-layout base.
* Lineage ID.
* External review title.

The title may contain spaces and ordinary punctuation.

It is not a Git refname.

---

### 5.2 Overall description

A staircase may have an optional multiline description.

The description is intended for:

* Design intent.
* Scope.
* Review guidance.
* Rationale.
* Expected landing order.
* Important caveats.
* Links to related context.

The description is stored as UTF-8 text with canonical LF line endings.

No Markdown dialect is required by the storage model. Clients may render it as Markdown.

---

### 5.3 Per-step metadata

A managed step may have its own:

* Title.
* Description.
* Labels.
* Links.

Per-step metadata is keyed by stable step ID, never by ordinal or branch name.

Example:

```text
step: 77aa...
title: Introduce parser model
description: Adds the internal representation without changing callers.
```

Persistent per-step metadata requires management because an implicit staircase has no stable step IDs.

---

### 5.4 Labels

Labels are user-facing classification values.

Examples:

```text
api-change
needs-design-review
experimental
release-blocker
```

Core label semantics are:

* Case-sensitive.
* Exact-value based.
* Unordered.
* Duplicate-free.
* Not hierarchical unless a convention says otherwise.

The canonical metadata representation sorts labels by encoded value.

Labels must not be interpreted as provider labels unless explicitly mapped by a provider policy.

---

### 5.5 Links

A link contains:

```text
stable link ID
relationship
URI
optional display label
optional description
```

Example relationships include:

```text
issue
design
documentation
incident
review
dependency
custom:<namespace>
```

Links are metadata.

They do not establish provider review identity merely because a URL resembles a pull request or Gerrit change.

---

### 5.6 Namespaced extension fields

Extensions may store additional user-facing metadata using namespaced keys:

```text
example.com/team
provider.github/display-group
company.internal/tracking-id
```

Unknown extension fields must be preserved when the core rewrites metadata.

An extension field must not silently alter core staircase semantics.

Semantic extensions require an explicit capability or policy definition.

---

### 5.7 Creation and modification provenance

Metadata may include:

```text
created-at
created-by
updated-at
updated-by
```

The identity fields are optional opaque strings.

They must not be inferred from commit authorship.

Timestamps use a canonical offset-aware representation.

---

### 5.8 Fields that must not be stored as user metadata

The following are derived or operational state and must not be copied into free-form metadata as authoritative facts:

* Current commit count.
* Current branch names.
* Current verification result.
* Current review approval state.
* Current mergeability.
* Current target OID.
* Current worktree dirtiness.
* Current provider availability.

Clients compute these from authoritative state.

---

## 6. Metadata Commands

Recommended commands include:

```console
git staircase describe auth
git staircase describe auth --edit
git staircase metadata show auth
git staircase metadata edit auth
git staircase metadata set-title auth "Authentication redesign"
git staircase metadata add-label auth api-change
git staircase metadata remove-label auth experimental
git staircase metadata add-link auth \
    --relation design \
    --url <URI>
git staircase metadata show-step auth:2
git staircase metadata edit-step auth:2
```

`describe --edit` should edit only the overall title and description unless explicitly expanded.

---

### 6.1 Editor safety

Metadata editing must:

1. Read the expected current metadata revision.
2. Write the edited metadata to a temporary file.
3. Validate encoding and size.
4. Canonically serialize it.
5. Write the new metadata object.
6. Update the staircase record against the expected old record OID.

If the record changed concurrently, the update must fail rather than overwrite the other edit.

---

### 6.2 No last-write-wins

Concurrent metadata changes use compare-and-swap semantics.

The tool may offer a three-way metadata merge, but it must not silently discard either revision.

---

### 6.3 Size and content constraints

Implementations must impose bounded sizes.

Recommended initial limits are:

```text
title:               4 KiB
description:         1 MiB
individual label:    1 KiB
individual link URI: 16 KiB
complete metadata:   4 MiB
```

Metadata must not contain NUL bytes.

Terminal output must safely escape control characters.

URLs and rendered Markdown are untrusted display content.

---

## 7. Lifecycle States

The core lifecycle states are:

```text
active
archived
```

Future extensions may add states such as:

```text
superseded
tombstoned
```

but they must not overload archived semantics.

---

### 7.1 Active

An active staircase:

* Appears in default `git staircase list`.
* May have a public canonical staircase ref.
* May own primary local branches under `refs/heads/`.
* May be structurally mutated.
* May be uploaded, verified, or landed.
* May have active worktree draft attachments.

---

### 7.2 Archived

An archived staircase:

* Is managed.
* Does not appear in default active listings.
* Has no public ref under `refs/staircases/`.
* Has no Staircase-owned branch under `refs/heads/`.
* Is retained under `refs/staircase-archive/`.
* Preserves its lineage, structure, metadata, and restoration manifest.
* Is read-only for structural and review-mutating operations by default.

---

## 8. Meaning of Archive

The command:

```console
git staircase archive <selector>
```

performs a reversible lifecycle transition:

```text
active -> archived
```

Its required effects are:

1. Preserve the current staircase record.
2. Preserve the commits and metadata needed for restoration.
3. Remove the public active staircase ref.
4. Remove or migrate all owned active branch refs.
5. Remove active internal state refs.
6. Create archived state refs.
7. Capture branch configuration needed for restoration.
8. Record an archive event.
9. Prevent ordinary Staircase mutation until unarchive.
10. Perform no remote mutation unless explicitly requested.

---

## 9. Porcelain Visibility Contract

### 9.1 Guaranteed hidden entities

After successful archive, the following Staircase-owned entities must no longer appear in their ordinary active namespaces:

```text
refs/staircases/<name>
refs/heads/<owned-primary-branch>
refs/heads/<owned-active-alias>
refs/staircase-state/<lineage-id>/...
```

Consequently, Staircase-owned archived branches are absent from ordinary branch listings.

---

### 9.2 Tags

A Staircase-owned tag is moved out of `refs/tags/` only when it is explicitly classified as an **active-layout tag**.

Immutable snapshot tags are historical artifacts and remain visible by default.

An explicit option may archive owned snapshot tags:

```console
git staircase archive auth --include-snapshot-tags
```

Unowned tags are never moved.

---

### 9.3 Remote-tracking refs

Refs under:

```text
refs/remotes/
```

are not moved by default.

They represent observations maintained through remote fetch configuration and may be recreated on the next fetch.

Therefore an archived staircase may still have related names visible in:

```console
git branch --remotes
```

when those are remote-tracking refs.

Suppressing them requires an explicit remote or provider operation.

---

### 9.4 Full-ref visibility

Archived refs remain ordinary Git refs in a custom namespace.

They may be found by commands that intentionally enumerate all refs.

The archive namespace must not be described as hidden storage or a security boundary.

Git’s `hideRefs` configuration applies to ref advertisement and selected revision-walking behavior, particularly for fetch and push services. It is not a general local secrecy mechanism, and Git warns that hidden refs do not necessarily prevent access to their target objects.

---

## 10. Ownership Model

### 10.1 Owned primary branches

Branches recorded as primary branches in the staircase descriptor are owned by the staircase.

They are archived by default.

---

### 10.2 Owned active aliases

A managed staircase may explicitly own additional active aliases.

Each owned alias records:

```text
ref ID
full refname
expected OID
purpose
visibility class
restoration policy
```

Owned active aliases are archived.

---

### 10.3 Unowned aliases

A branch or tag pointing to the same cut but not recorded as owned remains unchanged.

Archive must warn when unowned local branches continue to expose archived cuts:

```text
warning: 2 unowned branches still point into the archived staircase

  refs/heads/debug-auth
  refs/heads/jomo/auth-backup
```

The warning does not imply permission to move them.

---

### 10.4 Ownership is not inferred from names alone

The following are insufficient to establish ownership:

* A matching branch prefix.
* Sequential-looking names.
* A matching branch-layout base.
* A matching OID.
* A matching upstream.
* A common reflog history.
* A provider review association.

Implicit staircases may infer ownership only under the strict recognition rules of Addendum C.

Archiving an implicit staircase automatically adopts it before moving refs.

If ownership remains ambiguous, archive must require explicit ref selection.

---

## 11. Archive Namespace

The canonical archive namespace is:

```text
refs/staircase-archive/<lineage-id>/
```

Required refs include:

```text
refs/staircase-archive/<lineage-id>/record
refs/staircase-archive/<lineage-id>/steps/<step-id>
refs/staircase-archive/<lineage-id>/owned/<ref-id>
```

Where:

* `record` points to the archived record tree.
* Each `steps/<step-id>` ref points to the retained cut commit.
* Each `owned/<ref-id>` ref preserves an owned ref’s exact target.

Archive ref paths use stable IDs rather than former branch names.

The former full refnames are stored inside the archive manifest.

This avoids:

* Ref prefix conflicts.
* Escaping arbitrary refnames into archive paths.
* Dependence on changing branch-layout names.
* Collisions between branches with similar spellings.

---

## 12. Archive Manifest

The archived record contains an immutable archive manifest.

The manifest records:

```text
archive event ID
lineage ID
archive time
optional actor
optional reason
previous active record OID
canonical staircase name
branch-layout profile and base
owned refs
expected source OIDs
archive retention refs
branch configuration snapshots
worktree attachment observations
draft disposition
provider disposition
name-reservation policy
```

For each owned ref:

```text
ref ID
original full refname
object type
original OID
archive refname
ownership class
visibility class
restoration policy
```

The archive manifest is authoritative for unarchive planning.

---

## 13. Archive Preconditions

Before archiving, the implementation must:

1. Resolve the selected staircase.
2. Adopt it if it is implicit.
3. Acquire the lineage operation lock.
4. Read the current record once.
5. Verify expected owned refs and OIDs.
6. Enumerate all worktrees.
7. Inspect attached drafts.
8. Inspect active Git and Staircase operations.
9. Compute the complete archive ref transaction.
10. Validate archive-namespace collisions.
11. Snapshot affected branch configuration.
12. Produce a dry-run plan before mutation when requested.

---

### 13.1 Active operations

Archive must refuse while the staircase participates in:

* An active rebase.
* An active merge.
* A cherry-pick or revert sequence.
* An interrupted restack.
* An incomplete split, join, reorder, or landing operation.
* An unresolved provider mutation.

The operation must first be completed, aborted, or explicitly converted into a durable recovery snapshot.

Archive is not an operation-abort mechanism.

---

### 13.2 Structural state

A managed staircase may be archived while:

* Clean.
* Stale.
* Diverged.
* Incomplete.
* Partially landed.

Archive records the exact state rather than repairing it.

An implicit staircase must be materialized and unambiguous before automatic adoption and archive.

---

## 14. Worktrees

### 14.1 Checked-out owned branches

An owned branch cannot simply be removed while a worktree’s symbolic `HEAD` still names it.

Archive must inspect every linked worktree. Git supports multiple worktrees with independently checked-out branches and detached `HEAD` states.

---

### 14.2 Clean worktrees

By default, a clean worktree attached to an owned branch is detached at the exact current commit during archive.

This changes:

```text
HEAD -> refs/heads/feature
```

to:

```text
HEAD -> <exact feature-tip OID>
```

It does not change:

* Checked-out files.
* Index content.
* Current commit.
* Draft content, because the worktree is clean.

The worktree naturally becomes detached after archive.

---

### 14.3 Dirty worktrees

A dirty worktree attached to the staircase blocks archive by default.

The user must choose one of:

```console
git staircase archive auth --snapshot-drafts
git staircase archive auth --detach-dirty-worktrees
git staircase archive auth --leave-worktrees
```

Semantics:

#### `--snapshot-drafts`

Capture each affected draft losslessly under Addendum H, detach the worktree, and archive its attachment.

#### `--detach-dirty-worktrees`

Detach at the same exact OID while preserving the current index and filesystem state.

The live draft remains in the worktree but becomes attached to an archived lineage. Structural mutation remains prohibited until the draft is detached, snapshotted, discarded, or the staircase is unarchived.

#### `--leave-worktrees`

Valid only when no affected worktree is attached to a branch that must be removed.

It is not a way to leave a broken symbolic `HEAD`.

---

### 14.4 Conflicted drafts

A worktree with an unmerged index blocks archive unless its owning Git operation is first resolved or an explicitly supported operation snapshot is created.

---

### 14.5 Worktree restoration

The archive manifest records former worktree attachments for diagnostic and optional restoration purposes.

Unarchive does not reattach worktrees by default.

A worktree may have changed purpose while the staircase was archived.

---

## 15. Branch Configuration

Git stores branch-specific configuration under keys such as:

```text
branch.<name>.remote
branch.<name>.merge
branch.<name>.pushRemote
branch.<name>.rebase
branch.<name>.description
```

Git branch descriptions are configuration associated with branch names.

---

### 15.1 Capture

Archive captures every configuration entry belonging to an owned branch.

The snapshot preserves:

* Full key.
* Every value.
* Value order where the configuration permits multiple values.
* Original branch short name.

---

### 15.2 Removal

After branch refs have been archived, their active:

```text
branch.<name>.*
```

configuration sections are removed.

This prevents a later unrelated branch reusing the same name from inheriting stale:

* Upstream configuration.
* Push destination.
* Rebase policy.
* Description.
* Tool-specific branch settings.

---

### 15.3 Configuration transaction limits

Ref updates and Git configuration updates do not necessarily share one physical transaction.

Archive must therefore use:

* A repository-level operation lock.
* A durable journal.
* A complete configuration snapshot.
* Resume and rollback behavior.
* Explicit interrupted-state reporting.

It must not claim physical atomicity across worktree `HEAD`, refs, and configuration.

---

## 16. Reflogs

### 16.1 Reachability guarantee

Archive guarantees preservation of current branch tips and all objects reachable from retained archive refs.

It does not guarantee byte-for-byte migration of every branch reflog.

---

### 16.2 No direct reflog-file manipulation

The implementation must not assume reflogs are stored as ordinary files under:

```text
.git/logs/
```

It must use supported Git ref interfaces.

---

### 16.3 Archived reflogs

Implementations may create reflogs for archive refs with entries such as:

```text
staircase: archived refs/heads/feature-2
```

`git update-ref` supports grouped ref changes and explicit reflog creation.

---

### 16.4 Unarchive reflogs

Restored branches receive a new reflog entry:

```text
staircase: restored from archive <archive-event-id>
```

Previous branch-reflog continuity is best-effort unless an implementation has a ref-backend-supported migration mechanism.

Staircase recovery must rely on archive refs and manifests, not solely on reflogs, which may expire.

---

## 17. Archive Ref Transaction

All ref changes must be computed before mutation.

The reference transaction includes, as applicable:

### Creates

```text
refs/staircase-archive/<lineage>/record
refs/staircase-archive/<lineage>/steps/<step-id>
refs/staircase-archive/<lineage>/owned/<ref-id>
```

### Deletes

```text
refs/staircases/<canonical-name>
refs/staircase-state/<lineage>/record
refs/staircase-state/<lineage>/steps/<step-id>
refs/heads/<owned-branch>
other owned active refs
```

Every update is protected by an expected old OID or nonexistence condition.

Git’s ref transaction plumbing can group create, update, delete, and verify operations.

---

### 17.1 Publication order

The logical archive sequence is:

1. Write the new lifecycle blob.
2. Write the archive manifest.
3. Write the archived record tree.
4. Create the operation journal.
5. Prepare affected worktrees.
6. Prepare the configuration change.
7. Begin the ref transaction.
8. Verify all active refs.
9. Create archive refs.
10. Delete active refs.
11. Commit the ref transaction.
12. Remove active branch configuration.
13. Finalize worktree state.
14. Mark the operation complete.
15. Remove temporary recovery state according to policy.

A crash at any phase must leave sufficient state to resume or roll back deterministically.

---

### 17.2 No temporary branch namespace for ordinary ref permutations

When one ref transaction can express the complete migration, branches need not be renamed through temporary `refs/heads/` names.

Archive refs are created directly from the expected source OIDs.

---

## 18. Archive Name Reservation

### 18.1 Canonical staircase name

The canonical staircase name is reserved by default while archived.

For example, archiving:

```text
refs/staircases/auth
```

removes that active ref but preserves the name `auth` in the archive manifest.

A later attempt to create another staircase named `auth` must fail with:

```text
error: staircase name 'auth' is reserved by an archived staircase
```

Recommended remedies are:

```console
git staircase unarchive auth
git staircase create auth-v2
git staircase archive release-name auth
```

---

### 18.2 Releasing a name

A name may be released explicitly:

```console
git staircase archive release-name auth
```

Releasing the name:

* Does not delete the archived staircase.
* Does not change lineage.
* Removes the name reservation.
* Records a lifecycle event.
* Means unarchive may require a new canonical name.

---

### 18.3 Branch names

Archived branch names are soft restoration preferences, not enforceable reservations.

Ordinary Git commands may create new branches with the former names.

Unarchive must therefore perform collision checks.

---

## 19. Provider and Remote State

### 19.1 Default provider disposition

The default archive provider disposition is:

```text
keep
```

This means:

* Preserve review associations.
* Preserve cached provider state.
* Perform no network request.
* Do not close, abandon, merge, or delete anything remotely.

---

### 19.2 Explicit provider actions

A provider may define explicit actions such as:

```console
git staircase archive auth --github=close
git staircase archive auth --gerrit=abandon
git staircase archive auth --remote-branches=delete
```

Such actions:

* Are separate planned remote mutations.
* Require provider readiness and authentication.
* Require explicit user intent.
* Must be journaled.
* Must reconcile uncertain outcomes.
* Must not be implied by the word `archive`.

---

### 19.3 Remote-tracking refs

Archive does not remove:

```text
refs/remotes/<remote>/...
```

unless an explicit provider or remote action does so.

Removing a remote-tracking ref without changing fetch configuration may only hide it until the next fetch.

---

### 19.4 Provider revalidation

Unarchive does not contact providers by default.

Provider state is marked with its observation age.

The next review or verification command may revalidate it.

---

## 20. Operations on Archived Staircases

The following operations are allowed:

```text
show
list --archived
inspect metadata
edit overall metadata
add or remove labels
inspect structure
compute diffs
verify immutable committed revisions
export
unarchive
delete explicitly
```

The following are prohibited by default:

```text
split
join
drop
reorder
materialize a draft
rebase
restack
upload
land
create or update reviews
change active branch layout
```

A structurally mutating command may offer:

```text
error: staircase is archived; unarchive it before mutation
```

It must not silently unarchive as a side effect.

Metadata edits may be made while archived because they do not reactivate development refs.

---

## 21. Unarchive

The command:

```console
git staircase unarchive <selector>
```

performs:

```text
archived -> active
```

It restores active state according to the archive manifest and current repository conditions.

---

### 21.1 Selection

An archived staircase may be selected by:

* Reserved former canonical name.
* Lineage ID.
* Archived record OID.
* Explicit archive event ID.
* A unique archived title search result when a command explicitly supports search.

A title alone is not a canonical identity.

---

### 21.2 Default restoration

Default unarchive attempts to restore:

* The former canonical staircase name.
* Active internal state refs.
* All owned primary branches.
* All owned active aliases.
* Branch configuration.
* Sequential layout policy.
* Current cut OIDs.
* Archived drafts as attachments or snapshots, according to their disposition.

It does not reattach worktrees automatically.

---

## 22. Unarchive Collision Rules

### 22.1 Canonical staircase-name collision

If:

```text
refs/staircases/<former-name>
```

belongs to another lineage, unarchive fails.

The user may provide another name:

```console
git staircase unarchive <id> --name auth-restored
```

---

### 22.2 Missing destination branch

If a required branch ref does not exist, it is created at the archived OID.

---

### 22.3 Existing branch at the same OID

An existing unowned branch at the expected OID is still an independent entity.

Unarchive fails unless the user explicitly allows adoption:

```console
git staircase unarchive auth --adopt-existing-branches
```

Before adoption, Staircase must check:

* Worktree use.
* Branch configuration.
* Existing ownership.
* Remote tracking.
* Provider associations.

---

### 22.4 Existing branch at another OID

Unarchive must not overwrite it.

Available resolutions include:

```console
git staircase unarchive auth --branch-base auth-restored
git staircase unarchive auth --branches=rename
git staircase unarchive auth --branches=none
```

---

### 22.5 Restore with no local branches

The option:

```console
git staircase unarchive auth --branches=none
```

reactivates the managed staircase using metadata-backed cuts and internal step refs without recreating primary local branches.

The staircase becomes active but branchless.

A later layout-normalization command may recreate branches.

---

### 22.6 Sequential-layout renaming

For staircases using Addendum C, the user may select a new branch-layout base:

```console
git staircase unarchive auth --branch-base auth-v2
```

The complete final layout is computed before any branch is created.

---

### 22.7 Configuration collision

If destination:

```text
branch.<name>.*
```

configuration exists and is not owned by the archived staircase, unarchive fails.

Configuration is not merged automatically.

---

## 23. Worktree Reattachment

Unarchive does not change worktree attachment by default.

An explicit option may request restoration:

```console
git staircase unarchive auth --reattach-worktrees
```

A worktree is reattached only when:

* It still exists.
* It remains detached.
* Its current `HEAD` equals the archived step OID.
* Its index and worktree state are compatible.
* It has not been attached to another staircase.
* The destination branch is not checked out incompatibly elsewhere.
* Reattachment will not switch the conceptual step.

Otherwise, the worktree remains unchanged and a diagnostic is reported.

---

## 24. Unarchive Transaction

The logical unarchive sequence is:

1. Resolve the archived record.
2. Acquire the lineage and repository locks.
3. Compute canonical name and branch destinations.
4. Check every ref and configuration collision.
5. Build the active lifecycle revision.
6. Build the active record tree.
7. Create an operation journal.
8. Begin one ref transaction.
9. Verify archived refs and destination nonexistence.
10. Create active state refs.
11. Create public staircase ref.
12. Create or adopt active owned branches.
13. Delete archived refs.
14. Commit the transaction.
15. Restore branch configuration.
16. Optionally reattach eligible worktrees.
17. Finalize draft attachments.
18. Complete the operation journal.

Failure after partial non-ref work must be resumable or reversible.

---

## 25. Lifecycle Events

The lifecycle revision retains an ordered event history.

Core event kinds include:

```text
created
adopted
archived
unarchived
renamed
name-released
deleted
```

An archive event contains:

```text
event ID
time
actor, if available
record OID before archive
record OID after archive
canonical name
reason
provider disposition
draft disposition
```

An unarchive event contains:

```text
event ID
time
actor, if available
archived record OID
active record OID
restored canonical name
branch restoration policy
collision resolutions
```

Lifecycle event history is user-facing audit information.

It does not change the structure revision.

---

## 26. Idempotency

### 26.1 Archive of an archived staircase

Archiving an already archived staircase is a successful no-op when the selected lineage is unambiguous.

It does not create another archive event.

Changing the reason requires an explicit metadata or lifecycle edit.

---

### 26.2 Unarchive of an active staircase

Unarchiving an already active staircase is a successful no-op when the selected lineage is unambiguous.

---

### 26.3 Interrupted operations

If an archive or unarchive operation journal already exists, a new lifecycle command must first:

* Resume it.
* Abort it.
* Reconcile it.
* Or explicitly discard it after proof that no state would be lost.

It must not begin a second overlapping lifecycle transaction.

---

## 27. Transport

Archived staircase state is not assumed to travel under ordinary branch or tag refspecs.

An explicit Staircase transport operation may include:

```text
refs/staircase-archive/*
```

Example:

```console
git staircase push --include-archived
git staircase fetch --include-archived
```

Transport must include:

* Archived record refs.
* Step-retention refs.
* Owned-ref retention refs.
* Referenced structural and metadata objects.
* Required cut commits and ancestry.

Name and lineage collisions are resolved under the same rules as active staircase transport.

---

## 28. Garbage Collection and Reachability

Archive refs must preserve every object required to:

* Inspect the archived staircase.
* Restore every retained cut.
* Recreate owned branches.
* Read persistent metadata.
* Interpret provider associations.
* Restore retained draft snapshots.
* Resume explicitly retained recovery state.

Textual OIDs inside an archive manifest are not sufficient object-retention roots by themselves.

Required commits must be reachable through archive refs.

Archive does not promise retention of:

* Expired reflog-only commits.
* Unselected discarded drafts.
* Temporary provider artifacts.
* Unowned refs.
* Recovery objects explicitly excluded by retention policy.

---

## 29. Delete Is Separate

Deleting an archived staircase requires a separate explicit command:

```console
git staircase delete --archived auth
```

Deletion may remove the final refs preserving:

* Structural descriptors.
* Metadata.
* Cut commits.
* Recovery state.

The command must explain what remains reachable elsewhere.

Archive must never degrade into deletion merely because the staircase has remained unused for a long period.

No automatic expiration policy is part of this addendum.

---

## 30. Listing and Display

### 30.1 Default listing

```console
git staircase list
```

shows active staircases only.

---

### 30.2 Archived listing

```console
git staircase list --archived
```

may show:

```text
auth
  state: archived
  title: Authentication redesign
  archived: 2027-04-18
  steps: 4
  reason: Superseded by the unified identity project
```

---

### 30.3 All lifecycle states

```console
git staircase list --all
```

shows active and archived staircases with explicit state markers.

---

### 30.4 Standard Git listing

After archive:

```console
git branch
```

must not show the archived Staircase-owned local branches.

The command:

```console
git for-each-ref refs/staircase-archive/
```

may still show the archive implementation refs.

This distinction is intentional.

---

## 31. Commands

Recommended commands include:

```console
git staircase describe <selector>
git staircase describe <selector> --edit

git staircase metadata show <selector>
git staircase metadata edit <selector>
git staircase metadata show-step <step>
git staircase metadata edit-step <step>

git staircase archive <selector>
git staircase archive <selector> --reason <text>
git staircase archive <selector> --dry-run
git staircase archive <selector> --snapshot-drafts
git staircase archive <selector> --detach-dirty-worktrees
git staircase archive release-name <selector>

git staircase unarchive <selector>
git staircase unarchive <selector> --name <new-name>
git staircase unarchive <selector> --branch-base <base>
git staircase unarchive <selector> --branches=exact
git staircase unarchive <selector> --branches=rename
git staircase unarchive <selector> --branches=none
git staircase unarchive <selector> --adopt-existing-branches
git staircase unarchive <selector> --reattach-worktrees

git staircase list --archived
git staircase list --all
git staircase show --archived <selector>
```

---

## 32. Example: Basic Archive

Before:

```text
refs/staircases/auth
refs/heads/auth-1
refs/heads/auth-2
refs/heads/auth
```

Command:

```console
git staircase archive auth \
    --reason "Paused pending API direction"
```

After:

```text
refs/staircase-archive/<lineage>/record
refs/staircase-archive/<lineage>/steps/<step-1-id>
refs/staircase-archive/<lineage>/steps/<step-2-id>
refs/staircase-archive/<lineage>/steps/<step-3-id>
refs/staircase-archive/<lineage>/owned/<branch-1-id>
refs/staircase-archive/<lineage>/owned/<branch-2-id>
refs/staircase-archive/<lineage>/owned/<branch-3-id>
```

The following no longer exist:

```text
refs/staircases/auth
refs/heads/auth-1
refs/heads/auth-2
refs/heads/auth
```

Result:

```console
$ git branch
# auth branches are absent

$ git staircase list
# auth is absent

$ git staircase list --archived
auth  3 steps  archived
```

---

## 33. Example: Archive While on the Tip Branch

Before:

```text
HEAD -> refs/heads/auth
refs/heads/auth -> C3
```

The worktree is clean.

Archive changes the worktree to:

```text
HEAD -> C3
```

and moves staircase retention to:

```text
refs/staircase-archive/<lineage>/...
```

The files and index remain unchanged.

The user now sees a detached `HEAD`, which is valid and expected.

---

## 34. Example: Unowned Alias

Before archive:

```text
refs/heads/auth       -> C3   owned
refs/heads/auth-debug -> C3   unowned
```

After archive:

```text
refs/heads/auth       absent
refs/heads/auth-debug -> C3
```

The command reports:

```text
archived staircase 'auth'

warning:
  unowned branch refs/heads/auth-debug still points to the archived tip
```

Staircase does not move it.

---

## 35. Example: Unarchive Collision

Archived restoration plan:

```text
refs/heads/auth-1 -> C1
refs/heads/auth   -> C2
```

Current repository:

```text
refs/heads/auth -> X
```

where `X != C2`.

Default unarchive fails:

```text
error: cannot restore refs/heads/auth

expected destination:
  absent

actual:
  X

the existing branch is not owned by the archived staircase
```

The user may choose:

```console
git staircase unarchive auth --branch-base auth-restored
```

producing:

```text
refs/heads/auth-restored-1 -> C1
refs/heads/auth-restored   -> C2
```

---

## 36. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 36.1 Metadata is not structure

Editing descriptions, labels, or links does not change structural identity.

### 36.2 Lifecycle is not structure

Archive and unarchive do not rewrite staircase commits or cuts.

### 36.3 Archived owned branches leave `refs/heads/`

They must no longer appear as local branches.

### 36.4 Archive is not secrecy

Custom archive refs remain discoverable through explicit full-ref enumeration.

### 36.5 Only owned refs are moved

Name similarity and OID equality do not grant ownership.

### 36.6 Archive preserves reachability

Every object required for deterministic restoration has a real Git reachability root.

### 36.7 Archive is local and offline by default

External reviews and remote branches are unchanged unless explicitly requested.

### 36.8 Checked-out branches are handled deliberately

No worktree is left with a broken symbolic `HEAD`.

### 36.9 Dirty worktrees are not discarded

They block archive or receive an explicit preservation disposition.

### 36.10 Branch configuration is preserved without leaking

Owned branch configuration is captured and removed from active branch-name sections.

### 36.11 Unarchive never overwrites an unowned branch

Even a same-OID branch requires explicit adoption.

### 36.12 Archived names are reserved by default

Name reuse requires unarchive, rename, or explicit release.

### 36.13 Structural mutation requires active lifecycle

Archived staircases must be unarchived before ordinary development mutation.

### 36.14 Archive is reversible; delete is separate

No retention ref is removed merely because the lifecycle becomes archived.

### 36.15 Persistent state updates are lease-protected

Metadata, archive, and unarchive operations compare against expected old record and ref values.

---

## 37. Summary

Persistent staircase state is divided into:

```text
lineage
    identity of the evolving staircase

structure
    commits, cuts, steps, target, and structural policy

metadata
    title, description, labels, links, and per-step explanations

lifecycle
    active or archived state and restoration history

record
    one exact binding of structure, metadata, and lifecycle
```

Archiving moves Staircase-owned active refs out of ordinary Git branch and tag namespaces while preserving the exact objects and metadata required for restoration.

It does not delete source history, rewrite commits, close external reviews, or conceal objects from deliberate low-level ref inspection.

The governing rules are:

> Metadata explains the staircase. Structure defines it. Lifecycle determines whether it is active.

and:

> Archive removes active names, not history.
