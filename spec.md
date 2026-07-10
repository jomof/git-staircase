# Git Staircase: Conceptual Specification

## 1. Status

This document defines a proposed first-class abstraction called a **staircase** for Git repositories.

A staircase represents a review-oriented body of work that may span multiple commits and multiple local branches. It is intended to support workflows in which related changes are reviewed, reshaped, tested, rebased, and landed as an ordered whole or as a sequence of dependent review steps.

The command name is `git staircase`. It is a git extension.

This document focuses on the conceptual model and user-visible semantics. It does not yet prescribe a storage format, implementation language, or complete command-line interface.

---

## 2. Motivation

Git provides first-class concepts for commits, branches, tags, trees, and repositories, but it does not provide a first-class abstraction for a review-oriented stack of dependent changes.

In many development workflows, a body of work has the following properties:

* It consists of multiple commits.
* It is intended to be reviewed as one or more ordered units.
* Some review units depend on earlier review units.
* The units may be represented by several local branches.
* The entire body of work is expected to build and test successfully when landed.
* Individual prefixes may also be expected to build and test successfully.
* The work may be rebased, amended, split, joined, reordered, or otherwise reshaped during review.
* The useful identity of the work may need to survive some of those transformations.

A staircase introduces a higher-level abstraction above individual commits and branches.

---

## 3. Terminology

### 3.1 Commit graph

A Git repository contains a directed acyclic graph of commits.

Let:

[
G = (V, E)
]

where:

* (V) is the set of commits.
* (E) contains parent relationships between commits.

For commits (x) and (y):

[
x \prec y
]

means that (x) is an ancestor of (y).

---

### 3.2 Integration boundary

An **integration boundary** identifies history that is considered already upstream or already integrated.

It is represented conceptually as an ancestor-closed set of commits:

[
U \subseteq V
]

If a commit is in (U), all of its ancestors are also in (U).

Common integration boundaries include:

* The commits reachable from `origin/main`.
* The commits reachable from a release branch.
* The union of commits reachable from several upstream refs.
* A resolved upstream commit together with all of its ancestors.

A staircase is always interpreted relative to an integration boundary.

The integration boundary is part of the staircase definition. The same branch or commit tip may describe a different staircase relative to a different boundary.

---

### 3.3 Cut

A **cut** is a commit that marks the cumulative end of a review step.

For a staircase with (k) steps, its cuts are:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

Each cut includes all earlier work in the staircase.

The final cut (a_k) is the aggregate top of the staircase.

---

### 3.4 Prefix

The cumulative prefix associated with cut (a_i) is:

[
P_i = \operatorname{Ancestors}(a_i) \setminus U
]

where (\operatorname{Ancestors}(a_i)) includes (a_i).

Each prefix contains all staircase commits included through step (i).

---

### 3.5 Step

A **step** is the difference between two consecutive cumulative prefixes.

For the first step:

[
S_1 = P_1
]

For later steps:

[
S_i = P_i \setminus P_{i-1}
]

Each step must contain at least one commit.

A step is not necessarily a single commit. It may contain any nonempty sequence or region of commits between two cuts.

---

### 3.6 Body

The **body** of a staircase is the complete set of commits belonging to it:

[
B = P_k
]

Equivalently:

[
B = S_1 \cup S_2 \cup \dots \cup S_k
]

The body represents all work introduced by the staircase relative to its integration boundary.

---

### 3.7 Staircase

A **staircase** is an ordered sequence of cumulative cuts over a dependency-closed commit region relative to an integration boundary.

Formally:

[
\mathcal{S} = (U; a_1, a_2, \dots, a_k)
]

subject to:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

and:

[
S_i \neq \varnothing
]

for all steps (i).

A staircase therefore defines:

* An integration boundary.
* An aggregate body.
* An ordered set of cuts.
* An ordered decomposition of the body into review steps.

---

### 3.8 One-step staircase

A **one-step staircase** is a staircase with exactly one cut:

[
\mathcal{S} = (U; a_1)
]

A normal topic branch containing a stack of commits is therefore a degenerate staircase with one review step.

The number of commits and the number of steps are independent.

A one-step staircase may contain many commits.

