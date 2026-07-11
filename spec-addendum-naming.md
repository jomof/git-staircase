# Addendum B: Staircase Names, Selectors, References, and Identifiers

## 1. Status and Scope

This addendum modifies the following documents:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**

It defines:

* Which existing Git naming forms `git staircase` accepts.
* How accepted names are resolved and type-checked.
* Which forms may be persisted.
* The distinction between names, selectors, object identities, and lineage identities.
* How managed staircase names are represented as Git refs.
* How staircase revision, lineage, step, body, and outcome identities are produced.
* How names and identities behave under rename, rebase, split, join, transport, concurrency, and deletion.
* How implicit staircases are named without granting them false persistent identity.
* How machine-readable output differs from human-readable display.

This addendum is normative.

Where this addendum conflicts with earlier examples involving `refs/staircases/*`, staircase identity, or selector resolution, this addendum takes precedence.

---

## 2. Design Principles

### 2.1 Consume broadly, persist narrowly

`git staircase` should accept the ordinary forms by which Git users name commits and refs.

After resolving an input, however, it must persist only canonical forms:

* Full refnames.
* Full object IDs.
* Explicit object-format information where repository context is unavailable.
* Stable staircase lineage IDs.
* Stable step IDs.
* Canonically serialized staircase revision descriptors.

Expressions such as:

```text
main~3
HEAD@{yesterday}
v2.1-14-g2414721
:/fix authentication
```

may be accepted as command input, but must not be stored as authoritative identity.

---

### 2.2 Names and identities are distinct

A **name** is a mutable, human-facing locator.

An **identity** answers whether two observations refer to the same entity or revision under a specified equivalence relation.

For staircases:

* A refname identifies a movable named staircase.
* A lineage ID identifies the same evolving staircase across rewrites and renames.
* A staircase revision OID identifies one exact immutable staircase state.
* A commit OID identifies one exact commit.
* A step ID identifies one conceptual managed step across applicable rewrites.
* A step ordinal identifies only a current position.

No one identifier should be overloaded to serve all of these purposes.

---

### 2.3 Resolve first, then type-check

A string that Git can resolve is not necessarily valid for every staircase role.

Examples:

* A staircase cut must resolve to a commit.
* An integration target must resolve to a commit or supported integration-boundary object.
* A managed staircase revision must resolve to a valid staircase descriptor object.
* A lineage ID must match the staircase lineage-ID grammar.
* A revision range is not a single commit.

Commands must resolve a selector and then verify its required type.

For commit-valued inputs, the semantic equivalent of:

```console
git rev-parse --verify '<input>^{commit}'
```

should be used. Git explicitly recommends the `^{type}` form when a caller needs to ensure that an expression resolves to a particular object type.

---

### 2.4 Ambiguity must be surfaced

Git itself has precedence rules for ambiguous abbreviated refnames. It searches special top-level names, `refs/<name>`, tags, branches, remote-tracking refs, and remote `HEAD` refs in that order.

`git staircase` adds another semantic namespace on top of Git’s revision namespace. It must not silently use precedence when one spelling plausibly denotes several different staircase or Git entities.

The command must:

1. Compute all relevant interpretations.
2. Collapse interpretations that resolve to the same semantic object.
3. Proceed if exactly one distinct interpretation remains.
4. Report an ambiguity if several distinct interpretations remain.

Explicitly typed options bypass this cross-namespace ambiguity.

---

### 2.5 Human-friendly abbreviations are not persistent identity

Abbreviated OIDs, shortened refnames, derived implicit names, and `git describe` output are display conveniences.

They must not appear in:

* Persistent staircase descriptors.
* Transport records.
* Verification keys.
* Review mappings.
* Recovery metadata.
* Machine-readable identifiers advertised as stable.

---

### 2.6 Git objects are immutable; refs are movable

The core representation should follow Git’s existing division:

* Immutable staircase state is stored as a Git object.
* A mutable staircase name is stored as a Git ref pointing to that object.
* Historical movement is recorded in a reflog.
* Multi-ref state changes use reference transactions and compare-and-swap checks.

This mirrors the relationship between commits and branches rather than inventing an unrelated naming database.

---

## 3. Terminology

### 3.1 Selector

A **selector** is user input accepted by a command to locate an entity.

A selector may be:

* A full refname.
* An abbreviated refname.
* A Git revision expression.
* An OID or abbreviated OID.
* A staircase name.
* A lineage ID.
* A staircase revision ID.
* A step ID.
* A step ordinal.
* A derived implicit-staircase label.

A selector is not necessarily suitable for storage.

---

### 3.2 Canonical name

A **canonical staircase name** is the suffix of a ref under:

```text
refs/staircases/
```

For example:

```text
auth
```

has canonical full refname:

```text
refs/staircases/auth
```

A staircase may be managed without a canonical name. Such a staircase is selected by lineage ID until a name is assigned.

A managed staircase has at most one canonical name in the core model.

---

### 3.3 Full staircase refname

The full refname of a named managed staircase is:

```text
refs/staircases/<name>
```

Example:

```text
refs/staircases/auth
```

The ref points to the immutable descriptor object for the staircase’s current revision.

It does not point directly to the aggregate top commit.

---

