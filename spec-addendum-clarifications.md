# Addendum J: Canonical Vocabulary, Record Concurrency, and Machine-Output Semantics

## 1. Status and Scope

This addendum modifies and clarifies the complete Git Staircase specification corpus.

It supersedes conflicting or ambiguous terminology in earlier documents, including:

* The proposed `track` alias for `adopt`.
* Use of “tracked,” “adopted,” and “managed” as interchangeable staircase states.
* Use of “provider” and “adapter” as interchangeable extension concepts.
* Ambiguous use of “integration boundary,” “target,” and “staircase revision.”
* Optional automatic merging of concurrent metadata edits.
* Underspecified empty output from `git staircase list`.
* Inconsistent top-level and provider-qualified upload command examples.

Backward compatibility with removed aliases and deprecated terminology is not required.

---

## 2. Normative Language

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

Where an earlier addendum uses a deprecated synonym, implementations and future specification text MUST interpret it according to the canonical terminology in this addendum.

New implementations MUST expose only the canonical command names unless a later addendum explicitly introduces another command with distinct semantics.

---

## 3. Canonical Staircase States and Transitions

### 3.1 Implicit staircase

An **implicit staircase** is a staircase reconstructed from current Git structure without a persistent staircase record.

“Discovered staircase” MAY describe how the staircase was found, but it is not a separate lifecycle state.

Canonical state name:

```text
implicit
```

---

### 3.2 Managed staircase

A **managed staircase** is a staircase with a persistent lineage ID and persistent staircase record.

Canonical state name:

```text
managed
```

The following state names are removed:

```text
tracked staircase
adopted staircase
```

The word “adopted” MAY appear only as the past tense of the `adopt` operation or as the name of a lifecycle event.

For example:

```text
The implicit staircase was adopted.
```

It MUST NOT be used as the persistent state name.

---

### 3.3 Adoption

The only canonical operation that converts an implicit staircase into a managed staircase is:

```console
git staircase adopt <selector>
```

The command:

```console
git staircase track
```

is removed and MUST NOT be implemented as an alias.

Automatic adoption, as defined by Addendum A, performs the same semantic transition as an explicit `adopt` command.

The transition is:

```text
implicit -> managed
```

Adoption does not mean:

* Creating commits.
* Creating a review.
* Attaching a worktree draft.
* Enabling a branch-layout policy.
* Archiving the staircase.

Those are separate operations.

---

### 3.4 Active and archived

A managed staircase has one lifecycle state:

```text
active
archived
```

The canonical lifecycle commands are:

```console
git staircase archive <selector>
git staircase unarchive <selector>
```

The following are not aliases for `unarchive`:

```text
restore
reopen
activate
unhide
```

Those words MAY be used descriptively, but MUST NOT identify the lifecycle command.

---

## 4. Canonical Extension Terminology

### 4.1 Provider

A **provider** is an installed implementation that exposes one or more typed capabilities.

Examples include:

```text
repo
gerrit
github
```

The term **adapter** is removed from the normative vocabulary.

Earlier references to:

```text
review adapter
verification adapter
transport adapter
```

mean:

```text
provider bound to the review capability
provider bound to the verification capability
provider bound to the transport capability
```

---

### 4.2 Capability binding

A **capability binding** associates one provider with one typed capability in a workspace, repository, worktree, or invocation.

Examples:

```text
workspace = repo
review = gerrit
verification = gerrit
review = github
```

A provider profile remains shorthand that creates several capability bindings.

A profile is not itself a provider.

---

## 5. Canonical Integration Terminology

### 5.1 Integration context

An **integration context** is the complete structure used to determine which history is already integrated.

It may include:

```text
integration anchor
integration set
symbolic integration target
provenance
resolution mode
```

The term **integration boundary** is removed from normative prose.

Earlier references to an integration boundary mean the integration set contained in an integration context.

---

### 5.2 Integration anchor

An **integration anchor** is an exact commit OID.

For the ordinary single-anchor case, the integration set is:

[
U = \operatorname{Ancestors}(T)
]

where (T) is the integration anchor.

---

### 5.3 Integration set

