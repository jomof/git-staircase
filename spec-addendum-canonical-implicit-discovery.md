# Addendum K: Canonical Implicit Discovery, Selector Integrity, and Deterministic Addressability

## 1. Status and Scope

This addendum modifies and clarifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**
5. **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**
6. **Addendum I: Persistent Staircase Records, User Metadata, and Archival Lifecycle**
7. **Addendum J: Canonical Vocabulary, Record Concurrency, and Machine-Output Semantics**

It defines:

* The distinction between raw discovery evidence and canonical implicit staircases.
* Exact equivalence rules for duplicate discovery candidates.
* Deterministic structural-key generation.
* Rules for discovering one-step staircases.
* Naming and alias behavior for implicit staircases.
* The referential-integrity contract between `list` and later commands.
* Resolution of bare names and typed selectors.
* Correct ambiguity diagnostics and suggested commands.
* Archive behavior for implicit staircases.
* Concurrency and state-change behavior between discovery and mutation.
* Required behavior when multiple discovery paths produce the same staircase.
* Required behavior when genuinely distinct staircases share one human-facing name.

This addendum supersedes any earlier behavior that permits:

* `list` to display one implicit staircase while another command sees several noncanonical candidates behind that entry.
* Duplicate discovery candidates to become user-facing ambiguity.
* Same-top candidates to be treated as equivalent without comparing their complete structure.
* A typed name selector to be suggested as a remedy for a name ambiguity.
* Diagnostics that fail to show the facts distinguishing ambiguous candidates.

---

## 2. Core Principle

Discovery is a normalization pipeline:

```text
raw discovery evidence
        ↓
normalized candidate records
        ↓
structural equivalence classes
        ↓
canonical implicit staircases
        ↓
human names and machine selectors
        ↓
command selection
```

Only **canonical implicit staircases** may be:

* Listed.
* Named.
* Selected.
* Shown.
* Adopted.
* Archived.
* Passed to structural operations.

Raw discovery evidence is internal implementation data.

The governing rule is:

> Duplicate evidence must never become duplicate user-visible staircases.

---

## 3. Raw Discovery Evidence

A **raw discovery record** is one observation suggesting that a staircase may exist.

Possible evidence sources include:

* A local branch.
* A sequential branch family.
* A managed or provider-supplied cut.
* A branch upstream.
* A workspace integration context.
* An explicit command argument.
* A graph traversal beginning at a particular ref.
* A review-provider association.
* A previously cached discovery result.

Several raw records may describe the same staircase.

Examples include:

```text
refs/heads/feature
refs/remotes/user/feature
detached HEAD at the same commit
provider review head at the same commit
two graph traversals reaching the same cut chain
two symbolic integration locators resolving to the same anchor
```

Raw records have no user-facing identity.

They must not receive independently selectable structural keys.

---

## 4. Normalized Implicit Candidate

A raw record is normalized into a candidate containing at least:

```text
repository identity
object format
integration-context identity
ordered cut OIDs
active cut OIDs
ordered step decomposition
materializing local refs
additional aliases
discovery provenance
relationship to the integration context
```

Normalization must:

1. Resolve every revision expression to a full OID.
2. Resolve symbolic integration locators to exact integration anchors.
3. Remove structurally integrated lower cuts.
4. Reject empty steps.
5. Verify cut ancestry.
6. Canonically order unordered evidence.
7. Separate structural facts from names and provenance.
8. Preserve enough diagnostic information to explain rejected or distinct candidates.

---

## 5. Canonical Structural Identity

### 5.1 Canonical structural tuple

For an implicit linear staircase, canonical structural identity is determined by:

[
C =
(R,O,I,A)
]

where:

* (R) is the canonical repository identity.
* (O) is the repository object format.
* (I) is the canonical integration-context identity.
* (A) is the ordered sequence of active cut OIDs.

The ordered step decomposition is derivable from (I) and (A), but implementations may include it in the serialized identity as a consistency check.