### 3.4 Qualified staircase name

The qualified shorthand is:

```text
staircases/<name>
```

Git revision resolution checks `refs/<refname>` before checking tag, branch, and remote namespaces, so:

```text
staircases/auth
```

naturally resolves to:

```text
refs/staircases/auth
```

when that ref exists.

This is the preferred compact form for scripts that use positional selectors.

---

### 3.5 Bare staircase name

A **bare staircase name** omits both prefixes:

```text
auth
```

Bare names are a `git staircase` convenience. Standard Git does not search `refs/staircases/` when resolving the bare spelling.

Bare names must therefore undergo the ambiguity checks defined in this addendum.

---

### 3.6 Lineage ID

A **lineage ID** is an opaque, stable identifier assigned when a staircase becomes managed.

It identifies the same evolving staircase across:

* Ref renames.
* Commit rewrites.
* Rebases.
* Restacks.
* Step reordering.
* Partial landing, when continuity is preserved.
* Changes to exact staircase revision OIDs.

A lineage ID is not derived from:

* A staircase name.
* Commit OIDs.
* Step OIDs.
* Target refs.
* Repository location.
* Current content.

The canonical initial format is a lowercase UUID:

```text
3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

Commands should accept lineage IDs through an explicitly typed option:

```console
git staircase show --id 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

A lineage ID must not be interpreted as a Git object name.

---

### 3.7 Staircase revision ID

A **staircase revision ID** is the full OID of the immutable descriptor object representing one exact staircase state.

It changes whenever any semantically relevant descriptor field changes, including:

* Integration boundary.
* Ordered step membership.
* Current cut OIDs.
* Step identities.
* Materialization state.
* Verification or landing policy stored in the descriptor.
* Review mappings stored in the descriptor.
* Discovery overrides.
* Parent staircase revision.

It does not change merely because the canonical name ref is renamed.

Within one repository, the raw full OID is sufficient:

```text
982cc1c...
```

Outside an established repository object-format context, it must be represented as a pair:

```text
sha1:<full-oid>
```

or:

```text
sha256:<full-oid>
```

Implementations must not assume that OIDs are always 40 hexadecimal characters. Git exposes the repository’s storage, input, output, and compatibility object formats through `git rev-parse --show-object-format`.

The algorithm-qualified form is a staircase interchange format. It is not required to be accepted directly by ordinary Git revision parsing.

---

### 3.8 Step ID

A **step ID** is an opaque stable identifier for one managed conceptual step.

Its initial format is a lowercase UUID.

A step ID remains stable under:

* Rebase.
* Restack.
* Commit amendment.
* Commit squashing within the step.
* Branch rename.
* Step reorder.
* Changes to the step’s ordinal position.

It does not identify one exact patch or one exact cut commit.

Step IDs do not exist for purely implicit staircases.

---

### 3.9 Step ordinal

A **step ordinal** is the current one-based position of a step:

```text
1
2
3
```

An expression such as:

```text
auth:2
```

means the step currently occupying position 2.

It does not mean the same conceptual step after a reorder, insertion, split, join, or deletion.

Ordinals are convenient selectors, not identities.

---

### 3.10 Cut OID

A **cut OID** is the full commit OID currently marking the cumulative end of a step.

It identifies one exact commit.

A rewrite changes the cut OID even when the managed step ID remains unchanged.

---

### 3.11 Structural revision key

An **implicit structural revision key** identifies one exact discovered staircase structure without assigning lineage.

It is computed from a canonical description containing at least:

* Object format.
* Resolved integration boundary.
* Ordered full cut OIDs.
* Discovery schema version.

It may be shown as:

```text
implicit@7ba91cd
```

in human output.

The full key may be used for repeatable selection while all referenced objects remain available.

It is not a lineage ID and does not survive rewriting.

---

## 4. Accepted Existing Git Naming Forms

### 4.1 Commit-ish inputs

Where a command expects a commit, `git staircase` should accept any Git revision expression that resolves unambiguously to a commit.

This includes:

* Full OIDs.
* Unique abbreviated OIDs.
* Branch refs.
* Tag refs that peel to commits.
* Remote-tracking refs.
* `HEAD`.
* Relevant special revision names.
* Reflog selectors.
* Parent and ancestor expressions.
* `git describe` output.
* Commit-message search expressions.

Examples:

```text
HEAD
main
refs/heads/main
origin/main
refs/remotes/origin/main
v2.1
main~3
merge-branch^2
HEAD@{yesterday}
@{-1}
main@{upstream}
v2.1-14-g2414721
:/fix authentication race
```

Git accepts full OIDs, unique leading OID substrings, refnames, `git describe` output, ancestry suffixes, reflog expressions, and message searches as revision syntax.

After resolving such an expression, `git staircase` must retain the full resulting OID for the current operation.

---

### 4.2 Ref inputs

Where a command specifically expects a ref rather than merely a commit, it must distinguish:

* The refname itself.
* The object currently referenced by that ref.

Examples include:

* A target ref intended to advance over time.
* A branch used to materialize a cut.
* A managed staircase name ref.
* A remote-tracking ref used for discovery.

The canonical stored form must be a full refname:

```text
refs/heads/main
refs/remotes/origin/main
refs/staircases/auth
```