The **integration set** is the ancestor-closed set of commits treated as already integrated.

The integration set, rather than a branch name, is used in the formal staircase definition.

---

### 5.4 Symbolic integration target

A **symbolic integration target** is an optional moving ref or provider-resolved locator expressing future target intent.

Example:

```text
refs/remotes/origin/main
```

It is distinct from the exact integration anchor used by a particular operation.

---

### 5.5 Review destination

A **review destination** is the provider-specific repository and branch, or equivalent target, to which review work is submitted.

A review destination is not automatically the integration context.

---

### 5.6 Use of “target”

The unqualified word **target** MUST NOT be used in normative data models or machine-readable fields.

Specifications and implementations MUST use one of:

```text
integration anchor
symbolic integration target
review destination
landing destination
rewrite destination
```

Human-facing output MAY use “target” only when the surrounding label makes the meaning unambiguous.

---

### 5.7 `--onto`

The option:

```console
--onto <commit-ish>
```

supplies or overrides the integration anchor for the current operation unless the command explicitly documents a more specialized rewrite meaning.

The value is resolved immediately to a full commit OID.

If the input is a ref, the implementation MAY retain the full refname as symbolic provenance, but the exact OID governs the operation.

---

## 6. Canonical Revision Terminology

Addendum I separates structural, metadata, lifecycle, and combined persistent state.

The generic term **staircase revision** is therefore removed from normative data models.

Implementations MUST use the exact revision type intended.

---

### 6.1 Structure revision

A **structure revision** identifies one exact staircase structure.

Its identifier is:

```text
structure revision OID
```

It changes when commits, cuts, steps, integration context, or structural policy changes.

---

### 6.2 Metadata revision

A **metadata revision** identifies one exact user-facing metadata blob.

Its identifier is:

```text
metadata revision OID
```

---

### 6.3 Lifecycle revision

A **lifecycle revision** identifies one exact lifecycle blob.

Its identifier is:

```text
lifecycle revision OID
```

---

### 6.4 Record revision

A **record revision** binds one structure revision, one metadata revision, and one lifecycle revision.

Its identifier is:

```text
record revision OID
```

The record revision OID is the compare-and-swap unit for persistent staircase updates.

---

### 6.5 Staircase record

A **staircase record** is the persistent record tree described by Addendum I.

The earlier phrase **staircase descriptor** refers only to the structure component after the tree-based record format has been introduced.

The terms have the following exact meanings:

```text
structure descriptor
    Canonical structural blob.

staircase record
    Canonical tree binding structure, metadata, and lifecycle.

record ref
    Ref pointing to the current staircase record.
```

---

### 6.6 Exact command output

Commands displaying identifiers MUST label their types.

The following output is nonconforming:

```text
revision: abc123
```

Conforming output uses an exact label:

```text
structure revision: abc123
metadata revision:  7de912
record revision:    4aa870
```

---

## 7. Canonical Core Commands

The following command names have distinct meanings and MUST NOT be implemented as aliases for one another.

| Command       | Canonical meaning                                                      |
| ------------- | ---------------------------------------------------------------------- |
| `discover`    | Find staircase candidates from current structure.                      |
| `adopt`       | Convert an implicit staircase into a managed staircase.                |
| `split`       | Insert a step boundary.                                                |
| `join`        | Remove a boundary between adjacent steps.                              |
| `drop`        | Remove one step’s changes from the staircase.                          |
| `delete`      | Remove the managed staircase record and selected owned refs.           |
| `rebase`      | Replay the complete selected staircase onto a new integration anchor.  |
| `restack`     | Repair upper-step dependencies after a lower step changed.             |
| `reorder`     | Change the order of existing steps.                                    |
| `materialize` | Convert staged draft state into commits and staircase structure.       |
| `archive`     | Move an active managed staircase into archived lifecycle state.        |
| `unarchive`   | Move an archived staircase back into active lifecycle state.           |
| `normalize`   | Repair a derived layout without changing intended staircase structure. |
| `verify`      | Evaluate a verification subject against a verification profile.        |
| `land`        | Integrate reviewed work into its landing destination.                  |