Names, refs, display labels, and discovery provenance are not part of structural identity.

---

### 5.2 Repository identity

Repository identity must distinguish different object databases even when they contain commits with identical OIDs.

Within one invocation, the canonical Git common directory plus repository object format may be used.

Persistent cross-location identity may use a stronger repository identity when defined elsewhere.

Two repositories do not share an implicit staircase merely because the same commit OID exists in both.

---

### 5.3 Integration-context identity

For the ordinary single-anchor case, integration-context identity contains:

```text
kind: single-anchor
anchor OID: <full OID>
```

Two symbolic locators resolving to the same anchor OID identify the same exact integration context.

Example:

```text
refs/remotes/origin/main -> A
refs/remotes/upstream/main -> A
```

These produce one integration-context identity with two provenance aliases.

---

### 5.4 Multiple integration anchors

For an explicitly defined multi-anchor integration set, identity contains the canonical irredundant anchor set.

Before serialization:

1. Resolve every anchor to a full commit OID.
2. Remove duplicate OIDs.
3. Remove an anchor whose ancestor set is wholly contained in another selected anchor’s ancestor set.
4. Sort the remaining OIDs lexically by raw encoded OID.
5. Include the integration-set kind.

Automatic heuristic discovery must not construct a multi-anchor integration set unless another addendum explicitly permits it.

---

### 5.5 Ordered cuts

Cuts are serialized as full commit OIDs in bottom-to-tip order.

Two candidates with the same aggregate top but different lower cuts are different staircases.

Example:

```text
candidate A cuts:
  C1, C3

candidate B cuts:
  C2, C3
```

These candidates share top `C3` but have different decomposition.

They must not be collapsed.

---

### 5.6 Active cuts

Cuts already structurally integrated into the selected integration set are removed before canonical active identity is computed.

A fully integrated candidate has no active cuts and is not an active implicit staircase.

Historical or provider-associated records may still be represented through managed state, but they are not discovered as active implicit staircases solely from an empty body.

---

## 6. Structural Equivalence

Two normalized implicit candidates are structurally equivalent if and only if all of the following are equal:

1. Repository identity.
2. Object format.
3. Canonical integration-context identity.
4. Ordered active cut OIDs.

Equivalent candidates are one canonical implicit staircase.

Their following properties are merged:

* Materializing refs.
* Human-facing name aliases.
* Symbolic integration locators.
* Discovery provenance.
* Provider hints.
* Corroborating refs.
* Diagnostic evidence.

---

### 6.1 Same top is insufficient

The following condition alone does not establish equivalence:

```text
candidate A top OID = candidate B top OID
```

Candidates with the same top may differ in:

* Integration anchor.
* Lower cuts.
* Step boundaries.
* Active integrated prefix.
* Repository identity.

Such candidates remain distinct.

---

### 6.2 Same structure with different provenance

The following candidates are equivalent:

```text
candidate A:
  integration anchor: A
  cuts: C1, C2
  discovered from refs/heads/feature

candidate B:
  integration anchor: A
  cuts: C1, C2
  discovered from a sequential branch scan
```

They form one canonical staircase with merged provenance.

---

### 6.3 Same structure with several refs

If several refs materialize the same cuts, they do not create several staircases.

Example:

```text
refs/heads/feature       -> C2
refs/heads/feature-copy  -> C2
```

with the same integration context and cut sequence produces one staircase with two nominal aliases or materializing refs.

Ownership of those refs is a separate question.

---

## 7. Structural Key

### 7.1 Definition

Every canonical implicit staircase receives one structural key:

```text
implicit@<digest>
```

The digest is computed over a domain-separated canonical serialization of the structural tuple.

Conceptually:

```text
git-staircase-implicit-structure 1
repository <repository-identity>
object-format <format>
integration-kind <kind>
integration-anchor <full-oid>
cut <full-oid>
cut <full-oid>
...
```