---

### 3.9 Staircase family

A **staircase family** is a connected dependency structure that contains multiple possible staircase paths.

For example:

```text
           ui
          /
core ----<
          \
           cli
```

Both `ui` and `cli` depend on `core`, but neither depends on the other.

This structure is not one linear staircase. It may be represented as:

* Two staircases sharing a common prefix.
* One staircase family containing two paths.

A staircase remains strictly linear. Forked dependency structures belong to the broader staircase-family abstraction.

---

### 3.10 Materialized staircase

A **materialized staircase** is one whose cuts currently exist as a valid ancestry chain in the repository:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

A materialized staircase can be checked out, diffed, tested, and traversed using ordinary Git objects.

---

### 3.11 Managed staircase

A **managed staircase** has persistent metadata that records its intended identity and shape.

A managed staircase may temporarily cease to be materialized.

For example, after amending or rebasing a lower step, an upper branch may still depend on the previous version of that step. The intended staircase still exists, but it is stale and requires restacking.

A managed staircase may retain:

* A stable lineage identifier.
* A human-readable name.
* Its intended target.
* Its ordered steps.
* Refs or other locators for current step tips.
* Review metadata.
* Verification policy.
* Landing policy.

---

### 3.12 Discovered staircase

A **discovered staircase** is inferred from existing Git structure without requiring prior staircase metadata.

Discovery may use:

* Commit ancestry.
* Local branch tips.
* Remote-tracking branch tips.
* Naming conventions.
* Configured target branches.
* Review metadata.
* User-supplied constraints.

Discovery is descriptive rather than authoritative.

A discovered staircase may later be adopted as a managed staircase.

---

## 4. Core Model

### 4.1 A staircase is not a branch

A branch is a movable ref pointing to one commit.

A staircase is a higher-level object composed of:

* A target boundary.
* A body of commits.
* Multiple cumulative cuts.
* An ordered decomposition into steps.
* Optional persistent identity and policy.

A single branch may represent the aggregate top of a staircase, but it does not by itself describe internal review boundaries.

---

### 4.2 A staircase is not merely a set of commits

An unordered set of commits does not capture:

* Dependency ordering.
* Review boundaries.
* Cumulative prefixes.
* The intended landing target.
* The distinction between splitting and joining.
* Whether intermediate prefixes are meaningful.

The staircase body must be dependency-closed relative to its integration boundary, and its cuts must be ordered by ancestry.

---

### 4.3 A staircase is a filtered commit region

A staircase can be understood as a filtration:

[
\varnothing = P_0 \subset P_1 \subset P_2 \subset \dots \subset P_k
]

Each cumulative prefix adds one nonempty review step:

[
S_i = P_i \setminus P_{i-1}
]

This filtration is the structural core of the staircase abstraction.

---

## 5. Relationship to Branches

### 5.1 Branches may materialize cuts

A common staircase representation uses one branch per cut.

Example:

```text
M---A---B---C---D---E---F
^           ^       ^   ^
origin/main core    ui  tests
```

Refs:

```text
origin/main         -> M
feature/auth-core   -> C
feature/auth-ui     -> E
feature/auth-tests  -> F
```

This defines:

```text
Step 1: M..C = {A, B, C}
Step 2: C..E = {D, E}
Step 3: E..F = {F}
```

The refs expose staircase cuts, but the branches are not themselves the staircase.

---

### 5.2 Cuts may have zero, one, or several refs

A cut may be:

* Named by one local branch.
* Named by several equivalent refs.
* Named only by persistent staircase metadata.
* Temporarily unnamed.
* Recoverable only through reflogs or stored object IDs.

The conceptual model must not require exactly one branch per step.

---

### 5.3 Branch names are aliases

A naming convention such as:

```text
auth
auth-2
auth-3
```

or:

```text
auth-core
auth-ui
auth-tests
```

may help discover and display a staircase.

A normalized base such as `auth` is a useful nominal identity, but it is not sufficient as canonical identity because:

* Naming conventions vary.
* Branches may be renamed.
* Different repositories may use different ref namespaces.
* Distinct staircases may normalize to the same name.
* A clone may not contain the same local refs.