The following proposed synonyms are removed:

```text
track             for adopt
indent            for split
unindent          for join
restore           for unarchive
remove            for drop or delete
sync              for rebase or restack
commit-draft      for materialize
```

A future command MAY use one of these words only if it defines a genuinely different operation.

---

## 8. Review Publication and Staircase Transport

The specification previously used both:

```console
git staircase upload
git staircase review upload
```

for review publication.

The canonical review-publication command is:

```console
git staircase review upload <selector>
```

The top-level command:

```console
git staircase upload
```

is removed.

---

### 8.1 Review creation

Creating a provider review without assuming that one already exists uses:

```console
git staircase review create <selector>
```

A provider MAY combine creation and initial publication internally, but command semantics remain distinct:

```text
review create
    Establish one or more remote review identities.

review upload
    Publish exact commit revisions to new or existing reviews.
```

A high-level command MAY perform both when explicitly documented.

---

### 8.2 Staircase transport

Transporting Staircase refs, records, archive state, or metadata uses:

```console
git staircase push
git staircase fetch
```

These commands are not review publication commands.

For example:

```console
git staircase push --include-archived
```

transports Staircase state, not GitHub pull-request updates or Gerrit patch sets.

---

### 8.3 Review state

The canonical command for provider review state is:

```console
git staircase review status <selector>
```

This is distinct from:

```console
git staircase status <selector>
```

which reports local staircase, layout, worktree, and lifecycle state.

---

## 9. Strict Compare-and-Swap for Persistent Record Updates

### 9.1 CAS unit

Every persistent staircase mutation MUST use the current **record revision OID** as its compare-and-swap unit.

A metadata command MUST NOT compare only the previous metadata revision OID.

This rule prevents a metadata edit from overwriting a concurrent:

* Rebase.
* Split.
* Join.
* Archive.
* Unarchive.
* Policy update.
* Provider-association update.
* Independent metadata edit.

---

### 9.2 Active named staircase

For an active named staircase, the operation reads and verifies:

```text
refs/staircases/<name>
refs/staircase-state/<lineage-id>/record
```

Both refs MUST point to the same expected record revision OID.

The update transaction MUST move both refs to the same new record revision OID.

If either ref differs, the operation fails as a consistency or concurrency error.

---

### 9.3 Active unnamed staircase

For an active unnamed staircase, the CAS ref is:

```text
refs/staircase-state/<lineage-id>/record
```

---

### 9.4 Archived staircase

For an archived staircase, the CAS ref is:

```text
refs/staircase-archive/<lineage-id>/record
```

Metadata editing while archived updates the archived record ref.

It MUST NOT recreate active refs or unarchive the staircase.

---

### 9.5 Metadata mutation algorithm

A metadata-only mutation MUST perform the following steps:

1. Resolve the staircase to one lineage and one current record ref set.
2. Read the current record revision OID exactly once for the edit session.
3. Read its structure, metadata, and lifecycle components.
4. Record the current record revision OID as `expected-record`.
5. Obtain the user’s metadata change.
6. Validate and canonically serialize the new metadata.
7. Write the new metadata blob.
8. Construct a new record tree containing:

   * The unchanged structure component.
   * The new metadata component.
   * The unchanged lifecycle component.
9. Write the new record tree.
10. Begin a ref transaction.
11. Verify every applicable record ref still equals `expected-record`.
12. Update every applicable record ref to the new record revision OID.
13. Commit the transaction.

The operation MUST NOT re-read the latest record and silently apply the edit to that newer record.

---

### 9.6 Concurrent update failure

If any applicable record ref no longer points to `expected-record`, the command MUST fail.

The stable machine-readable error code is:

```text
concurrent-record-update
```

Recommended human output is:

```text
error: concurrent staircase update detected

expected record:
  <expected-record-oid>

current record:
  <current-record-oid>

the edit was not applied
```

A metadata command MAY additionally state:

```text
metadata edit conflicted with a concurrent staircase update
```

The shorter message:

```text
error: concurrent metadata edit detected
```

is insufficient when the concurrent change was structural or lifecycle-related.

