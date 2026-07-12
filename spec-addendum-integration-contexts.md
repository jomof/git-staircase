# Addendum D: Integration Contexts and Detached-HEAD Workspaces

## 1. Status and Scope

This addendum modifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**

It defines how `git staircase` determines the integration context of a staircase in repositories where:

* `HEAD` may normally be detached.
* The checked-out commit may be selected by an external workspace manager.
* There may be no local `main`, `master`, or equivalent integration branch.
* The exact workspace revision may be pinned by commit rather than by a moving branch.
* Local staircase branches may remain based on an earlier workspace revision after the workspace advances.
* Review destination may be separate from the commit currently checked out.
* Multiple Git repositories may participate in a larger workspace, while each repository retains its own commit graph.

This addendum does not introduce dependencies on any particular:

* Workspace manager.
* Monorepo manager.
* Code-review system.
* Presubmit system.
* Remote-ref naming convention.
* Default branch name.

The design is based entirely on generic Git objects, refs, configuration, worktree state, and optional provider interfaces.

---

## 2. Core Correction

The original specification requires an integration boundary but may be read as assuming that the boundary is derived from a branch such as:

```text
main
origin/main
master
origin/master
```

That assumption is invalid in many legitimate Git environments.

An integration boundary may instead be derived from:

* An exact commit OID.
* A detached worktree checkout.
* A tag.
* A remote-tracking ref.
* A worktree-specific anchor ref.
* A configured revision expression.
* A workspace-context provider.
* A union of explicitly supplied integration anchors.

A staircase therefore does not require a named integration branch.

It requires a resolved integration set.

---

## 3. Revised Terminology

### 3.1 Integration context

An **integration context** describes the history considered already integrated for the purpose of interpreting one staircase.

For the common single-anchor case:

[
\mathcal{I} = (Q, T, U, P)
]

where:

* (Q) is an optional symbolic locator or target policy.
* (T) is an exact resolved commit OID called the integration anchor.
* (U) is the ancestor-closed integration set:

[
U = \operatorname{Ancestors}(T)
]

* (P) is provenance describing how the context was obtained.

The integration context may therefore be valid even when (Q) is absent.

Example:

```text
symbolic locator: none
integration anchor: ba19eb864d...
integration set: all commits reachable from ba19eb864d...
provenance: detached worktree HEAD
```

---

### 3.2 Integration anchor

An **integration anchor** is an exact commit whose reachable history is treated as already integrated.

It need not be:

* A branch tip.
* An ancestor of every staircase cut.
* The merge base between the staircase and its upstream.
* A commit with any locally visible ref.
* The eventual review destination.

An integration anchor may have advanced beyond the point from which the staircase was originally created.

---

### 3.3 Symbolic target locator

A **symbolic target locator** is an optional moving name or policy that can be resolved to an integration anchor.

Examples include:

```text
refs/remotes/origin/main
refs/tags/release-2027.04
refs/workspace/current/project-x
```

A symbolic target locator records moving intent.

An integration anchor records the exact commit used for one operation or staircase revision.

These must not be conflated.

---

### 3.4 Landing base

For display and replay planning, the **landing base** of staircase cut (a_i) relative to integration anchor (T) is one or more merge bases between (T) and (a_i).

The landing base is informative.

It is not the integration boundary itself.

The integration set remains:

[
U = \operatorname{Ancestors}(T)
]

This distinction permits a staircase to remain meaningful when the integration anchor has advanced after the staircase was created.

---

### 3.5 Workspace anchor

A **workspace anchor** is an exact commit that the current worktree or an external workspace environment presents as its synchronized, pinned, or selected baseline.

A detached `HEAD` may be used as a workspace anchor under the rules in this addendum.

A workspace anchor is evidence for an integration context. It is not automatically persistent staircase identity.

---

### 3.6 Review destination

A **review destination** identifies where a staircase or its steps are intended to be submitted for review or integration.

It is separate from the integration context.

A review destination may include:

* A remote.
* A destination branch.
* A review namespace.
* A review-system project.
* An upload policy.

The following are independently meaningful:

```text
integration anchor: exact workspace commit
symbolic target: none
review destination: configured externally
```

and:

```text
integration anchor: old resolved target OID
symbolic target: refs/remotes/origin/release
review destination: another branch or review queue
```

Commands that only need the commit graph must not require a review destination.

---

### 3.7 Integration-context candidate

An **integration-context candidate** is a possible integration context produced from one evidence source.

Each candidate contains:

```text
resolved anchor OID
optional symbolic locator
source
scope
authority
compatibility result
diagnostic explanation
```

Candidates are resolved and ranked separately for each implicit staircase candidate.

---

## 4. Design Principles

### 4.1 Detached `HEAD` is a normal state

A detached `HEAD` must not be treated as an error or exceptional repository state.

The absence of a current local branch does not imply:

* An invalid checkout.
* A missing integration context.
* A lack of reviewable work.
* A lack of local branches elsewhere in the repository.
* That `HEAD` is itself staircase work.

---

### 4.2 Do not infer branch names by convention

The implementation must not guess integration context from names such as:

```text
main
master
trunk
develop
default
```

unless those refs are selected by stronger evidence such as:

* Explicit user input.
* Branch upstream configuration.
* Remote `HEAD`.
* Worktree configuration.
* A context provider.
* Existing managed staircase metadata.

---

### 4.3 Exact commit contexts are first-class

An exact commit OID is a complete and valid integration anchor.

The tool must not require that the OID be reachable through a branch ref.

---

### 4.4 Current checkout and integration context are distinct

When `HEAD` is attached to a staircase branch, it is usually a cut or tip, not the integration anchor.

When `HEAD` is detached, it may represent:

* A workspace baseline.
* A historical commit selected for inspection.
* A staircase cut.
* A local unreferenced commit.
* A transient commit during rebase, bisect, or conflict resolution.

The tool must apply the contextual rules in this addendum rather than assuming that all detached checkouts have the same meaning.

---

### 4.5 Integration context is selected per staircase

`git staircase list` must not require one repository-wide integration boundary before it can produce any output.

Different staircases in one repository may have:

* Different targets.
* Different pinned baselines.
* Different managed integration contexts.
* Different external review destinations.

Integration-context inference therefore occurs per staircase candidate.

---

### 4.6 Read-only listing is best-effort

`git staircase list` must list all staircases it can resolve.

It must not fail globally merely because one or more implicit candidates lack a resolvable integration context.

---

### 4.7 No network access by default

Integration-context inference must use local information by default.

The following commands must not fetch, contact a review server, or query an external workspace service unless explicitly configured or requested:

```console
git staircase list
git staircase show
git staircase status
git staircase discover
```

Staleness of local refs is reported as provenance, not silently repaired through network access.

---

### 4.8 Prefer evidence over heuristics

Explicit, persistent, or provider-supplied context takes precedence over checkout-based inference.

Heuristic discovery must never override a managed staircase’s stored integration context.

---

### 4.9 Never store `HEAD` as moving intent

When detached `HEAD` supplies an integration context, the authoritative value for the operation is its resolved full OID.

The literal expression:

```text
HEAD
```

must not be persisted as the staircase’s target locator.

`HEAD` is worktree-relative and may later identify a completely different commit or branch.

---

## 5. Revised Staircase Definition

Given integration context:

[
\mathcal{I} = (Q,T,U,P)
]

and ordered cuts:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

the cumulative prefix remains:

[
P_i = \operatorname{Ancestors}(a_i) \setminus U
]

The steps remain:

[
S_1=P_1
]

and:

[
S_i=P_i\setminus P_{i-1}
]

for (i>1).

The integration anchor (T) is not required to be an ancestor of (a_1).

This is intentional.

If the integration target advances after the staircase is created, (T) and the staircase may diverge after a common merge base while the staircase body remains well-defined.

---

## 6. Integrated Prefixes

### 6.1 Structurally integrated cuts

A cut (a_i) is structurally integrated when:

[
a_i \in U
]

equivalently, when (a_i) is reachable from the integration anchor.

Because the cuts form an ancestry chain, structurally integrated cuts form a prefix:

[
a_1,\dots,a_m
]

for some (m), possibly zero or (k).

---

### 6.2 Active staircase

If the first (m) cuts are structurally integrated, the active staircase begins at:

[
a_{m+1}
]

The active step sequence is:

[
S_{m+1},\dots,S_k
]

A fully integrated staircase has:

[
m=k
]

and no active steps.

---

### 6.3 Structural integration is exact

Structural integration means that the exact commit is reachable from the integration anchor.

It does not detect:

* Cherry-picked equivalent patches.
* Squash merges with different commit IDs.
* Manually recreated changes.
* Semantically equivalent code.

Such equivalence requires a separate patch, outcome, or review-system identity.

---

## 7. Integration Relationship States

An integration context may relate to a staircase in one of the following ways.

### 7.1 Based on current anchor

The integration anchor is an ancestor of the first active cut:

[
T \prec a_{m+1}
]

The active staircase is directly based on the current integration anchor.

---

### 7.2 Target advanced

The integration anchor and the first active cut share history, but the integration anchor is not an ancestor of that cut.

The staircase is based on an earlier common history and may require rebase, merge, or replay before landing.

Example:

```text
             T1---T2   current integration anchor
            /
B---C---D
        \
         A1---A2       staircase
```

The integration set is still derived from `T2`.

The landing base may be `D`.

---

### 7.3 Prefix integrated

One or more lower cuts are reachable from the integration anchor, while upper cuts remain active.

The active staircase is shortened relative to the original cut sequence.

---

### 7.4 Target incorporated by an upper step

A later cut may merge or otherwise include the integration anchor even when lower cuts were based on an earlier anchor.