The tool may accept an abbreviated spelling, but it must resolve and store the full refname.

Existence checks on a known full refname should use exact-ref semantics equivalent to:

```console
git show-ref --verify -- refs/heads/main
```

rather than suffix matching. Git recommends full refnames with `--verify` when exactness matters.

---

### 4.3 Annotated tags

An annotated tag may be accepted wherever its target type satisfies the command.

If a command requires a commit, the tag must peel to a commit.

If a command requires a staircase descriptor, the tag may point to a descriptor object.

Annotated tags are separate Git objects containing a target, tagger information, a message, and an optional signature. Lightweight tags are refs pointing directly to an object.

This addendum uses annotated tags for optional immutable, human-named staircase snapshots, as specified later.

---

### 4.4 Reflog selectors

Reflog selectors may be consumed as ephemeral command input:

```text
main@{1}
HEAD@{yesterday}
refs/staircases/auth@{2}
```

After resolution, only the full resulting OID may be used for the operation.

A reflog expression must never be stored as durable identity because:

* It is local to a repository’s ref history.
* Its numeric position can change as entries expire.
* It may not exist in another clone.
* It describes how to find an object, not the object itself.

---

### 4.5 Revision expressions

Parent, ancestry, and peel syntax may be accepted for ephemeral selection:

```text
main^
main^2
main~3
v2.1^{commit}
v2.1^{}
```

They must be resolved immediately.

The original expression may be retained only as non-authoritative provenance for display or diagnostics.

---

### 4.6 Tree and path expressions

Expressions such as:

```text
HEAD:README.md
main:src
:2:src/parser.rs
```

resolve to blobs or trees rather than commits.

They must be rejected in commit-valued roles.

They may be accepted only by a future command whose interface explicitly requires a blob, tree, or index entry.

Git’s `rev:path` and index-stage syntaxes are object-selection forms distinct from commit selection.

---

### 4.7 Revision ranges

Expressions such as:

```text
A..B
A...B
^A B
```

denote commit sets, not one commit.

They must not be accepted where a single cut, target, top, or descriptor is required.

A command that intends to accept a range must expose that fact explicitly:

```console
git staircase discover --range main..feature
```

or:

```console
git staircase step create --commits main..feature
```

The positional staircase selector must never silently reinterpret a range as a staircase name.

---

### 4.8 Refspecs

A refspec such as:

```text
refs/staircases/*:refs/remotes/origin/staircases/*
```

is a transport mapping, not an entity name.

Refspecs are accepted only in fetch, push, import, export, or configuration contexts.

Git defines a refspec as a source and optional destination mapping used to update refs.

---

### 4.9 Pathspecs

Pathspecs are never staircase names.

A command accepting both staircase selectors and pathspecs must separate them with `--`:

```console
git staircase diff auth -- src/parser.rs
```

---

## 5. Staircase Selector Resolution

### 5.1 Explicitly typed selectors

The following forms are authoritative and should be preferred by scripts:

```console
git staircase show --ref refs/staircases/auth
git staircase show --name auth
git staircase show --id <lineage-id>
git staircase show --revision <descriptor-oid>
git staircase discover --top <commit-ish>
git staircase show-step --step-id <step-id>
```

Typed selectors do not participate in cross-type guessing.

They must still undergo validation and type checking.

---

### 5.2 Full staircase refname

A positional selector beginning with:

```text
refs/staircases/
```

must be interpreted as an exact managed staircase refname.

It must not fall through to ordinary suffix-based ref matching.

The referenced object must:

1. Exist.
2. Have the expected descriptor object type.
3. Begin with the staircase descriptor signature.
4. Parse under a supported schema version.
5. Pass descriptor integrity checks.

---

### 5.3 Qualified staircase refname

A positional selector beginning with:

```text
staircases/
```

must be expanded to:

```text
refs/staircases/
```

and then handled as an exact managed staircase refname.

Example:

```text
staircases/auth
```

becomes:

```text
refs/staircases/auth
```

---

### 5.4 Bare selector algorithm

For a bare positional selector such as:

```text
auth
```

the command must independently evaluate the following candidate interpretations:

1. Managed staircase name:

   ```text
   refs/staircases/auth
   ```

2. Standard Git revision expression:

   ```text
   auth
   ```

3. Unique implicit staircase display name:

   ```text
   auth (implicit)
   ```

4. A supported explicitly recognizable structural revision key.

The command then applies these rules:

* If no candidate resolves, fail.
* If exactly one candidate resolves, use it.
* If several candidates resolve to the same staircase revision, use that staircase and report the canonical form when appropriate.
* If several candidates resolve to different entities, fail with an ambiguity report.
* Never select based solely on discovery order.

Example:

```text
error: selector 'auth' is ambiguous

managed staircase:
  refs/staircases/auth
  lineage: 3d7d16d1-...

Git revision:
  refs/heads/auth
  commit: 6ab192f...

implicit staircase:
  auth@7ba91cd
  top: c05a881...

use one of:
  git staircase show --name auth
  git staircase discover --top refs/heads/auth
  git staircase show --structural-key <full-key>
```

---

### 5.5 OID selectors