The digest algorithm and serialization version must be explicit.

---

### 7.2 Determinism

Under unchanged:

* Git objects.
* Relevant refs.
* Provider-free or provider-supplied integration context.
* Discovery configuration.

the same canonical implicit staircase must receive the same structural key across invocations.

Discovery traversal order must not affect the key.

---

### 7.3 One key per equivalence class

Equivalent raw or normalized candidates must receive one structural key.

It is nonconforming for two structural keys to refer to structurally equivalent candidates.

---

### 7.4 Prefix display

Human output may abbreviate the digest.

The displayed prefix must be unique among all canonical implicit staircases in the current selection scope.

If the default prefix is not unique, the implementation must extend it until unique.

It must not print two identical or ambiguous abbreviated structural keys.

Machine output must contain the full structural key.

---

## 8. One-Step Staircase Discovery

### 8.1 Nonempty body requirement

A one-step implicit staircase exists only when its body relative to the selected integration context is nonempty.

For top cut (a_1) and integration set (U):

[
S_1 =
\operatorname{Ancestors}(a_1)\setminus U
]

The candidate is valid only when:

[
S_1 \neq \varnothing
]

---

### 8.2 Integrated branch is not a staircase

If:

```text
top cut ∈ integration set
```

the branch contributes no active step.

It must not be listed as a one-step implicit staircase.

Therefore, if local `main` points exactly to its integration anchor or is already reachable from that anchor, output such as:

```text
main 1 step clean (implicit)
```

is nonconforming.

---

### 8.3 Integration-target ref is not automatically work

A ref selected solely as the symbolic integration target must not also be discovered as staircase work unless it contains commits outside the exact integration set selected for that candidate.

The existence of the ref is not enough.

---

### 8.4 Local commits ahead of integration

A branch conventionally named `main` may validly be a one-step implicit staircase when it contains local commits outside its integration context.

Example:

```text
refs/remotes/origin/main -> A
refs/heads/main          -> C
A < C
```

with commits between `A` and `C`.

The branch spelling does not exempt it from staircase discovery.

The deciding fact is nonempty unintegrated history.

---

### 8.5 No empty synthetic step

Discovery must not manufacture a one-step staircase whose only “step” contains zero commits.

An empty body is an empty discovery result, not a clean staircase.

---

## 9. Canonical Names and Aliases

### 9.1 Implicit display name

An implicit staircase may have one canonical display name and zero or more aliases.

Names are derived from materializing refs or another explicitly defined naming source.

Names do not define structural identity.

---

### 9.2 Alias collapse

If several branch names refer to one structurally equivalent staircase, selector resolution by any unambiguous alias selects the same canonical staircase.

Example:

```text
canonical display name:
  feature

aliases:
  feature-copy
  users/jomo/feature
```

All may resolve to the same structural key.

---

### 9.3 Canonical display-name selection

When several aliases exist, the canonical display name is chosen deterministically using:

1. The complete conforming sequential-layout tip name, when one exists.
2. An explicitly selected primary branch name.
3. A unique local branch attached to the aggregate top.
4. The lexically first eligible local branch short name.
5. The structural key when no human name is available.

Current checkout state must not change the canonical name unless it changes the underlying eligible primary-name evidence.

---

### 9.4 Distinct staircases with the same name

Two non-equivalent canonical staircases may have the same human-facing name.

In that case, the name is ambiguous.

Neither candidate may be hidden.

---

## 10. Listing Contract

### 10.1 Listing operates on canonical staircases

`git staircase list` must list canonical managed and implicit staircases.

It must not list raw discovery records.

It must not group non-equivalent candidates solely because they share:

* A name.
* A top OID.
* A branch prefix.
* A review title.

---

### 10.2 One listed entry means one selectable object

Each listed staircase entry represents exactly one canonical selectable staircase.

Under unchanged repository and configuration state, the displayed selector must resolve to that staircase.