This is permitted when the resulting cumulative prefix and step sets remain well-defined.

---

### 7.5 Fully integrated

The aggregate top is reachable from the integration anchor.

The staircase has no active unintegrated body relative to that context.

---

### 7.6 Unrelated

The integration anchor and staircase have no common ancestry in the available object graph.

The candidate is incompatible with that integration context.

---

### 7.7 Indeterminate

The relationship cannot be proven because required history or objects are unavailable.

Examples include:

* Shallow history.
* Missing promisor objects.
* Corrupt object storage.
* An unavailable alternate object database.
* A provider returning an unresolved revision.

Indeterminate must not be reported as unrelated.

---

## 8. Integration-Context Sources

Candidates are considered in the following authority order.

### 8.1 Explicit command input

Explicit input has highest authority.

Examples:

```console
git staircase list --onto <commit-ish>
git staircase show auth --onto <commit-ish>
git staircase discover --onto <commit-ish>
```

The input is resolved immediately to a full commit OID.

If the input is a ref, the command may retain both:

```text
symbolic locator
resolved anchor OID
```

An invalid explicit input is an error.

---

### 8.2 Managed staircase metadata

A managed staircase’s descriptor supplies its authoritative integration context.

The descriptor may contain:

```text
target-ref <full-refname>
target-oid <full-oid>
target-mode exact|tracking
target-provenance <value>
```

A current detached checkout must not silently replace this context.

---

### 8.3 Interrupted operation state

An active staircase operation may retain an exact context in its journal or recovery refs.

That context governs resume and abort behavior.

---

### 8.4 Worktree-specific configuration

A worktree may configure an integration context:

```text
staircase.integrationAnchor
staircase.integrationTarget
```

Recommended semantics:

```text
staircase.integrationAnchor
    A commit-ish resolved as an exact anchor.

staircase.integrationTarget
    A full refname or revision expression representing moving intent.
```

If both are present:

* The anchor defines the exact current boundary.
* The target records intended future resolution.
* A mismatch is reported rather than silently reconciled.

Worktree-scoped configuration takes precedence over repository-wide defaults.

---

### 8.5 Repository and user configuration

Repository, global, or system configuration may supply default targets or anchor-ref patterns.

Examples:

```text
staircase.defaultIntegrationTarget
staircase.integrationAnchorRef
```

A configured ref must be resolved locally.

Configuration does not imply permission to fetch.

---

### 8.6 Context provider

An optional context provider may supply workspace-specific integration information without changing core staircase semantics.

A provider may be configured conceptually as:

```text
staircase.contextProvider <provider-name>
```

The implementation invokes a separately installed command such as:

```text
git-staircase-context-<provider-name>
```

The provider returns a versioned structured result containing some or all of:

```json
{
  "version": 1,
  "integration_anchor": "<full-oid-or-revision>",
  "symbolic_target": "<full-refname-or-null>",
  "review_destination": null,
  "authority": "workspace",
  "description": "workspace-selected revision"
}
```

Provider output must be resolved and type-checked by `git staircase`.

A provider may not inject staircase membership or lineage merely by supplying an integration anchor.

---

### 8.7 Unanimous branch upstream configuration

For an implicit staircase materialized by local branches, the configured upstreams of its primary branches may provide integration context.

The upstream evidence is usable when:

* Every relevant configured upstream resolves.
* The upstreams agree on the same full refname or the same OID.
* Conflicting branch upstreams are absent.
* The selected target is compatible with the staircase.

If lower and upper steps intentionally use different upstreams, automatic inference fails rather than selecting one.

---

### 8.8 Detached worktree context

When `HEAD` is detached, its exact OID may supply a workspace integration anchor under Section 10.

This source is evaluated after explicit, managed, configured, provider, and upstream sources.

---

### 8.9 Remote default refs

A remote default ref may be considered only when:

* No stronger source resolves.
* Exactly one compatible remote default is available.
* It is not a staircase, operation, review, or recovery ref.
* The result is unambiguous.

Examples may include symbolic remote `HEAD` refs.

No preference is given to a remote named `origin`.

---

### 8.10 Aggressive graph inference

Scanning arbitrary remote-tracking refs and selecting a nearest compatible ancestor or merge base is not part of default inference.

An implementation may offer:

```console
git staircase list --infer-boundary=aggressive
```

Aggressive inference must:

* Report the selected evidence.
* Report rejected alternatives.
* Refuse incomparable maximal candidates.
* Never persist the result automatically as moving target intent.

---

## 9. Candidate Agreement and Conflict

### 9.1 OID coalescing

Several candidates resolving to the same anchor OID represent one exact integration context.

Their symbolic locators and provenance may be retained as aliases.

Example:

```text
HEAD
refs/remotes/workspace/current
refs/tags/build-2027.04.18
```

may all resolve to the same OID.

---

### 9.2 Same symbolic target, different observed OIDs