---

## 6. Discovery

### 6.1 Discovery inputs

A discovery operation may examine:

* A selected integration boundary.
* Local branch tips.
* Remote branch tips.
* Explicit commit tips.
* Existing staircase metadata.
* Ref naming patterns.
* Review identifiers embedded in commits.
* Previously recorded lineage relationships.

---

### 6.2 Minimal structural criterion

Two branch tips may belong to the same multi-step staircase only if one depends on the other by commit ancestry.

For branch tips (x) and (y), they may form consecutive or nonconsecutive cuts only when:

[
x \prec y
]

or:

[
y \prec x
]

Unrelated branch tips do not form one staircase solely because their names are similar.

---

### 6.3 Discovery as a partial order

The ancestry relation among candidate cuts forms a partially ordered set.

A staircase is a chain in that partial order.

A staircase family is a connected structure containing multiple chains.

Discovery must not silently linearize incomparable branches unless explicitly requested.

---

### 6.4 Discovery ambiguity

The following cases may be ambiguous:

* Multiple refs point to the same cut.
* Several chains share a prefix.
* A branch name suggests membership but ancestry does not.
* A lower step was rewritten and upper steps are stale.
* Several possible integration boundaries produce different staircases.
* Merge commits create multiple plausible decompositions.
* The same commits are covered by overlapping managed staircases.

The tool should report ambiguity rather than guessing invisibly.

Example:

```console
$ git staircase discover --onto origin/main

auth-family
  shared prefix:
    feature/auth-core

  paths:
    feature/auth-core -> feature/auth-ui
    feature/auth-core -> feature/auth-cli
```

---

### 6.5 Adoption

A discovered staircase may be converted into a managed staircase:

```console
git staircase adopt auth \
    --onto origin/main \
    feature/auth-core \
    feature/auth-ui \
    feature/auth-tests
```

Adoption establishes persistent intent that can survive temporary loss of structural ancestry.

---

## 7. Identity

### 7.1 Identity as an invariant

A staircase does not have one universally correct identity.

An identity is useful relative to a class of transformations.

For an identity function (I) and transformation (T):

[
I(x) = I(T(x))
]

when that identity is intended to treat (x) and (T(x)) as the same entity.

Different workflows require different invariants.

---

### 7.2 Lineage identity

A **lineage identity** represents the same evolving staircase across history-rewriting operations.

It should remain stable under operations such as:

* Rebase.
* Restack.
* Commit amendment.
* Commit-message edits.
* Branch renames.
* Internal squashing.
* Changes to exact commit object IDs.
* Changes to local ref names.

A lineage identity cannot reliably be derived from current commit objects alone.

It requires an intentionally preserved identifier, such as:

```text
refs/staircases/<uuid>
```

or an identifier stored in staircase metadata.

The lineage identity answers:

> Is this the same evolving staircase project?

---

### 7.3 Exact revision identity

An **exact revision identity** represents one exact materialization of a staircase.

It should include at least:

* The Git object format or hash algorithm.
* The resolved integration boundary.
* The ordered cut commit IDs.

Conceptually:

[
I_{\text{revision}} =
H(\text{object-format}, U, a_1, a_2, \dots, a_k)
]

It remains stable under:

* Branch renames.
* Cloning, when all referenced objects are preserved.
* Adding unrelated refs.

It changes under:

* Rebase.
* Amendment.
* Split.
* Join.
* Target-boundary change.
* Any cut commit change.

The exact revision identity answers:

> Is this precisely the same staircase materialization?

---

### 7.4 Body identity

A **body identity** represents the exact aggregate commit history of the staircase without necessarily identifying its internal decomposition.

A possible form is:

[
I_{\text{body}} = H(U, a_k)
]

where (a_k) is the staircase top.

Because a commit ID recursively commits to its ancestors, the top commit identifies the exact reachable history. The integration boundary is still required to identify which region of that history belongs to the staircase.

A body identity remains stable under:

* Branch renames.
* Adding or removing internal cut labels.
* Splitting or joining steps without rewriting commits.

It changes under:

* Commit rewriting.
* Aggregate top changes.
* Integration-boundary changes.

The body identity answers:

> Is this exactly the same aggregate commit body?

---

### 7.5 Decomposition identity

A **decomposition identity** represents the ordered aggregate change introduced by each step.

For each step:

```text
Step 1: diff(U, a1)
Step 2: diff(a1, a2)
...
Step k: diff(a{k-1}, ak)
```

The identity may be computed from an ordered sequence of normalized per-step patches.

It may remain stable under:

* Rebasing while preserving each step’s effective patch.
* Squashing or reorganizing commits within a step.
* Commit-message edits.
* Ref renaming.

It changes under:

* Splitting a step.
* Joining steps.
* Moving changes between steps.
* Substantively altering any step’s patch.

The decomposition identity answers:

> Does this staircase still present the same ordered review units?

---

### 7.6 Outcome identity

An **outcome identity** represents the aggregate resulting source state.

A simple form may use:

```text
base tree ID
top tree ID
```

or a normalized aggregate diff.

It may remain stable under:

* Rebase.
* Squash.
* Commit reordering that preserves the final tree.
* Step split or join.
* Commit-message changes.

It changes when the final source outcome changes.

The outcome identity answers:

> Does this staircase ultimately produce the same result?

---

### 7.7 Patch-series identity

A **patch-series identity** represents semantic similarity of a series of changes despite changes in exact commit structure.

It may be derived using concepts similar to Git patch IDs.

It may survive:

* Rebase.
* Cherry-pick.
* Commit metadata changes.
* Some context-line changes.

Its exact normalization rules must be explicitly defined because different normalizations may collapse changes that are operationally distinct.

The patch-series identity answers:

> Is this recognizably the same series of changes?

---

### 7.8 Nominal identity

A **nominal identity** is a human-readable name such as:

```text
auth
```

It may be inferred from branch naming or assigned explicitly.

It is useful for:

* Command-line selection.
* Display.
* Discovery.
* Team communication.

It is not sufficient as an intrinsic or globally unique identity.

---

### 7.9 Review identity

A **review identity** associates staircase steps with external code-review records.

Examples include:

* Gerrit change identifiers.
* GitHub pull requests.
* GitLab merge requests.
* Internal review-system records.

Review identity may survive local rebases even when exact commit IDs change.

The staircase abstraction must not be tied to one review system.

---

## 8. State Model

A managed staircase may be in one of several states.

### 8.1 Clean

All configured cuts form the intended ancestry chain.

```text
a1 ≺ a2 ≺ ... ≺ ak
```

All refs and metadata agree.

---

### 8.2 Stale

One or more lower steps have changed, but dependent upper steps still reference previous versions.

Example:

```text
C_old---D---E   feature/auth-ui
   \
    C_new      feature/auth-core
```

The intended relationship remains known through managed metadata, but the current tips no longer form one materialized staircase.

---

### 8.3 Diverged

A step has multiple incompatible current candidates, or refs and metadata disagree about which commit represents the step.

---

### 8.4 Incomplete

One or more intended steps have no current commit tip or cannot be resolved.

---

### 8.5 Ambiguous

The repository contains more than one structurally plausible interpretation.

---

### 8.6 Partially landed

Some lower prefixes have been integrated into the target, while upper steps remain local or under review.

The staircase body and cuts must be recomputed relative to the updated integration boundary.

---

### 8.7 Verified

The staircase has current verification evidence for a particular:

* Staircase revision.
* Resolved target.
* Verification profile.

---

### 8.8 Verification-stale

Previously recorded verification no longer applies because:

* The target changed.
* The staircase revision changed.
* The verification configuration changed.

---

## 9. Structural Operations

### 9.1 Split

Splitting inserts a new cut into an existing step.

Given cuts:

[
a_{i-1} \prec a_i
]

and a commit (b) satisfying:

[
a_{i-1} \prec b \prec a_i
]

a split transforms:

[
(..., a_{i-1}, a_i, ...)
]

into:

[
(..., a_{i-1}, b, a_i, ...)
]

Example:

```console
git staircase split auth:2 --at <commit>
```

A pure split changes decomposition but does not require rewriting commits.

---

### 9.2 Join

Joining removes a cut between adjacent steps.