---

### 9.7 No automatic merge

Core commands MUST NOT automatically merge concurrent metadata changes.

The earlier optional three-way metadata merge is removed from the normative core behavior.

A future explicit command MAY provide a user-visible metadata merge workflow, but it MUST:

* Be invoked explicitly.
* Show or expose the merge result before publication.
* Use the latest record revision as a new CAS base.
* Preserve unknown namespaced metadata fields.
* Fail on unresolved conflicts.
* Never be delegated implicitly to a workspace or review provider.

Metadata merging is a local Staircase record concern, not a provider capability.

---

### 9.8 Retry behavior

A command MAY offer:

```console
--retry-edit
```

or an equivalent explicit workflow.

A retry MUST:

1. Reload the current record.
2. Reopen or reconstruct the edit against the new metadata.
3. Require a new user decision.
4. Use the newly loaded record revision as the CAS base.

It MUST NOT blindly replay the previous serialized metadata over the newer record.

---

### 9.9 Revision effects

A successful metadata-only update has these identity effects:

```text
lineage ID:              unchanged
structure revision OID:  unchanged
metadata revision OID:   changed
lifecycle revision OID:  unchanged
record revision OID:     changed
```

A failed CAS changes none of them.

---

## 10. Other Persistent Mutations

The strict record-level CAS rule applies to all persistent record mutations, including:

* Structural mutations.
* Metadata mutations.
* Lifecycle mutations.
* Policy changes.
* Provider-association changes.
* Name reservation changes.
* Draft attachment persistence.

Operations MAY update additional refs, but they MUST verify the current record revision before publishing a new record.

No persistent command uses last-write-wins behavior.

---

## 11. List Output Modes

`git staircase list` supports three output modes:

```text
human
porcelain
json
```

The default mode is `human`.

The options are:

```console
git staircase list --human
git staircase list --porcelain
git staircase list --json
```

These options are mutually exclusive.

---

### 11.1 Matching set

The **matching set** is the set of staircases remaining after applying:

* Lifecycle filters.
* Managed or implicit filters.
* Provider filters.
* Name or selector filters.
* Workspace or repository scope.
* Any other explicit list predicates.

The empty-output rules apply when the matching set is empty.

Unresolved discovery candidates are diagnostics, not members of the matching set.

---

### 11.2 Human mode

When the matching set is empty, human mode MUST write:

```text
No staircases.
```

followed by one newline to standard output.

Exit status MUST be `0`, except under a strict diagnostic mode defined below.

---

### 11.3 Porcelain mode

When the matching set is empty, porcelain mode MUST write zero bytes to standard output.

It MUST NOT write:

* A blank line.
* Whitespace.
* `No staircases.`
* A header.
* A bootstrap message.
* A comment.

Exit status MUST be `0`, except under strict diagnostic mode.

---

### 11.4 JSON mode

When the matching set is empty, JSON mode MUST write the UTF-8 JSON value:

```json
[]
```

followed by one newline to standard output.

Exit status MUST be `0`, except under strict diagnostic mode.

The top-level JSON type for `git staircase list --json` is always an array.

Workspace-bootstrap information, warnings, and diagnostics MUST NOT change the top-level type.

---

### 11.5 Standard error

In porcelain and JSON modes:

* Normal progress text MUST NOT be written to standard error.
* Automatic workspace-configuration announcements MUST be suppressed.
* Warnings MAY be written to standard error only when they affect completeness or correctness.
* Machine-readable diagnostics SHOULD be available through an explicit diagnostics option.

In human mode, a one-time workspace-configuration message MAY be written to standard error.

It MUST NOT precede or contaminate machine-readable standard output.

---

### 11.6 Strict mode

The option:

```console
git staircase list --strict
```

changes exit behavior for unresolved or ambiguous candidates.

If the matching set is empty but listing completed without strict diagnostic failures:

```text
exit status = 0
```

If strict mode detects an unresolved or ambiguous candidate:

```text
exit status != 0
```

The output mode still follows its formatting contract.

For example, JSON mode MAY emit:

```json
[]
```