If two sources identify the same symbolic target but report different OIDs, the target is stale or was resolved at different times.

The command must select according to authority and report the discrepancy.

It must not merge their integration sets.

---

### 9.3 Conflicting authoritative contexts

Different authoritative candidates that resolve to different targets are an error for commands requiring one integration context.

Example:

```text
managed descriptor:  A
explicit provider:   B
```

unless the user explicitly selects one.

---

### 9.4 Conflicting heuristic contexts

When several equal-authority heuristic candidates produce different compatible anchors, inference is ambiguous.

The tool must not use refname order, lexical order, or shortest history distance as an invisible tie-breaker.

---

### 9.5 Multiple explicit anchors

A future or advanced command may explicitly define an integration set from several anchors:

```console
git staircase show auth \
    --integrated-revision release-a \
    --integrated-revision release-b
```

The resulting integration set is:

[
U =
\operatorname{Ancestors}(T_1)
\cup
\operatorname{Ancestors}(T_2)
]

Automatic heuristic inference must not construct such unions from unrelated candidates.

---

## 10. Detached-HEAD Inference

### 10.1 Eligibility

Detached `HEAD` is eligible as a workspace integration anchor when:

1. `HEAD` resolves to a commit.
2. `HEAD` is not symbolic to `refs/heads/*`.
3. The repository is not in a transient operation state that makes checkout meaning unreliable.
4. The candidate staircase is compatible with the resulting integration set.
5. `HEAD` is not clearly serving as one of the selected staircase cuts.

---

### 10.2 Detached `HEAD` is not a cut by default

A detached `HEAD` must not automatically become:

* A one-step staircase tip.
* A new staircase cut.
* A primary step ref.
* A managed staircase.

Implicit discovery normally uses durable refs or existing staircase metadata as cuts.

To use detached `HEAD` as a cut, the user must select it explicitly:

```console
git staircase show --top HEAD --onto <target>
```

or create an appropriate branch or managed cut.

This prevents arbitrary detached inspection commits from becoming surprise staircases.

---

### 10.3 Exact cut collision

If detached `HEAD` equals one of the candidate cut OIDs, it is ambiguous whether `HEAD` represents:

* The workspace baseline after partial integration.
* A detached checkout of a staircase step.
* A detached checkout of the staircase tip.

In this case, detached `HEAD` alone is insufficient.

It may still be used when corroborated by stronger evidence such as:

* A configured integration anchor.
* A provider.
* A non-local workspace or remote-tracking ref at the same OID.
* Managed target metadata.

Otherwise the integration context remains unresolved.

---

### 10.4 Descendant of aggregate top

If detached `HEAD` is a descendant of the aggregate top, the selected staircase may already be fully integrated relative to that checkout.

The tool must evaluate structural integration rather than assuming that `HEAD` is an upstream branch tip.

---

### 10.5 Diverged detached baseline

Detached `HEAD` may validly serve as the current integration anchor even when local staircase branches were created from an older revision.

Example:

```text
workspace HEAD: T2

B---T1---T2
     \
      A1---A2
```

The staircase body is computed relative to:

[
U = \operatorname{Ancestors}(T2)
]

The staircase may be reported as:

```text
target relation: advanced
landing base: T1
```

This is not an inference failure.

---

### 10.6 Corroborating refs

Refs resolving to the detached `HEAD` OID may increase confidence or supply display provenance.

Eligible corroborators may include:

```text
refs/remotes/*
refs/tags/*
configured workspace-anchor namespaces
```

The following must not be used as baseline corroborators by default:

```text
refs/heads/*
refs/staircases/*
refs/staircase-state/*
refs/staircase-operations/*
refs/bisect/*
temporary rewrite refs
review-upload destinations
```

A local branch at the same OID does not prove that the commit is an integration anchor.

---

### 10.7 Unreferenced detached commits

A detached `HEAD` with no corroborating ref may still serve as an exact current workspace anchor.

The output must make the limited provenance clear:

```text
integration anchor: ba19eb864d...
source: detached HEAD
symbolic target: none
```

The tool must not manufacture a branch name or remote target.

---

## 11. Transient Detached States

Detached `HEAD` must not be used as a fallback integration anchor during an active operation that temporarily changes checkout state.

Relevant operations include:

* Rebase.
* Cherry-pick.
* Revert.
* Merge conflict resolution.
* Bisect.
* Sequencer-driven replay.
* An interrupted staircase operation.

During such a state:

* Managed staircases remain listable using their descriptors.
* Explicit or configured integration contexts remain usable.
* Detached-HEAD fallback is disabled.
* `git staircase list` remains best-effort.
* Mutating commands must coordinate with or refuse the active operation.

A provider or configured exact anchor may still be used if it is independent of transient `HEAD`.

---

## 12. Branch-Attached Worktrees

When `HEAD` is attached to a local branch:

* `HEAD` is not used as an integration anchor merely because it is checked out.
* The branch may be a staircase cut or tip.
* Its configured upstream may provide integration context.
* Managed staircase metadata may provide integration context.
* Explicit or configured context remains authoritative.

If the attached branch is unrelated to the staircase being inspected, its upstream does not automatically govern that staircase.

---

## 13. Candidate Compatibility

An integration anchor (T) is compatible with a cut chain when:

1. The required commits and ancestry are available.
2. The cuts remain ordered:

[
a_1 \prec a_2 \prec \dots \prec a_k
]

3. The cumulative prefixes relative to:

[
U=\operatorname{Ancestors}(T)
]

are well-defined.
4. After removing structurally integrated lower cuts, each active step is nonempty.
5. The target and staircase are not unrelated.
6. The candidate does not cause a non-prefix set of cuts to be classified as fully integrated.
7. The resulting interpretation does not conflict with managed membership or explicit user selection.

Compatibility does not require that (T) be an ancestor of the first cut.

---

## 14. `git staircase list` Behavior

### 14.1 Managed staircases

Managed staircases must be listed using their stored descriptors even when:

* `HEAD` is detached.
* No integration branch exists.
* The current workspace anchor differs.
* The managed target ref is missing.
* The current worktree is at an unrelated commit.

Broken or unresolved managed state is shown as status, not converted into a global listing error.

---

### 14.2 Implicit discovery

For implicit staircases, `list` must:

1. Discover candidate cut chains independently of integration context where possible.
2. Resolve an integration context separately for each candidate.
3. List candidates with resolved contexts as implicit staircases.
4. Record unresolved candidates as diagnostics.
5. Continue processing other candidates.

---

### 14.3 No candidates

If no staircase candidates exist:

```console
$ git staircase list
```

should produce a successful empty result, for example:

```text
No staircases.
```

The command must not fail solely because no integration boundary was inferred.

---

### 14.4 Detached baseline with implicit staircases

Example:

```text
HEAD detached at ba19eb864d
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

Possible output:

```text
feature  3 steps  clean  (implicit)
  integration anchor: ba19eb864d
  source: detached workspace HEAD
```

---

### 14.5 Advanced workspace anchor

If detached `HEAD` has advanced beyond the staircase’s original base:

```text
feature  3 steps  target-advanced  (implicit)
  integration anchor: ba19eb864d
  landing base: 91c50d7a
```

The staircase remains listable.

---

### 14.6 Unresolved candidates

Default output may summarize unresolved candidates without failing:

```text
2 staircases
1 additional candidate could not resolve an integration context
```

Detailed output may be requested:

```console
git staircase list --diagnostics
```

Example:

```text
candidate: experimental-auth
cuts:
  refs/heads/experimental-auth-1
  refs/heads/experimental-auth

integration context:
  unresolved

considered:
  detached HEAD: incompatible
  branch upstreams: none
  configured target: none
```

---

### 14.7 Exit status

Recommended behavior:

* Exit `0` when listing completes, even if some implicit candidates are unresolved.
* Exit nonzero for repository corruption, invalid explicit arguments, or unreadable managed descriptors.
* Offer strict behavior:

```console
git staircase list --strict
```

Under strict mode, unresolved candidates or ambiguous integration contexts produce a nonzero exit status.

---

## 15. Commands That Require Integration Context

The following operations generally require a resolved integration context:

* Computing the complete staircase body.
* Verifying aggregate or prefix builds relative to upstream.
* Rebasing or replaying onto the current target.
* Determining structurally integrated prefixes.
* Producing upload ranges.
* Landing.
* Computing target-relative body or decomposition identities.

For a selected staircase, failure to resolve context is an operation-specific error.

Example:

```text
error: staircase 'auth' has no resolved integration context

considered:
  managed target: none
  branch upstream: none
  detached HEAD: equals staircase cut and is ambiguous

use:
  --onto <commit-ish>
or configure:
  staircase.integrationAnchor
```

This error applies to the selected operation, not to unrelated staircase listing.

---

## 16. Commands That Do Not Require Integration Context

The following may operate without an integration context when their requested result does not depend on one:

* Listing managed staircase names.
* Reading lineage IDs.
* Showing stored step IDs and cut OIDs.
* Showing branch-layout ownership.
* Renaming a managed staircase.
* Inspecting a stored descriptor revision.
* Deleting only managed metadata or refs.
* Displaying raw cut ancestry relationships.
* Validating sequential branch naming.

Such commands must not demand `--onto` merely because the repository lacks an attached integration branch.

---

## 17. Managed Staircases Created from Detached Context

### 17.1 Automatic adoption

When an implicit staircase is adopted while its integration context came only from detached `HEAD`, the initial managed descriptor must store:

```text
target-oid <full-detached-head-oid>
target-ref none
target-mode exact
target-provenance detached-workspace-head
```

It must not store:

```text
target-ref HEAD
```

---

### 17.2 Corroborating symbolic refs

If exactly one eligible stable ref corroborates the anchor, the tool may report it as a candidate symbolic target.

It must not silently convert that ref into persistent moving intent during automatic adoption.

The user may explicitly choose tracking behavior:

```console
git staircase target set auth \
    --track refs/remotes/workspace/current