A join transforms:

[
(..., a_{i-1}, a_i, a_{i+1}, ...)
]

into:

[
(..., a_{i-1}, a_{i+1}, ...)
]

Example:

```console
git staircase join auth:2 auth:3
```

A pure join preserves the body and aggregate outcome.

---

### 9.3 Reorder

Reordering changes the dependency order of steps.

Unlike split and join, reorder generally requires history rewriting because later steps must be replayed on different predecessors.

Example:

```console
git staircase reorder auth --steps 1,3,2
```

A reorder is valid only if the resulting patches can be applied and the resulting staircase satisfies configured structural and verification constraints.

---

### 9.4 Drop

Dropping removes one step’s changes from the staircase.

Example:

```console
git staircase drop auth:2 --restack
```

Dropping a nonfinal step generally requires replaying all dependent upper steps.

Drop changes:

* The body.
* The decomposition.
* The outcome, unless the dropped change was redundant.
* The exact revision.

It may preserve lineage identity.

---

### 9.5 Append

Appending adds one or more new commits above the current aggregate top and introduces either:

* A new step.
* Additional commits within the final step.

Example:

```console
git staircase append auth --new-step feature/auth-tests
```

---

### 9.6 Move commits between steps

A reshaping operation may transfer one or more commits or patch fragments from one step to another.

Example:

```console
git staircase move auth --from 3 --to 2 <commit>
```

This generally changes decomposition identity and may require rewriting dependent commits.

---

### 9.7 Rebase

Rebase moves the staircase body onto a new integration base.

Example:

```console
git staircase rebase auth --onto origin/main
```

A staircase-aware rebase must preserve or intentionally update:

* Step ordering.
* Step identities.
* Lineage metadata.
* Review associations.
* Ref mappings.

---

### 9.8 Restack

Restacking repairs dependent upper steps after a lower step has changed.

Example:

```console
git staircase restack auth
```

Restacking should replay each stale upper step onto the current version of its predecessor.

A restack commonly preserves:

* Staircase lineage identity.
* Nominal identity.
* Intended step ordering.
* Review identity.

It changes:

* Exact revision identity.
* Commit IDs.
* Possibly patch-series identity when conflicts require edits.

---

### 9.9 Delete

Deleting a managed staircase removes its managed refs and metadata.

Example:

```console
git staircase delete auth
```

Deletion should follow Git-native expectations:

* It deletes references, not commit objects directly.
* Commits remain recoverable while reachable from reflogs or other refs.
* Garbage collection remains responsible for eventual object removal.

The command should distinguish between:

* Deleting the managed entity.
* Deleting associated branch refs.
* Deleting external reviews.
* Abandoning reviews.
* Removing commits from another staircase.

These effects must not be conflated.

---

## 10. Verification Contract

### 10.1 Verification is not structural

Commit ancestry cannot establish whether a staircase:

* Builds.
* Passes tests.
* Satisfies style checks.
* Preserves APIs.
* Meets review policy.

Verification is an attached contract with recorded evidence.

---

### 10.2 Aggregate verification

Under aggregate verification, only the complete staircase outcome must pass.

Let (L(t, a_k)) represent landing staircase top (a_k) onto target (t).

Then:

[
V(L(t, a_k)) = \text{true}
]

Example:

```console
git staircase verify auth --aggregate
```

Aggregate verification permits intermediate prefixes that do not independently build or test successfully.

---

### 10.3 Prefix verification

Under prefix verification, every cumulative prefix must pass:

[
V(L(t, a_i)) = \text{true}
]

for all:

[
1 \leq i \leq k
]

Example:

```console
git staircase verify auth --each-prefix
```

Prefix verification is appropriate when each step is expected to be independently reviewable or landable.

---

### 10.4 Verification profiles

A verification profile may define:

* Build commands.
* Test commands.
* Static analysis.
* Formatting checks.
* Platform matrix.
* Environment variables.
* Required tool versions.
* Allowed failures.
* Timeouts.
* Whether verification applies to every prefix or only the aggregate.

Example:

```console
git staircase verify auth --profile presubmit
```

---

### 10.5 Verification evidence

A verification record should identify:

* Staircase lineage.
* Exact staircase revision.
* Resolved integration target.
* Verification profile.
* Command results.
* Timestamp.
* Execution environment.
* Optional external CI run identifiers.

Verification evidence becomes stale when any semantically relevant input changes.

---

## 11. Landing Semantics

### 11.1 Aggregate landing

The staircase lands as one combined change.

This may correspond to:

* Merging the aggregate top.
* Squashing the staircase.
* Applying one aggregate patch.
* Creating one external review.

---

### 11.2 Stepwise landing

Steps land in order.

Each step is landed only after all predecessors are integrated.

This may correspond to:

* A chain of dependent reviews.
* Multiple Gerrit changes.
* Multiple pull requests.
* Sequential cherry-picks.
* A review system with stacked-change support.

---

### 11.3 Partial landing

A lower prefix may land while upper steps remain active.

After partial landing:

* The integration boundary advances.
* Landed steps are no longer part of the unintegrated staircase body.
* Remaining steps may need rebasing or restacking.
* Lineage may either continue or fork into a successor staircase, depending on policy.

---

### 11.4 Landing policy

A managed staircase may declare:

* Aggregate-only landing.
* Stepwise landing.
* Either aggregate or stepwise.
* Required prefix verification.
* Required review approvals.
* Required target branch.
* Whether autosquash is allowed.
* Whether merge commits are allowed.

---

## 12. Merge Commits and Nonlinear History

### 12.1 Default linear-cut requirement

The staircase cuts themselves must form a chain:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

The commit region between cuts may contain merge commits.

---

### 12.2 Internal merges

An internal merge may combine side work into a later cut.

The body remains valid when all commits in the relevant region are dependencies of the aggregate top and are outside the integration boundary.

However, internal merges may make step attribution ambiguous.

The tool must define how commits reachable through multiple parent paths are assigned to steps.

The default rule should derive steps from set differences between cumulative prefixes rather than from first-parent traversal alone.

---

### 12.3 Multiple merge bases

The integration boundary should not be modeled solely as one merge-base commit.

Criss-cross merges or multiple upstream refs may produce more than one merge base.

Using an ancestor-closed integration set avoids requiring one unique merge base.

Implementations may use a simpler resolved base when repository history permits it, but the conceptual model remains set-based.

---

### 12.4 Forks

When candidate cuts are incomparable by ancestry, they cannot be successive cuts in one staircase.

They belong to separate staircases or to a staircase family.

---

## 13. Proposed Commands

The following command surface is illustrative rather than final.

### 13.1 Discover staircases

```console
git staircase discover
git staircase discover --onto origin/main
git staircase discover --refs 'refs/heads/feature/*'
git staircase discover --families
```

---

### 13.2 Adopt a staircase

```console
git staircase adopt auth \
    --onto origin/main \
    feature/auth-core \
    feature/auth-ui \
    feature/auth-tests
```

---

### 13.3 List staircases

```console
git staircase list
git staircase list --managed
git staircase list --discovered
git staircase list --families
git staircase list --stale
```

---

### 13.4 Show staircase details

```console
git staircase show auth
git staircase show auth --graph
git staircase show auth --commits
git staircase show auth --steps
git staircase show auth --ids
git staircase show auth --verification
```

Possible output:

```text
auth
  lineage: 3d7d16d1-...
  target: origin/main
  resolved target: 91f3b4c
  state: clean
  contract: prefix-valid

  step 1
    ref: feature/auth-core
    cut: c2ab981
    commits: 3

  step 2
    ref: feature/auth-ui
    cut: 8a019de
    commits: 2

  step 3
    ref: feature/auth-tests
    cut: 120db11
    commits: 1
```

---

### 13.5 Show identities

```console
git staircase id auth --kind=lineage
git staircase id auth --kind=revision
git staircase id auth --kind=body
git staircase id auth --kind=decomposition
git staircase id auth --kind=outcome
git staircase id auth --kind=patch-series
```

---

### 13.6 Reshape

```console
git staircase split auth:2 --at <commit>
git staircase join auth:2 auth:3
git staircase reorder auth --steps 1,3,2
git staircase move auth --from 3 --to 2 <commit>
git staircase drop auth:2 --restack
```