---

### 10.3 Unique name display

When a canonical name is unique, human output may show:

```text
spec-implement-review-identity  1 step  clean  (implicit)
```

The bare name must then select that exact staircase.

---

### 10.4 Ambiguous name display

When two non-equivalent staircases share a name, human output must disambiguate them in the primary listing.

Example:

```text
spec-implement-review-identity
  implicit@46045b37a030d3f9  1 step  clean
  integration anchor: 3a91c0...
  top: 81bdca...

spec-implement-review-identity
  implicit@9d516b0b4c3271f6  2 steps  clean
  integration anchor: 712ea4...
  cuts: 40f1ab..., 81bdca...
```

A compact form may be:

```text
spec-implement-review-identity@46045b37  1 step  clean  (implicit)
spec-implement-review-identity@9d516b0b  2 steps clean  (implicit)
```

The structural key remains the canonical machine selector.

---

### 10.5 No hidden ambiguity

The following sequence is forbidden under unchanged state:

```console
$ git staircase list
spec-implement-review-identity 1 step clean (implicit)

$ git staircase archive spec-implement-review-identity
error: selector is ambiguous
```

If the name is ambiguous for `archive`, it was already ambiguous for `list`.

---

### 10.6 Machine output

Each implicit staircase in JSON output must include:

```json
{
  "kind": "implicit",
  "structural_key": "implicit@<full-digest>",
  "canonical_name": "spec-implement-review-identity",
  "aliases": [],
  "integration_context": {
    "anchor_oid": "<full-oid>"
  },
  "cuts": [
    "<full-oid>"
  ],
  "top_oid": "<full-oid>",
  "materializing_refs": [
    "refs/heads/spec-implement-review-identity"
  ]
}
```

Equivalent raw candidates must not appear as duplicate array entries.

---

## 11. Referential Integrity

### 11.1 Definition

**Selector referential integrity** means:

> A selector presented by one command must identify the same canonical staircase in a later command while the relevant repository state remains unchanged.

Relevant state includes:

* Refs participating in discovery.
* Git objects.
* Integration-context configuration.
* Workspace-provider results.
* Staircase records.
* Discovery policy.

---

### 11.2 Human display selectors

When `list` displays a bare canonical name without qualification, that name must be uniquely selectable.

When uniqueness cannot be guaranteed, `list` must display a structural-key qualification.

---

### 11.3 Machine selectors

Automation should use:

```text
managed lineage ID
managed canonical name when unique
full implicit structural key
record revision OID when exact record selection is required
```

A top commit OID alone is not a complete implicit-staircase selector.

---

### 11.4 State changed after listing

If relevant state changes between `list` and a mutating command, the later command may legitimately fail.

The diagnostic must distinguish this from an ambiguity that was already present.

Recommended error:

```text
error: staircase selection changed since discovery

selector:
  spec-implement-review-identity

previously unique candidate:
  implicit@46045b...

current candidates:
  implicit@46045b...
  implicit@9d516b...
```

A command that did not receive a prior selection token may say:

```text
error: selector is no longer unique
```

It must still display the distinguishing facts.

---

## 12. Selector Resolution

### 12.1 Typed selectors

The canonical typed selectors are:

```text
--name <exact-name>
--id <managed-lineage-id>
--structural-key <implicit-key>
--record <record-revision-oid>
```

A command may also accept a bare selector according to Addendum B and this addendum.

---

### 12.2 `--structural-key`

`--structural-key` selects one canonical implicit staircase.

It must not select a raw candidate.

If no canonical staircase has that key, the command fails.

If more than one canonical staircase somehow has the same full key, the repository or implementation is inconsistent and the command must fail as an internal integrity error.

---

### 12.3 `--name`

`--name` performs exact name or alias matching.

It does not resolve ambiguity when several non-equivalent canonical staircases share that name.

Therefore this is a nonconforming ambiguity suggestion:

```console
git staircase show --name spec-implement-review-identity
```

when the error already established that the name matches several staircases.

---

### 12.4 Bare selector

A bare selector matching one canonical staircase succeeds.

A bare selector matching several non-equivalent canonical staircases fails.

Equivalent raw evidence has already been collapsed before this stage.

---

### 12.5 Managed and implicit name collision

If one managed staircase and one non-equivalent implicit staircase share the same name, a bare selector is ambiguous.

Neither silently takes precedence.

The user must use:

```text
--id
--structural-key
```

or another typed unique selector.

A managed staircase and an implicit candidate that describe the same lineage and exact current structure should normally be recognized as the managed staircase, not displayed twice.

---

## 13. Ambiguity Diagnostics

### 13.1 Required distinguishing fields

An ambiguity diagnostic must include enough information to explain why the candidates are distinct.

For each candidate, it must show at least:

```text
structural key
integration anchor or integration-context identity
ordered cuts or step count plus cut summary
top OID
materializing refs
```

It should also show:

```text
discovery provenance
relationship to integration context
canonical name and aliases
```

---

### 13.2 Identical visible facts

If two candidates have:

* Different structural keys, but
* The same repository identity,
* The same integration context,
* The same ordered cuts,
* The same top OID,

the implementation has violated canonicalization.

It must not report a user ambiguity.

Recommended error:

```text
error: internal discovery inconsistency

two structural keys were generated for the same canonical staircase
```

---

### 13.3 Same top, different structure

If candidates share a top but differ structurally, the diagnostic must show the actual distinction.

Example:

```text
candidate 1:
  structural key: implicit@46045...
  integration anchor: A
  cuts: C

candidate 2:
  structural key: implicit@9d516...
  integration anchor: B
  cuts: C
```

or:

```text
candidate 1 cuts:
  C

candidate 2 cuts:
  B1, C
```

Displaying only the shared top is insufficient.

---

### 13.4 Command-specific suggestions

Suggested commands must preserve the command the user invoked.

For:

```console
git staircase archive spec-implement-review-identity
```

valid suggestions are:

```console
git staircase archive \
    --structural-key implicit@46045b37a030d3f9

git staircase archive \
    --structural-key implicit@9d516b0b4c3271f6
```

The diagnostic must not suggest only `show` unless it is clearly labeled as an inspection command in addition to the actual resolution commands.

---

### 13.5 Complete suggestions

Every viable candidate must receive a corresponding typed-selector suggestion unless output is intentionally truncated and says so.

Showing two candidates but suggesting only one is nonconforming.

---

### 13.6 Inspection suggestions

Optional inspection commands may be added:

```console
git staircase show \
    --structural-key implicit@46045b37a030d3f9

git staircase show \
    --structural-key implicit@9d516b0b4c3271f6
```

These do not replace the command-specific remedies.

---

## 14. Archive of an Implicit Staircase

### 14.1 Canonical selection first

`archive` must resolve one canonical staircase before adoption or ref mutation begins.

It must not independently process every raw discovery record matching the selected name.

---

### 14.2 Automatic adoption

Archiving an implicit staircase performs:

```text
canonical implicit staircase
        ↓
managed active staircase
        ↓
managed archived staircase
```

Adoption and archive form one logical operation.

The resulting managed structure must correspond exactly to the selected canonical structural key.

---

### 14.3 Structural-key preservation

The archive manifest should record the originating implicit structural key as provenance.

The managed lineage ID and record revision become the persistent identity after adoption.

---

### 14.4 Ref ownership is a separate decision

Several refs may materialize one canonical implicit staircase.

This does not make the staircase selector ambiguous.

Archive must separately determine which refs are owned.

Possible outcomes include:

```text
one canonical staircase
one owned sequential branch family
two unowned aliases
```

The command archives owned refs and warns about unowned aliases under Addendum I.

---

### 14.5 Ambiguous ownership