on standard output while returning nonzero and emitting structured diagnostics separately.

---

### 11.7 Bootstrap failure

Failure to persist an automatically discovered workspace configuration is not equivalent to an empty staircase list.

If the command can safely continue with provider-free or invocation-local configuration, it SHOULD do so and apply the normal list-output contract.

If the command cannot establish a valid Git or workspace context, it MUST fail rather than output a successful empty list.

---

## 12. Idempotent and No-Op Output

Commands that perform idempotent state transitions, such as archiving an already archived staircase, SHOULD distinguish:

```text
successful state transition
successful no-op
```

in human and structured output.

They MUST return success for the defined idempotent no-op case.

This rule does not permit ambiguous selectors to become no-ops.

---

## 13. Superseded Clauses

The following earlier clauses are superseded:

### 13.1 Addendum A

The suggestion that `track` may be an alias for `adopt` is removed.

The phrase:

```text
managed staircase, tracked staircase, and adopted staircase
```

is replaced by:

```text
managed staircase
```

---

### 13.2 Addendum D and provider addenda

The term **adapter** is replaced by **provider bound to a capability**.

Examples:

```text
review adapter       -> review provider
verification adapter -> verification provider
transport adapter    -> transport provider
```

---

### 13.3 Addendum B and Addendum I

The generic phrase **staircase revision ID** is replaced by the exact applicable term:

```text
structure revision OID
metadata revision OID
lifecycle revision OID
record revision OID
```

Where earlier Addendum B defined a descriptor OID as representing the complete managed staircase state, Addendum I’s record-tree model takes precedence.

---

### 13.4 Addendum I

The suggestion that concurrent metadata edits may be resolved automatically through an optional three-way merge is removed.

Strict record-level CAS failure is the only core-compliant default.

---

### 13.5 Upload examples

Earlier top-level examples using:

```console
git staircase upload
```

are replaced by:

```console
git staircase review upload
```

Transport of Staircase records and refs continues to use:

```console
git staircase push
```

---

### 13.6 Indent and unindent terminology

Any parenthetical description of:

```text
split as indent
join as unindent
```

is removed.

The only canonical structural verbs are:

```text
split
join
```

---

## 14. Normative Invariants

An implementation conforming to this addendum MUST preserve the following invariants.

### 14.1 One canonical verb per core action

Aliases are not introduced merely for friendliness.

### 14.2 `adopt` is the sole implicit-to-managed transition command

`track` does not exist.

### 14.3 “Managed” is the persistent state term

“Tracked” and “adopted” are not parallel states.

### 14.4 Providers are capability implementations

“Adapter” is not a separate architectural concept.

### 14.5 Integration terms are typed

Integration anchor, symbolic integration target, and review destination are never collapsed into an unqualified target field.

### 14.6 Revision terms are typed

Structure, metadata, lifecycle, and record revisions remain distinct.

### 14.7 Persistent updates compare the complete record

A metadata edit cannot overwrite a concurrent structural or lifecycle update.

### 14.8 CAS conflicts fail

There is no implicit last-write-wins or automatic metadata merge.

### 14.9 Review publication and Staircase transport remain distinct

`review upload` publishes review revisions; `push` transports Staircase state.

### 14.10 Empty machine output is exact

Porcelain emits zero bytes; JSON emits an empty array.

### 14.11 Human empty output is explicit

Human mode emits `No staircases.` and succeeds.

### 14.12 Machine output is not contaminated by bootstrap prose

Automatic configuration messages do not appear on machine-readable standard output.

---

## 15. Summary

The canonical state and command vocabulary is:

```text
implicit
    discovered from Git structure

adopt
    create persistent management

managed
    persistent staircase state

archive
    deactivate and hide owned active refs

unarchive
    reactivate archived state
```

Persistent state uses four exact revision concepts:

```text
structure revision
metadata revision
lifecycle revision
record revision
```

Every persistent mutation compares and replaces the complete current record revision.

The governing rules are:

> One action has one canonical verb.

> Persistent edits replace an expected record, never whatever record happens to be current at publication time.

> Human output may explain; machine output must be exact.