```

---

### 17.3 Exact versus tracking targets

A managed staircase may use one of two target modes.

#### Exact mode

```text
target-mode exact
target-oid <oid>
target-ref none
```

The target does not move automatically.

#### Tracking mode

```text
target-mode tracking
target-ref <full-refname>
target-oid <last-resolved-oid>
```

The ref expresses moving intent.

Each operation still records the exact OID used.

---

### 17.4 Workspace target changes

If the current detached workspace anchor differs from a managed exact target, status may report:

```text
managed target:   91c50d7a
workspace anchor: ba19eb864d
relation: workspace context changed
```

The managed target must not change automatically.

An explicit operation may update it:

```console
git staircase target refresh auth --from-workspace
```

---

## 18. Worktree Scope

### 18.1 Current worktree only

Detached-HEAD inference uses the current worktree’s `HEAD`.

It must not inspect another linked worktree and silently use that checkout as the current workspace context.

---

### 18.2 Multiple worktrees

Different worktrees may have different:

* Detached anchors.
* Checked-out staircase branches.
* Worktree-specific target configuration.
* Active operations.

Managed staircase descriptors remain repository-wide unless explicitly designed otherwise.

Commands may offer:

```console
git staircase list --all-worktrees
```

but must identify each worktree’s context separately.

---

### 18.3 Bare repositories

A bare repository has no current worktree checkout.

Detached-HEAD inference is unavailable.

Managed staircases remain fully inspectable.

Implicit discovery requires:

* Explicit context.
* Configured context.
* Provider context.
* Branch upstream context.
* Another locally resolvable target source.

---

### 18.4 Unborn `HEAD`

In a repository without an initial commit:

* `HEAD` cannot supply an integration anchor.
* No commit-based staircase can be materialized.
* Managed metadata may still be inspected if objects exist through other refs.
* Commands requiring commits must fail with an unborn-repository diagnostic.

---

## 19. Shallow and Partial Repositories

### 19.1 Ancestry uncertainty

In a shallow or partially materialized repository, failure to find a merge base may mean either:

* The histories are unrelated.
* Required ancestors are unavailable.

The tool must inspect repository state and report indeterminate ancestry when missing history could affect the result.

---

### 19.2 No automatic fetch

Read-only discovery must not fetch missing history automatically.

It may recommend an explicit fetch or deepen operation.

---

### 19.3 Managed exact state

A managed descriptor remains readable when its referenced objects exist.

If a cut or target object is absent, the staircase state is incomplete or unavailable rather than nonexistent.

---

## 20. Replace Objects and Alternate Graph Views

Git replacement objects or equivalent graph-altering mechanisms may cause ancestry traversal to differ from raw object-parent relationships.

The implementation must use one consistent graph view for:

* Target compatibility.
* Step decomposition.
* Integrated-prefix detection.
* Verification planning.

Managed descriptors continue to store raw full OIDs.

If replacement objects are active, diagnostic output should disclose that the effective graph differs from the raw object graph.

A command may offer a raw-object mode equivalent in spirit to disabling replacement refs.

---

## 21. Review and Upload Semantics

### 21.1 Review destination is separately resolved

An upload command may require both:

```text
integration context
review destination
```

Resolution of one must not silently imply the other.

---

### 21.2 No target invention from detached `HEAD`

Detached `HEAD` may provide an exact integration anchor.

It must not by itself determine:

* Remote name.
* Destination branch.
* Review namespace.
* Submission strategy.
* Review topic.
* Presubmit configuration.

---

### 21.3 Review adapters

A review-system adapter may provide:

* Destination branch.
* Upload ref.
* Review IDs.
* Presubmit policy.
* Patch-set mapping.

These are attached review semantics.

They do not alter the generic integration-context model.

---

### 21.4 Upload range

An upload range may be computed from:

[
\operatorname{Ancestors}(a_k)\setminus U
]

even when the integration anchor has advanced and is not an ancestor of the staircase tip.

The review adapter decides whether the staircase must first be rebased or whether the review system accepts the divergent series.

---

## 22. Presubmit and Verification

Verification evidence must identify the exact integration anchor used:

```text
staircase revision OID
integration anchor OID
verification profile
environment identity
```

A symbolic target name alone is insufficient because it may move between verification runs.

In detached workspaces, the exact checked-out anchor provides a natural reproducibility key.

Review-system presubmit results may be attached separately through review identities.

---

## 23. Interaction with Sequential Branch Naming

### 23.1 No checked-out tip requirement

The sequential naming layout from Addendum C must not require that the bare tip branch be currently checked out.

A worktree may remain detached at the integration anchor while the staircase is materialized by:

```text
feature-1
feature-2
feature
```

---

### 23.2 Structural mutation while detached at the anchor

If the current worktree is detached at the integration anchor and the anchor is not rewritten:

* The worktree remains detached.
* The current `HEAD` OID remains unchanged.
* Primary staircase branches may be created, renamed, or updated through ref transactions.
* The tool must not automatically check out the staircase tip.

---

### 23.3 Detached `HEAD` at an affected staircase commit

If detached `HEAD` points to a commit that will be rewritten or retired:

* The operation must detect the relationship.
* A dirty worktree may require refusal.
* The tool must not silently attach `HEAD` to a branch.
* The command must define whether detached `HEAD`:

  * remains on the old commit,
  * follows the rewritten conceptual step,
  * moves to a specified destination,
  * or causes the operation to abort.

Recommended default:

* Preserve detached `HEAD` when its commit remains valid and retained.
* Refuse destructive movement unless the command explicitly owns and updates the current worktree.
* Offer an explicit option to follow the rewritten step.

---

## 24. Context Inspection Commands

The command suite should provide a way to inspect inference directly.

### 24.1 Show current context

```console
git staircase context
```

Example:

```text
worktree:
  HEAD: ba19eb864d
  state: detached
  transient operation: none