If staircase structure is unique but ref ownership is ambiguous, the command must report an ownership error, not a staircase-selector ambiguity.

Example:

```text
error: staircase is unique, but branch ownership is ambiguous

staircase:
  implicit@46045...

candidate primary branches:
  refs/heads/feature
  refs/heads/feature-copy
```

The user may explicitly identify owned refs or adopt before configuring ownership.

---

### 14.6 No duplicate lineage creation

Equivalent raw candidates must never cause archive to create several managed lineages.

One canonical implicit staircase produces at most one new lineage during one operation.

---

## 15. Discovery Snapshot for Mutating Commands

### 15.1 Immutable operation plan

After selection, a mutating command creates a discovery snapshot containing:

```text
canonical structural key
repository identity
integration context
ordered cuts
materializing refs
expected ref OIDs
discovery configuration fingerprint
```

The command plans against this snapshot.

---

### 15.2 Prepublication validation

Before publishing adoption, archive, or another mutation, the command must verify:

* Every selected cut still exists.
* Every expected owned ref still has its expected OID.
* The integration context remains valid.
* The canonical candidate still has the selected structural key.
* No conflicting managed record was created concurrently.

If validation fails, the command aborts and replans only through an explicit retry.

---

### 15.3 No silent reselection

A command must not respond to concurrent state changes by silently selecting another candidate with the same human-facing name.

The selected structural key governs the entire operation.

---

## 16. Discovery Provenance and Diagnostic Deduplication

### 16.1 Provenance merging

When equivalent candidates are discovered through several sources, diagnostics may show:

```text
discovered from:
  refs/heads/spec-implement-review-identity
  sequential branch-family scan
  review-provider local association
```

These are several observations of one staircase.

---

### 16.2 Provenance does not create identity

Adding or removing a redundant discovery source must not change the structural key.

For example, fetching an additional remote ref at the same cut must not create a new implicit staircase.

---

### 16.3 Provider hints

Provider review identities do not create a second staircase when they refer to the same local cut structure.

They are merged as provider associations or evidence.

A provider may distinguish review topology, but local implicit structural identity remains graph-based.

---

## 17. Caching

### 17.1 Cache contents

A discovery cache may store:

```text
canonical structural key
normalized structure
aliases
provenance
configuration fingerprint
relevant ref OIDs
```

---

### 17.2 Cache validity

Cached candidates must be invalidated when relevant:

* Refs move.
* Refs are created or deleted.
* Integration-context configuration changes.
* Provider workspace evidence changes.
* The object database changes in a way visible to discovery.
* Discovery policy changes.

---

### 17.3 Cache cannot define identity

A stale cache entry must not coexist as a second candidate beside its freshly discovered equivalent.

Cache records are evidence sources subject to canonicalization.

---

## 18. Correct Behavior for the Reported Case

Given the observed internal results:

```text
implicit@46045b37a030d3f9
top: 81bdcac7e76440cbc37aa5ebd4352f9173edefdb

implicit@9d516b0b4c3271f6
top: 81bdcac7e76440cbc37aa5ebd4352f9173edefdb
```

the implementation must determine which of the following cases applies.

---

### 18.1 Case A: Structurally equivalent

If both candidates have the same:

```text
repository identity
integration context
ordered cuts
```

they are one canonical staircase.

Required behavior:

```console
$ git staircase list
spec-implement-review-identity  1 step  clean  (implicit)

$ git staircase archive spec-implement-review-identity
Archived staircase 'spec-implement-review-identity'.
```

Only one structural key exists.

The duplicate raw evidence may appear only in diagnostics as merged provenance.

---

### 18.2 Case B: Same top, different integration context

If the candidates use different integration anchors, they are distinct.

Required listing:

```text
spec-implement-review-identity
  implicit@46045b37  1 step  clean
  integration anchor: <A>
  top: 81bdcac7

spec-implement-review-identity
  implicit@9d516b0b  1 step  clean
  integration anchor: <B>
  top: 81bdcac7
```