An OID-looking positional selector must not be assumed to be a commit.

The tool must inspect the referenced object:

* A valid staircase descriptor object denotes an exact managed staircase revision.
* A commit may denote a top or cut for implicit discovery.
* A tag object must be peeled or interpreted according to the command’s required type.
* A tree or blob that is not a valid descriptor must be rejected for staircase-valued commands.

An abbreviated OID is acceptable only if Git resolves it uniquely in the current repository.

Machine-readable output must return the full OID.

---

### 5.6 Ref versus OID preservation

When a user supplies a ref-valued expression, the command should retain both:

```text
symbolic input: refs/remotes/origin/main
resolved OID:   91f3b4c...
```

These values answer different questions:

* The full refname records the moving target selected by the user.
* The resolved OID records the exact target used for the current operation.

When a user supplies a non-ref expression such as:

```text
main~3
```

the command may record that expression as provenance, but the authoritative resolved value is the full OID.

A non-ref revision expression must not later be re-evaluated automatically as though it were a tracked target.

---

## 6. Staircase Name Rules

### 6.1 Validation

A canonical staircase name `<name>` is valid only if:

```console
git check-ref-format "refs/staircases/<name>"
```

accepts the resulting full refname.

Git refnames permit hierarchical slash-separated components but prohibit, among other things:

* Components beginning with `.`.
* Components ending in `.lock`.
* Consecutive dots.
* ASCII control characters.
* Spaces.
* `~`, `^`, `:`, `?`, `*`, `[`, and backslash.
* Leading, trailing, or repeated slashes.
* A trailing dot.
* The sequence `@{`.
* The single-character name `@`.

`git staircase` must not define a conflicting alternative refname grammar.

---

### 6.2 No silent normalization

An explicitly supplied name must not be silently:

* Lowercased.
* Case-folded.
* Unicode-normalized.
* Whitespace-trimmed.
* Slash-collapsed.
* Punctuation-stripped.
* Converted to a slug.
* Reduced to a branch-name stem.

If the exact requested name is invalid, creation must fail or require an explicit normalization option.

The normalized form produced by `git check-ref-format --normalize` may be offered diagnostically, but it must not be adopted without explicit user intent.

---

### 6.3 Encoding and portability

Git does not define one universal refname character encoding, although UTF-8 is preferred because output-processing tools may assume it.

Accordingly:

* Staircase names are compared as exact refname byte sequences.
* The tool must not apply Unicode canonical equivalence.
* Names are case-sensitive at the conceptual level.
* Implementations should warn about names that differ only by case or Unicode normalization because filesystem and tooling behavior may vary.
* Portable automation should prefer a conservative UTF-8 subset, typically ASCII letters, digits, hyphens, underscores, dots, and slashes.

These portability warnings do not change identity.

---

### 6.4 Hierarchical names

Names may be hierarchical:

```text
auth
auth/oauth
team-a/payments/refactor
release/2027-cleanup
```

The hierarchy is organizational only.

It does not imply:

* Staircase parentage.
* Shared lineage.
* Shared integration target.
* A staircase family.
* Step containment.

Relationships must be represented explicitly in staircase descriptors.

---

### 6.5 Ref prefix conflicts

The public staircase namespace must use leaf refs only:

```text
refs/staircases/<name>
```

No implementation metadata may be placed beneath a public staircase ref:

```text
refs/staircases/auth/steps/1
```

This would create a prefix conflict with:

```text
refs/staircases/auth
```

on ref backends that represent refs through filesystem-like paths.

Internal state must use a separate namespace.

---

### 6.6 Name uniqueness

A full staircase refname identifies at most one current managed staircase revision.

Creating:

```text
refs/staircases/auth
```

must fail if that ref already exists, unless an explicit replacement operation is requested and protected by an expected old lineage or descriptor OID.

A name may be reused after deletion, but the new staircase remains distinguishable by lineage ID.

The tool should warn when reflog or retained history indicates that the name previously referred to another lineage.

---

### 6.7 Zero or one canonical name

A managed staircase has:

* Zero canonical names, or
* One canonical name.

The core model does not permit several mutable canonical aliases for one lineage.

This avoids uncertain behavior when:

* Only one alias is pushed.
* One alias is updated and another is not.
* An alias is renamed independently.
* Remote copies diverge.
* A descriptor update must move several public refs.

A future alias facility may be added separately, but aliases must not be confused with canonical names.

---

## 7. Implicit Staircase Names

### 7.1 Implicit names are labels

An implicit staircase has no persistent canonical name.

Discovery may assign a display label derived from:

* A common branch-name stem.
* The top branch.
* A configured naming convention.
* A review-system topic.
* A generated structural key.

Example:

```text
auth (implicit)
```

This label is not a refname and is not lineage identity.

---

### 7.2 Deterministic disambiguation

If several implicit staircases receive the same display label, the tool must append a deterministic structural disambiguator:

```text
auth@7ba91cd (implicit)
auth@18ce204 (implicit)
```

The abbreviation must be long enough to distinguish the displayed candidates.

Machine-readable output must include the full structural revision key.

---

### 7.3 Adoption of a named implicit staircase

When an implicit staircase is explicitly adopted with a requested name:

```console
git staircase adopt --name auth <selector>
```