integration candidate:
  anchor: ba19eb864d
  source: detached HEAD
  symbolic target: none

corroborating refs:
  refs/remotes/workspace/current
```

---

### 24.2 Explain a staircase context

```console
git staircase context explain auth
```

Example:

```text
selected:
  anchor: ba19eb864d
  source: detached HEAD

compatibility:
  target relation: advanced
  landing base: 91c50d7a
  active steps: 3

rejected:
  refs/remotes/upstream/release
    reason: conflicting equal-authority candidate
```

---

### 24.3 Set worktree context

```console
git staircase context set \
    --worktree \
    --onto <commit-ish>
```

This stores a worktree-specific exact anchor.

A tracking target may be configured separately.

---

### 24.4 Clear worktree context

```console
git staircase context clear --worktree
```

Clearing configured context restores normal inference.

---

## 25. Machine-Readable Output

Machine-readable staircase output should include:

```json
{
  "integration_context": {
    "resolved": true,
    "anchor_oid": "<full-oid>",
    "integration_set_kind": "ancestors-of-anchor",
    "symbolic_target": null,
    "target_mode": "ephemeral",
    "source": "detached-head",
    "authority": "workspace",
    "corroborating_refs": [
      "refs/remotes/workspace/current"
    ],
    "relationship": "target-advanced",
    "landing_bases": [
      "<full-merge-base-oid>"
    ]
  }
}
```

For unresolved context:

```json
{
  "integration_context": {
    "resolved": false,
    "considered": [
      {
        "source": "detached-head",
        "result": "ambiguous-cut-collision"
      },
      {
        "source": "branch-upstreams",
        "result": "none"
      }
    ]
  }
}
```

---

## 26. Security and Provider Trust

### 26.1 No automatic provider probing

The tool must not search the filesystem or `PATH` for arbitrary workspace-manager commands and execute them speculatively.

A provider is invoked only when:

* Explicitly selected by command-line option, or
* Explicitly configured in a trusted configuration scope.

---

### 26.2 No shell interpretation

Provider names and arguments must be executed as structured process arguments rather than interpolated into a shell command.

---

### 26.3 Repository-local provider configuration

Executing commands named by repository-local configuration can introduce code-execution risk.

Implementations should require an explicit trust policy before executing providers configured by repository-controlled data.

Passive configuration containing only refs or revision expressions may be treated separately from executable provider configuration.

---

### 26.4 Provider output is untrusted input

Provider output must be:

* Schema validated.
* Size limited.
* Revision resolved through Git.
* Type checked as a commit where required.
* Rejected if it names unavailable or malformed objects.
* Treated as evidence, not direct permission to mutate refs.

---

## 27. Failure and Diagnostic Rules

### 27.1 No global boundary error for `list`

The following behavior is nonconforming:

```text
Error: Could not infer integration boundary and none was provided.
```

when issued globally by `git staircase list` before candidate-specific processing.

---

### 27.2 Selected-operation errors remain valid

A selected operation may fail when it genuinely requires integration context and none can be resolved.

The diagnostic must explain:

* Which staircase was selected.
* Which sources were considered.
* Why each source failed.
* Whether detached `HEAD` was eligible.
* Which explicit option or configuration can resolve the ambiguity.

---

### 27.3 Empty repository state

No staircase and no integration context is a valid state.

It should not be represented as an error.

---

### 27.4 Ambiguity is not absence

When several integration contexts are plausible, the tool must report ambiguity rather than saying that no boundary exists.

---

## 28. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 28.1 A branch is not required

A valid integration context may consist solely of an exact anchor OID.

---

### 28.2 Review destination is independent

No integration-context source automatically supplies review-upload semantics unless an explicit adapter or policy says so.

---

### 28.3 `list` is candidate-local

Failure to resolve one implicit staircase’s context does not prevent listing other staircases.

---

### 28.4 Detached `HEAD` is contextual evidence

Detached `HEAD` may supply an integration anchor, but it does not automatically become a staircase cut or persistent target ref.

---

### 28.5 Exact OIDs govern operations

Every operation records or uses the full exact integration anchor OID.

---

### 28.6 `HEAD` is never persisted as target intent

The literal worktree-relative expression `HEAD` is resolved before persistence.

---

### 28.7 Advanced targets remain usable

An integration anchor need not be an ancestor of the staircase cuts.

A common-history divergence is a relationship state, not an inference failure.

---

### 28.8 Managed context wins

Workspace inference does not silently replace a managed staircase’s stored integration context.

---

### 28.9 No default-branch folklore

The implementation does not prefer `main`, `master`, or any other branch spelling without evidence.

---

### 28.10 No hidden network operation

Read-only context inference does not fetch or contact external systems by default.

---

### 28.11 Missing history is indeterminate

Incomplete ancestry data is not classified as unrelated history.

---

### 28.12 Current worktree scope is explicit

Detached-HEAD inference uses only the current worktree unless another scope is explicitly requested.

---

## 29. Example: Detached Workspace at a Pinned Baseline

Repository state:

```text
HEAD detached at ba19eb864d
no local staircase branches
```

Command:

```console
git staircase list
```

Result:

```text
No staircases.
```

Verbose context:

```console
git staircase list --verbose
```

```text
workspace integration context:
  anchor: ba19eb864d
  source: detached HEAD
  symbolic target: none