---

### 13.7 Rebase and restack

```console
git staircase rebase auth --onto origin/main
git staircase restack auth
git staircase restack auth --from 2
```

---

### 13.8 Verify

```console
git staircase verify auth --aggregate
git staircase verify auth --each-prefix
git staircase verify auth --profile presubmit
```

---

### 13.9 Delete

```console
git staircase delete auth
git staircase delete auth --delete-step-refs
git staircase delete auth --keep-step-refs
```

---

## 14. Git-Native Semantics

A `git staircase` tool should behave consistently with established Git conventions.

### 14.1 Object preservation

Operations should create new commits and move refs rather than mutate Git objects.

---

### 14.2 Recoverability

Destructive-looking operations should preserve ordinary Git recoverability through reflogs and object retention where practical.

---

### 14.3 Explicit ambiguity

The tool should stop or report alternatives when repository state admits several materially different interpretations.

---

### 14.4 Plumbing and porcelain separation

The implementation should distinguish:

* Low-level operations for reading or updating staircase metadata.
* High-level user workflows such as split, join, restack, and verify.

---

### 14.5 Scriptability

Commands should support stable machine-readable output.

For example:

```console
git staircase list --format=json
git staircase show auth --format=json
git staircase status auth --porcelain
```

---

### 14.6 Ref compatibility

Where possible, staircase state should be represented using normal Git refs and objects so that it benefits from:

* Atomic ref updates.
* Reflogs.
* Fetch and push.
* Existing object storage.
* Existing revision syntax.
* Existing garbage collection behavior.

---

## 15. Storage Considerations

The exact representation remains open, but a managed staircase likely requires both mutable refs and immutable metadata.

Possible components include:

### 15.1 Lineage ref

```text
refs/staircases/<lineage-id>
```

This may point to:

* The aggregate top commit.
* A metadata commit.
* A custom manifest object.
* Another ref namespace containing the current staircase revision.

---

### 15.2 Step refs

```text
refs/staircases/<lineage-id>/steps/1
refs/staircases/<lineage-id>/steps/2
refs/staircases/<lineage-id>/steps/3
```

These refs may point to current cuts.

User-facing branch refs may optionally mirror them.

---

### 15.3 Metadata

Metadata may include:

```text
version
lineage identity
display name
target specification
resolved target
ordered steps
current cut IDs
associated refs
review identities
verification policy
landing policy
parent staircase or predecessor revision
```

---

### 15.4 Transport

Different aspects of staircase state have different transport properties.

Exact commit-based identities are portable when all referenced objects are transferred.

Local branch names are not inherently portable.

Persistent staircase refs may be transported using explicit Git refspecs.

External review identities may require separate synchronization.

---

## 16. Examples

### 16.1 One-step staircase

```text
M---A---B---C
^           ^
main        feature/auth
```

Definition:

```text
target: main
cuts: C
steps: 1
body: {A, B, C}
```

Conceptual command:

```console
git staircase adopt auth --onto main feature/auth
```

---

### 16.2 Three-step staircase across branches

```text
M---A---B---C---D---E---F
^           ^       ^   ^
main        core    ui  tests
```

Definition:

```text
target: main

step 1:
  cut: C
  commits: A, B, C

step 2:
  cut: E
  commits: D, E

step 3:
  cut: F
  commits: F
```

---

### 16.3 Splitting a step

Before:

```text
M---A---B---C---D---E
^           ^       ^
main        step1   step2
```

Current steps:

```text
step 1: A, B, C
step 2: D, E
```

Command:

```console
git staircase split auth:2 --at D
```

After:

```text
step 1: A, B, C
step 2: D
step 3: E
```

The commit graph may remain unchanged. Only the cut structure changes.

---

### 16.4 Joining steps

Before:

```text
step 1: A, B, C
step 2: D
step 3: E
```

Command:

```console
git staircase join auth:2 auth:3
```

After:

```text
step 1: A, B, C
step 2: D, E
```

---

### 16.5 Stale staircase

Before rewriting the lower step:

```text
M---A---B---C---D---E
            ^       ^
           core     ui
```