the name must pass the ordinary staircase-name rules.

When automatic adoption occurs without an explicit name:

* A unique valid implicit display label may be proposed as the canonical name.
* The label must not be silently normalized.
* The label must not be used if the corresponding ref already exists.
* If no safe unique name exists, the staircase must be adopted without a canonical name.

Example:

```text
adopted implicit staircase
lineage: 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
name: none
```

This is preferable to manufacturing a misleading or collision-prone name.

---

## 8. Persistent Ref Namespace

### 8.1 Public name ref

A named managed staircase is represented by:

```text
refs/staircases/<name>
```

This ref points to the immutable descriptor object for the current staircase revision.

Example:

```text
refs/staircases/auth
    -> <descriptor OID>
```

The referenced object is not the aggregate top commit.

Commands must read and validate the descriptor to obtain:

* Lineage ID.
* State.
* Integration boundary.
* Ordered step IDs.
* Cut OIDs.
* Aggregate top, when materialized.
* Policies and mappings.

---

### 8.2 Stable lineage state namespace

Every managed staircase has an internal namespace:

```text
refs/staircase-state/<lineage-id>/
```

Required refs are:

```text
refs/staircase-state/<lineage-id>/descriptor
refs/staircase-state/<lineage-id>/steps/<step-id>
```

The descriptor ref points to the same descriptor object as the public name ref, when a public name exists.

Each step ref points directly to the current cut commit for that step.

Optional refs may include:

```text
refs/staircase-state/<lineage-id>/target
refs/staircase-state/<lineage-id>/operation-base
refs/staircase-state/<lineage-id>/recovery/<operation-id>/<step-id>
```

These refs exist for:

* Reachability.
* Recovery.
* Exact lookup by lineage.
* Stale and incomplete state representation.
* Step continuity.
* Ref-level inspection.

---

### 8.3 Descriptor authority

The immutable descriptor object is the authoritative snapshot of a staircase revision.

Readers must:

1. Resolve the public or lineage descriptor ref once.
2. Read the resulting immutable descriptor object.
3. Use the full OIDs embedded in that descriptor.
4. Treat mutable internal refs as reachability, recovery, and consistency aids.

Readers must not reconstruct one logical read by independently reading several mutable step refs.

Git reference transactions update refs safely and can verify expected old values, but Git notes that a concurrent reader may still observe only a subset of a multi-ref transaction on some ref backends.

An immutable descriptor prevents such a reader from assembling a half-old, half-new staircase.

---

### 8.4 Descriptor object type

The initial representation is a canonical Git blob.

The blob must begin with an identifying header such as:

```text
git-staircase-descriptor 1
```

A blob is used because:

* Its OID is a pure function of canonical descriptor bytes.
* It is immutable.
* A ref may point directly to it.
* It can be inspected with ordinary Git object tools.
* It does not falsely masquerade as a source-code commit.

The descriptor OID is the staircase revision ID.

---

### 8.5 Canonical descriptor serialization

The descriptor serialization must be:

* Versioned.
* Deterministic.
* Unambiguous.
* Independent of map iteration order.
* Independent of locale.
* Independent of current time unless time is semantically part of the revision.
* Independent of canonical staircase name.
* Independent of local path names.
* Explicit about object format.
* Explicit about optional and repeated fields.

The canonical name is excluded because renaming a ref must not create a new staircase revision.

A conceptual descriptor contains:

```text
git-staircase-descriptor 1
object-format sha256
lineage 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
state clean
target-ref refs/remotes/origin/main
target-oid <full-oid>
parent-revision <full-descriptor-oid>

step <step-id-1>
cut <full-commit-oid-1>
materializing-ref refs/heads/feature/auth-core

step <step-id-2>
cut <full-commit-oid-2>
materializing-ref refs/heads/feature/auth-ui
```

The final wire grammar should be specified separately.

---

### 8.6 Reachability

Because OIDs appearing as text inside a blob do not by themselves create ordinary Git object-graph reachability, every current cut that must be retained must also be reachable through a direct state ref.

The internal step refs fulfill this requirement.

Recovery refs retain temporary or superseded cuts when necessary.

---

## 9. Production of Identities

### 9.1 Lineage creation

When a staircase is first adopted:

1. Generate a random lineage UUID.
2. Check that no internal state namespace already exists for it.
3. Generate stable step IDs for all current steps.
4. Create the initial descriptor.
5. Create internal refs.
6. Create the public name ref if a safe canonical name is available.
7. Record reflogs.
8. Commit the ref transaction.

The lineage ID must not be regenerated during normal mutation.

---

### 9.2 Revision creation

A new staircase revision is created by:

1. Resolving every required input to full OIDs and full refnames.
2. Constructing the complete logical post-operation state.
3. Canonically serializing the descriptor.
4. Writing the descriptor blob.
5. Using the blob’s full OID as the staircase revision ID.
6. Updating refs only after the descriptor object exists.

Two independently produced descriptors with identical canonical content must have the same OID under the same Git object format.

---

### 9.3 Step creation

A new conceptual step receives a new random step ID.

A step ID must never be inferred from:

* Its ordinal.
* Its branch name.
* Its cut OID.
* Its patch ID.
* Its review ID.