Bare-name archive must fail with both complete command-specific suggestions.

---

### 18.3 Case C: Same top, different cuts

If one candidate has additional lower cuts, they are distinct decompositions.

Required listing must expose the different step count or cut sequence.

---

### 18.4 Case D: No visible structural difference

If the implementation cannot show a structural difference, reporting ambiguity is forbidden.

It must treat the condition as an internal canonicalization defect.

---

## 19. Additional Required Diagnostics

The command:

```console
git staircase list --diagnostics
```

should show:

```text
raw discovery records: 4
normalized candidates: 2
canonical implicit staircases: 1
duplicates collapsed: 1
```

For each collapsed equivalence class:

```text
canonical:
  implicit@46045...

merged evidence:
  local branch scan
  sequential-layout scan
```

For distinct same-name candidates:

```text
name collision:
  spec-implement-review-identity

candidates:
  implicit@...
  implicit@...

distinguishing dimensions:
  integration context
```

---

## 20. Exit Behavior

### 20.1 `list`

Name ambiguity does not make `list` fail.

`list` displays every canonical candidate with sufficient disambiguation.

---

### 20.2 Selected read-only command

A read-only command with an ambiguous bare selector fails with a nonzero exit status and complete candidate diagnostics.

---

### 20.3 Selected mutating command

A mutating command with an ambiguous selector performs no:

* Adoption.
* Ref creation.
* Ref deletion.
* Record write.
* Provider mutation.
* Worktree change.

It fails before acquiring destructive operation state.

---

### 20.4 Internal canonicalization failure

An internal condition in which two structural keys represent the same canonical tuple must return a distinct integrity error.

It must not be presented as a user selection mistake.

---

## 21. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 21.1 Discovery evidence is not a staircase

Only canonical equivalence classes become user-visible staircases.

### 21.2 Equivalent candidates collapse

Different refs, traversals, caches, or providers do not create duplicate staircases when structure is identical.

### 21.3 Same top is not enough

Integration context and ordered cuts participate in identity.

### 21.4 One structural key identifies one canonical implicit staircase

Equivalent candidates cannot have different keys.

### 21.5 One listed entry is one selectable object

`list` cannot conceal an ambiguity that later commands expose under unchanged state.

### 21.6 Ambiguous names are disambiguated during listing

Every distinct candidate remains visible.

### 21.7 A unique listed name resolves uniquely

Bare-name selection must succeed under unchanged state.

### 21.8 Empty bodies are not one-step staircases

Integrated refs do not appear as synthetic clean steps.

### 21.9 Name ambiguity and ownership ambiguity are distinct

A unique staircase with several possible owned refs is not several staircases.

### 21.10 Typed selector advice must actually disambiguate

`--name` cannot be suggested as the remedy for a name collision.

### 21.11 Suggestions preserve command intent

An `archive` failure suggests `archive` commands, not only `show` commands.

### 21.12 Every candidate is represented in diagnostics

Diagnostics do not silently omit alternatives.

### 21.13 Mutating commands bind to a structural key

They do not silently reselect by human name after planning.

### 21.14 Archive adopts once

One canonical implicit staircase produces one managed lineage.

### 21.15 Structural-key differences must be explainable

If no structural distinction can be displayed, ambiguity is an implementation defect.

---

## 22. Summary

Implicit discovery proceeds through:

```text
evidence
    many observations

normalization
    exact OIDs and integration context

canonicalization
    one object per distinct staircase structure

naming
    human aliases over canonical objects

selection
    unique name or typed structural key
```

The observed behavior is nonconforming because it displays one staircase but later resolves the displayed name to two unexplained internal candidates.

The governing rules are:

> Listing is a promise of addressability.

> One staircase may have many witnesses, but it has only one structural key.

> If two candidates are genuinely different, the difference must be visible before the user attempts to mutate either one.