After rewriting `core`:

```text
M---A---B---C'       core
         \
          C---D---E  ui
```

The branch tips no longer form one ancestry chain.

A managed staircase can report:

```text
auth
  step 1: current
  step 2: stale
  required action: restack
```

Command:

```console
git staircase restack auth
```

Possible result:

```text
M---A---B---C'---D'---E'
            ^         ^
           core       ui
```

---

### 16.6 Staircase family

```text
M---A---B---C---D---E
            \
             F---G
```

Refs:

```text
core -> C
ui   -> E
cli  -> G
```

Discovered paths:

```text
core -> ui
core -> cli
```

The tool should not claim that `ui` precedes `cli` or that `cli` precedes `ui`.

---

## 17. Invariants

A materialized staircase must satisfy the following structural invariants.

### 17.1 Ordered cuts

[
a_1 \prec a_2 \prec \dots \prec a_k
]

---

### 17.2 Nonempty steps

[
S_i \neq \varnothing
]

for all (i).

---

### 17.3 Dependency closure

If a commit belongs to the staircase body, every ancestor required to reach the integration boundary must belong either to:

* The staircase body.
* The integration boundary.

No staircase commit may depend on an unaccounted-for local commit outside both sets.

---

### 17.4 Unique aggregate top

A staircase has one final cut (a_k).

Forked structures require a staircase family or separate staircases.

---

### 17.5 Deterministic decomposition

Given:

* An integration boundary.
* An ordered sequence of cuts.

The body, prefixes, and steps must be determined uniquely.

---

### 17.6 Explicit target resolution

Commands that depend on the target should record or display the resolved target used for the operation.

---

## 18. Open Questions

The following areas require further design.

### 18.1 Integration-boundary representation

Should the primary implementation support:

* One resolved base commit.
* One symbolic target ref.
* Several target refs.
* A fully general ancestor-closed commit set.

The conceptual model supports the general case, but the initial implementation may choose a constrained representation.

---

### 18.2 Merge semantics

How should merge commits within a step be presented?

Should the tool:

* Preserve all merge structure.
* Prefer first-parent views for display.
* Offer flattening operations.
* Reject some nonlinear bodies from managed staircases.

---

### 18.3 Patch normalization

What exact normalization defines:

* Patch-series identity.
* Decomposition identity.
* Outcome equivalence.

Whitespace, file renames, binary files, mode changes, and conflict resolutions require explicit treatment.

---

### 18.4 Review mapping

Can one step correspond to:

* Exactly one external review.
* Multiple external reviews.
* No external review.
* A review that also includes commits outside the staircase.

The core model should not unnecessarily constrain review-system integration.

---

### 18.5 Shared prefixes

Should shared prefixes be represented as:

* Separate staircases with duplicated metadata.
* A staircase family.
* A persistent parent-child relation between staircases.
* A general review DAG abstraction above staircases.

---

### 18.6 Step naming

Should steps have:

* Integer positions only.
* Stable step identifiers.
* Human-readable names.
* Both stable IDs and mutable display names.

Numeric positions are convenient but unstable under insertion and reordering.

---

### 18.7 Nested staircases

Can a staircase step itself contain a subordinate staircase?

This may be useful for hierarchical review, but it may also complicate identity and landing semantics.

---

### 18.8 Cross-repository staircases

The initial model assumes one Git object graph.

Supporting coordinated changes across repositories would require a higher-level aggregate that refers to several repository-local staircases.

---

## 19. Summary Definition

A staircase is a finite filtration of a dependency-closed region of the Git commit graph, relative to an integration boundary, whose successive differences form ordered review steps.

Each staircase has:

* An integration boundary.
* A dependency-closed body.
* One or more ordered cuts.
* One or more nonempty steps.
* One aggregate top.
* Optional refs that materialize its cuts.
* Optional persistent lineage metadata.
* Optional review, landing, and verification contracts.

A staircase may be discovered from existing Git structure or managed as a persistent entity.

Its useful identity depends on the transformation under consideration. Therefore the system should expose several identities rather than pretending that one identifier can simultaneously represent lineage, exact history, review decomposition, and aggregate outcome.