Those values may change while the conceptual step persists.

---

### 9.4 Split identity rule

Splitting one managed step creates two conceptual steps.

There is no mathematically unique answer to which child remains “the original” step. The operation therefore requires an explicit deterministic policy.

The default is:

* The child ending at the original cut retains the original step ID.
* The newly inserted lower child receives a new step ID.

Example:

```text
before:
  step X: C..E
  cut: E

after split at D:
  step Y: C..D
  step X: D..E
```

This default preserves the identity attached to the pre-existing terminal cut and minimizes movement of existing review associations.

The user may override the disposition explicitly:

```console
git staircase split auth:2 --at D --keep-id=lower
```

The original ID must never be assigned to both children.

---

### 9.5 Join identity rule

Joining adjacent managed steps produces one conceptual step.

The default is:

* The later step’s ID survives.
* The earlier step’s ID is retired.

Example:

```text
before:
  step X: C..D
  step Y: D..E

after:
  step Y: C..E
```

The later step is retained because its terminal cut remains the joined step’s terminal cut.

An explicit option may select the surviving identity:

```console
git staircase join auth:2 auth:3 --keep-id=<step-id>
```

The retired ID may be recorded in revision history but must not remain active.

---

### 9.6 Drop identity rule

Dropping a step retires its step ID.

The ID must not be silently reused for a later unrelated step.

---

### 9.7 Reorder identity rule

Reordering moves step IDs with the conceptual changes they represent.

Ordinals are recomputed after the operation.

---

### 9.8 Revision-derived identities

Body, decomposition, patch-series, and outcome identities are typed derived digests.

They must not be presented as Git OIDs unless they are actually OIDs of stored Git objects.

Recommended machine forms include:

```text
body:sha256:<digest>
decomposition:sha256:<digest>
outcome:sha256:<digest>
patch-series:sha256:<digest>
```

These typed values are staircase-domain identities.

They must be selected through typed options rather than passed to ordinary Git revision parsing.

---

## 10. Name Production and Mutation

### 10.1 Create

Creating a name means creating:

```text
refs/staircases/<name>
```

with an expected-old value asserting that the ref does not exist.

The new value is the current descriptor OID.

---

### 10.2 Rename

Renaming a staircase changes its refname, not its lineage or revision.

A rename from `auth` to `authentication` must atomically:

1. Verify that:

   ```text
   refs/staircases/auth
   ```

   still points to the expected descriptor OID.

2. Verify that:

   ```text
   refs/staircases/authentication
   ```

   does not exist.

3. Create the new ref with the same descriptor OID.

4. Delete the old ref.

5. Write descriptive reflog messages.

The descriptor object does not change.

---

### 10.3 Unname

Removing the canonical name deletes only:

```text
refs/staircases/<name>
```

The managed staircase remains reachable and selectable through:

```text
refs/staircase-state/<lineage-id>/descriptor
```

and its lineage ID.

Unnaming is distinct from deleting the managed staircase.

---

### 10.4 Delete

Deleting a managed staircase removes:

* Its public name ref, if any.
* Its lineage descriptor ref.
* Its active step refs.
* Its policy and recovery refs, subject to retention policy.

Deletion does not directly delete Git objects.

Ordinary Git reachability, reflogs, and garbage collection govern eventual object removal.

---

### 10.5 Snapshot names

A user may assign an immutable human name to one exact staircase revision by creating an annotated Git tag:

```text
refs/tags/staircase/<snapshot-name>
```

The annotated tag targets the staircase descriptor blob.

Example:

```console
git staircase tag auth-before-api-change auth
```

Conceptually:

```text
refs/tags/staircase/auth-before-api-change
    -> annotated tag object
        -> staircase descriptor blob
```

Annotated tags may carry messages and cryptographic signatures, making them suitable for named immutable snapshots. Git permits tags to refer to objects, not only commits.

Snapshot tags must not move unless the user explicitly requests force replacement.

A snapshot tag is not the mutable staircase name.

---

## 11. Ref Update Safety

### 11.1 Compare-and-swap

Every mutation must verify expected old values.

The semantic model is:

```text
update <ref> <new-oid> <expected-old-oid>
```

A mutation must fail if another process moved the ref after the command read it.

Git’s `update-ref` supports this compare-and-swap behavior directly.

---

### 11.2 Reference transactions

Operations that modify several refs must use one Git reference transaction where supported.

The transaction may include:

* Public staircase-name refs.
* The lineage descriptor ref.
* Step refs.
* Recovery refs.
* Symbolic refs, if introduced in the future.

`git update-ref --stdin` supports grouped regular and symbolic ref updates with explicit `start`, `prepare`, `commit`, and `abort` operations.

---

### 11.3 Descriptor-first publication

A mutation must follow this order:

1. Write all new commit and descriptor objects.
2. Prepare the complete new descriptor.
3. Begin the ref transaction.
4. Verify all expected old refs.
5. Update reachability and step refs.
6. Update the lineage descriptor ref.
7. Update the public name ref.
8. Commit the transaction.

A ref must never point to an object that has not already been written successfully.

---

### 11.4 Reflogs

The implementation should create reflogs for:

* Public staircase-name refs.
* Lineage descriptor refs.
* Active step refs.
* Operation recovery refs where useful.

Ref updates should include meaningful reasons:

```text
staircase: rebase onto origin/main
staircase: split step 2 at <oid>
staircase: rename auth to authentication
staircase: restack from step 1
```

Git can create reflogs explicitly during `update-ref` operations.

---

### 11.5 Direct ref-file access is prohibited

The implementation must not assume refs live as loose files under `.git/refs`.

Refs may be loose, packed, or stored through another ref backend.

The implementation must use Git’s ref APIs or plumbing commands.

Git’s documentation specifically encourages `git show-ref` rather than direct access to files under `.git`.

---

## 12. Transport

### 12.1 Custom refs require explicit transport

Staircase refs are not ordinary branches or tags and must not be assumed to travel under default fetch or push configuration.

Transport should use explicit refspecs or a staircase-aware wrapper.

Example public-name mapping:

```text
refs/staircases/*:
refs/remotes/origin/staircases/*
```

Example internal-state mapping:

```text
refs/staircase-state/*:
refs/remotes/origin/staircase-state/*
```

A complete implementation must construct valid source and destination refspecs rather than relying on textual examples alone.

Git push can update branches, tags, and other refs, and refspecs define the source object and destination ref.

---

### 12.2 Remote-tracking staircase refs

Fetched remote staircase names should normally be stored under:

```text
refs/remotes/<remote>/staircases/<name>
```

Fetched remote internal state may be stored under:

```text
refs/remotes/<remote>/staircase-state/<lineage-id>/...
```

Remote-tracking staircase refs are observations of remote state.

They must not be treated as locally managed mutable staircases until explicitly imported, checked out, adopted, or otherwise materialized according to command semantics.

---

### 12.3 Lineage collision

If imported state uses a lineage ID already present locally:

* If both sides identify the same descriptor revision, the states agree.
* If one revision is a known descendant of the other, the operation may fast-forward according to policy.
* If the descriptor histories differ, the lineage is diverged.
* The tool must not silently assign a new lineage ID.
* The tool must not silently overwrite either side.

A lineage collision with incompatible ancestry is a semantic conflict, not a random-ID collision to be papered over.

---

### 12.4 Name collision

A remote and local staircase may share a canonical name while having different lineage IDs.

The name collision must not cause lineage merging.

The import operation may:

* Retain the remote-tracking name only.
* Require a new local name.
* Import the staircase unnamed.
* Replace the local name only with explicit force and lease checks.

---

### 12.5 Atomic remote updates

When a staircase update requires several remote refs to change together, `git staircase push` should request atomic push when the remote supports it.

If the remote does not support the required atomic behavior, the tool must either:

* Refuse the operation.
* Use a protocol that publishes the descriptor last and supports safe recovery.
* Clearly report that partial remote publication is possible.

It must not claim atomic publication when the remote cannot provide it.

---

## 13. Output Rules

### 13.1 Human-readable output

Human output may abbreviate:

* OIDs.
* Structural revision keys.
* Lineage IDs.
* Step IDs.

The abbreviation must be sufficient to distinguish all relevant displayed candidates.

Example:

```text
auth
  ref:       staircases/auth
  lineage:   3d7d16d1
  revision:  982cc1c
  top:       c05a881
```

Human output must label the type of each value.

A naked hexadecimal string is insufficient when it could mean:

* Descriptor OID.
* Commit OID.
* Body digest.
* Outcome digest.
* Structural key.

---

### 13.2 Machine-readable output

Machine-readable output must include full typed fields.

Example:

```json
{
  "management": "managed",
  "name": "auth",
  "refname": "refs/staircases/auth",
  "lineage_id": "3d7d16d1-14c8-4d86-a55d-9ce54094bc25",
  "revision": {
    "object_format": "sha256",
    "oid": "<full-descriptor-oid>",
    "object_type": "blob"
  },
  "state": "clean",
  "target": {
    "refname": "refs/remotes/origin/main",
    "oid": "<full-target-oid>"
  },
  "top_oid": "<full-top-commit-oid>",
  "steps": [
    {
      "id": "<full-step-id>",
      "ordinal": 1,
      "cut_oid": "<full-cut-oid>",
      "materializing_ref": "refs/heads/feature/auth-core"
    }
  ]
}
```

Machine output must not depend on display abbreviations.

---

### 13.3 Canonicalization command

The command suite should provide a plumbing-like canonicalizer:

```console
git staircase rev-parse <selector>
```

Useful options include:

```console
git staircase rev-parse --ref <selector>
git staircase rev-parse --lineage <selector>
git staircase rev-parse --revision <selector>
git staircase rev-parse --top <selector>
git staircase rev-parse --step <selector>
git staircase rev-parse --format=json <selector>
```

For a managed staircase, canonical output should include:

```text
refname
lineage ID
descriptor OID
descriptor object format
state
top commit OID, if defined
```

For an implicit staircase, it should include:

```text
management: implicit
structural revision key
resolved target OID
ordered cut OIDs
derived display name, if any
```

---

## 14. Shell and Input Safety

### 14.1 Option termination

Commands accepting arbitrary Git names must support `--` to terminate option parsing.

Example:

```console
git staircase show -- --strange-but-valid-selector
```

Names that begin with `-` should be supplied through a typed option or after `--`.

---

### 14.2 No shell reinterpretation

User-supplied selectors and refnames must be passed as data.

They must not be:

* Evaluated as shell code.
* Interpolated into shell command strings.
* Treated as filesystem paths.
* Split on whitespace.
* Expanded as globs unless the command explicitly accepts patterns.

---

### 14.3 NUL-safe plumbing

When invoking Git plumbing for batches of arbitrary refs, implementations should use NUL-delimited input and output where available.

This avoids ambiguity from quoting and unusual valid refname bytes.

---

### 14.4 Pattern versus literal names

Commands that accept patterns must distinguish them from literal names.

Examples:

```console
git staircase list --pattern 'team-a/*'
git staircase show --name 'team-a/auth'
```

A literal selector must never acquire wildcard meaning merely because it contains pattern-like characters.

Git’s refname grammar already prohibits several wildcard characters in actual refnames, but command interfaces should retain the literal-versus-pattern distinction.

---

## 15. Command Examples

### 15.1 Select by managed name

```console
git staircase show auth
git staircase show staircases/auth
git staircase show refs/staircases/auth
git staircase show --name auth
git staircase show --ref refs/staircases/auth
```

The typed and qualified forms are preferred when ambiguity matters.

---

### 15.2 Select by lineage

```console
git staircase show \
    --id 3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

---

### 15.3 Select exact staircase revision

```console
git staircase show --revision <full-descriptor-oid>
```

This does not follow the current staircase ref.

---

### 15.4 Use ordinary Git names as targets and cuts

```console
git staircase rebase auth --onto origin/main

git staircase split auth:2 --at HEAD~3

git staircase discover \
    --onto refs/remotes/origin/main \
    --top feature/auth-tests
```

Each commit-valued expression is resolved and verified as a commit.

---

### 15.5 Rename without changing revision

```console
git staircase rename auth authentication
```

Result:

```text
deleted: refs/staircases/auth
created: refs/staircases/authentication
lineage: unchanged
revision: unchanged
```

---

### 15.6 Create an immutable snapshot tag

```console
git staircase tag auth-before-rebase auth
```

Result:

```text
refs/tags/staircase/auth-before-rebase
    -> annotated tag
        -> exact staircase descriptor object
```

---

### 15.7 Disambiguate a branch and staircase with the same bare name

```text
$ git staircase show auth

error: selector 'auth' is ambiguous

managed staircase:
  refs/staircases/auth

branch:
  refs/heads/auth

use:
  git staircase show --name auth
or:
  git staircase discover --top refs/heads/auth
```

---

## 16. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 16.1 Names are refs

A canonical managed staircase name is represented by exactly one full refname:

```text
refs/staircases/<name>
```

---

### 16.2 Revisions are immutable objects

One exact managed staircase state is represented by one immutable descriptor object whose full OID is the staircase revision ID.

---

### 16.3 Lineage is not content-derived

The lineage ID remains stable when names, commits, cuts, or descriptors change.

---

### 16.4 Step identity is not position

A step ID remains distinct from its current ordinal.

---

### 16.5 Full forms are persisted

Persistent state contains full refnames and full OIDs.

It never relies on abbreviations or DWIM resolution.

---

### 16.6 Stored OIDs are typed

Every stored OID has a defined semantic type, including:

* Descriptor blob.
* Commit cut.
* Target commit.
* Verification artifact.
* Derived non-Git digest.

---

### 16.7 One logical read uses one descriptor revision

A reader must not assemble a staircase revision by racing across independently mutable refs.

---

### 16.8 Mutations are lease-protected

Every changed ref is updated against an expected old value.

---

### 16.9 Rename does not rewrite identity

Renaming a staircase changes its public refname but not its:

* Lineage ID.
* Descriptor OID.
* Step IDs.
* Cut OIDs.
* Outcome identity.

---

### 16.10 Implicit labels are not promoted accidentally

A discovered label does not become a canonical refname unless adoption explicitly or safely creates that ref.

---

### 16.11 No hidden resolution precedence

When a bare selector could denote distinct staircase and Git entities, the command reports ambiguity.

---

## 17. Summary Model

The naming model consists of five distinct layers:

### Human name

```text
auth
```

A mutable user-facing name.

### Canonical refname

```text
refs/staircases/auth
```

The Git-native movable name for the managed staircase.

### Lineage ID

```text
3d7d16d1-14c8-4d86-a55d-9ce54094bc25
```

The stable identity of the evolving staircase.

### Staircase revision OID

```text
<full descriptor blob OID>
```

The immutable identity of one exact staircase state.

### Commit and step identities

```text
step ID: <UUID>
cut OID: <full commit OID>
```

The step ID follows the conceptual review step. The cut OID identifies its exact current cumulative commit.

The governing rule is:

> Accept Git’s rich naming language at command boundaries, resolve it immediately, type-check it, and reduce it to full refs, full OIDs, and explicit staircase identities before storing or transporting anything.

Or, in the compact Git-shaped form:

> Refs name evolving things. OIDs name immutable things. Lineage IDs connect immutable revisions into one continuing staircase.