No staircases.
```

---

## 30. Example: Staircase Branches Above a Detached Baseline

Repository state:

```text
HEAD detached at B

B---A1---A2---A3
    ^    ^    ^
    F-1  F-2  F
```

Command:

```console
git staircase list
```

Result:

```text
F  3 steps  based-on-target  (implicit)
  integration anchor: B
  source: detached HEAD
```

---

## 31. Example: Workspace Advanced Since Staircase Creation

Repository state:

```text
          T1---T2
         /
B---C---D
         \
          A1---A2---A3
```

```text
HEAD detached at T2
F-1 -> A1
F-2 -> A2
F   -> A3
```

Result:

```text
F  3 steps  target-advanced  (implicit)
  integration anchor: T2
  landing base: D
```

The tool does not require a branch named after the target.

---

## 32. Example: Detached Checkout of the Staircase Tip

Repository state:

```text
B---A1---A2
    ^     ^
    F-1   F
```

```text
HEAD detached at A2
```

Detached `HEAD` equals a staircase cut.

Without corroborating context, the tool must not use `A2` as the integration anchor.

Possible result:

```text
candidate: F
integration context: unresolved

reason:
  detached HEAD equals staircase tip and may represent the work itself
```

The user may resolve this with:

```console
git staircase list --onto B
```

or configured workspace context.

---

## 33. Example: Partial Integration

Before integration:

```text
B---A1---A2---A3
    ^    ^    ^
    F-1  F-2  F
```

Current integration anchor reaches `A1`:

```text
B---A1---T
         \
          ...
```

Relative to `T`:

* `A1` is integrated.
* `A2` and `A3` remain active.

Possible output:

```text
F  2 active steps  prefix-integrated  (implicit)
  integrated cuts: 1
  integration anchor: T
```

The active branch layout may require explicit normalization before branch renaming, as defined in Addendum C.

---

## 34. Informative Applicability

Some multi-repository workspace tools intentionally synchronize project worktrees to a selected manifest revision and support detaching projects back to that revision. This makes detached checkout state a normal workspace baseline rather than an unusual browsing mode.

Git itself distinguishes symbolic refs from non-ref revision expressions, so a detached exact OID is already a normal Git object-selection state even though it lacks a local branch name.

Code-review systems may use a destination namespace or target branch that is distinct from the commit currently checked out. For example, Gerrit review uploads identify a target through review-upload semantics rather than requiring that target to be the current local branch. This supports keeping review destination separate from the generic integration context.

These examples motivate the design but are not dependencies of it.

---

## 35. Summary

A staircase requires an integration set, not an integration branch.

In the common single-anchor case:

[
U=\operatorname{Ancestors}(T)
]

where (T) may be an exact detached workspace commit.

The current worktree may therefore remain detached at a synchronized baseline while staircase cuts are represented by other local branches.

`git staircase list` must:

* List managed staircases without consulting the current branch.
* Infer integration context separately for each implicit candidate.
* Use detached `HEAD` as an exact workspace anchor when appropriate.
* Continue when some candidates remain unresolved.
* Return an empty successful result when no staircases exist.
* Avoid inventing review destinations or default branch names.

The governing rule is:

> Integration history is defined by reachable commits. Branches, workspace checkouts, and review destinations are independent ways of locating or using that history.
