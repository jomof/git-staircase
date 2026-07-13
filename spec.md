# Git Staircase Specification

## 1. Status and scope

This document defines **Git Staircase**, a first-class Git abstraction for an ordered, review-oriented body of work. A staircase may span multiple commits and local branches, may be reshaped during review, and may be landed as one aggregate change or as an ordered series of dependent changes.

The command name is:

```console
git staircase
```

This specification defines the core model, command semantics, persistence, discovery, workspace integration, drafts, verification, review-provider contracts, landing, lifecycle, transport, output, concurrency, recovery, and security behavior.

Provider-specific behavior for systems such as `repo`, Gerrit, and GitHub is defined separately. The core MUST accommodate those providers through typed capabilities and MUST NOT embed assumptions that belong to one provider.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

Backward compatibility with command aliases or terminology not defined here is not required.

---

## 2. Motivation

Git provides commits, branches, tags, trees, refs, worktrees, and repositories, but it does not provide a first-class object for a stack of dependent review units.

A staircase addresses work with these properties:

* It contains one or more commits.
* It is divided into one or more ordered review steps.
* Each step may contain several commits.
* Later steps depend on earlier steps.
* Cuts may be materialized by several local branches, one branch, or no public branch.
* The whole body and selected prefixes may have different verification contracts.
* Lower steps may be amended while upper steps remain conceptually attached.
* Steps may be split, joined, reordered, dropped, or have changes moved between them.
* Review identity may need to survive rebases and amendments.
* Work may be partially landed while the remaining steps preserve lineage.
* Staged, unstaged, and untracked work must be handled without flattening their distinctions.
* Multi-repository workspaces and detached `HEAD` must be normal operating conditions.

The staircase abstraction sits above commits and branches while remaining Git-native in storage, addressing, recoverability, and transport.

---

## 3. Goals and non-goals

### 3.1 Goals

The implementation MUST:

1. Make a currently discoverable staircase usable without initialization or prior adoption.
2. Preserve durable identity only when durable intent is needed.
3. Work in an ordinary single Git repository without external providers.
4. Compose workspace, review, verification, transport, and landing providers independently.
5. Treat attached and detached worktrees as equally valid.
6. Preserve Git object immutability and ref-based publication.
7. Surface ambiguity instead of resolving it through hidden precedence.
8. Plan complete mutations before publication and protect them with leases.
9. Preserve staged and unstaged boundaries through supported rewrite workflows.
10. Provide deterministic recovery after interruption where physical atomicity is unavailable.
11. Produce stable, uncontaminated machine-readable output.
12. Keep provider observations, local structure, user metadata, and lifecycle state distinct.

### 3.2 Non-goals

The core does not:

* Replace ordinary Git staging commands.
* Make a branch equivalent to a staircase.
* Treat every step as one commit.
* Treat every step as one external review.
* Infer a conventional branch such as `main`, `master`, or `trunk` without evidence.
* Assume a remote named `origin` is authoritative.
* Contact a network service merely to list local staircases.
* Execute repository-supplied code during passive provider discovery.
* Promise that archive hides objects from deliberate low-level inspection.
* Coordinate commits from several repositories inside one repository-local staircase.
* Silently squash, reorder, force-push, stage files, adopt refs, or merge metadata.

A future workspace-level aggregate MAY coordinate several repository-local staircases, but it is not itself one staircase.

---

## 4. Canonical vocabulary

### 4.1 Representation state

An **implicit staircase** is reconstructed from current Git structure and has no persistent staircase record.

A **managed staircase** has a stable lineage ID and a persistent staircase record.

The only canonical transition from implicit to managed state is **adoption**:

```console
git staircase adopt <selector>
```

The persistent state is called `managed`. Terms such as `tracked staircase` and `adopted staircase` are not parallel states.

### 4.2 Lifecycle state

A managed staircase has one lifecycle state:

```text
active
archived
```

The canonical transitions are:

```console
git staircase archive <selector>
git staircase unarchive <selector>
```

Archive is reversible deactivation. Delete is a separate destructive ref-removal operation.

### 4.3 Provider

A **provider** is an installed implementation of one or more typed capabilities.

A **capability binding** associates a provider with one capability in a workspace, repository, worktree, staircase, or invocation.

A **provider profile** is shorthand that creates several bindings. A profile is not a provider.

The term `adapter` is not used as a separate architectural concept.

### 4.4 Integration terms

An **integration context** is the complete structure used to determine which history is already integrated. It contains, as applicable:

```text
integration anchor
integration set
symbolic integration target
resolution mode
provenance
```

An **integration anchor** is an exact commit OID.

An **integration set** is the ancestor-closed set treated as already integrated.

A **symbolic integration target** is an optional moving ref or provider locator expressing future target intent.

A **review destination** is the provider-specific repository and branch, or equivalent destination, to which review work is submitted.

A **landing destination** is the exact destination used by a landing operation.

A **rewrite destination** is the exact commit used as the new parent context for a rewrite.

Normative data models MUST NOT use an unqualified `target` field.

### 4.5 Revision terms

A **structure revision OID** identifies one exact staircase structure.

A **metadata revision OID** identifies one exact user-facing metadata blob.

A **lifecycle revision OID** identifies one exact lifecycle blob.

A **record revision OID** identifies one exact record tree binding structure, metadata, and lifecycle.

A command MUST label each revision type. It MUST NOT print an ambiguous field such as `revision: abc123`.

---

## 5. Formal model

### 5.1 Commit graph

Let a Git repository contain a directed acyclic commit graph:

\[
G = (V, E)
\]

where `V` is the set of commits and `E` contains parent edges.

For commits `x` and `y`:

\[
x \prec y
\]

means that `x` is an ancestor of `y`.

### 5.2 Integration context

For the ordinary single-anchor case with anchor `T`:

\[
U = \operatorname{Ancestors}(T)
\]

where `U` is the integration set and includes `T`.

An explicitly configured integration context MAY contain several anchors. The canonical multi-anchor set MUST:

1. Resolve every anchor to a full commit OID.
2. Remove duplicate OIDs.
3. Remove an anchor whose ancestor set is wholly contained in another selected anchor's ancestor set.
4. Sort the irredundant OIDs lexically by raw encoded OID.
5. Record the integration-set kind.

Automatic heuristic discovery MUST NOT invent a multi-anchor integration set unless an explicitly selected discovery policy permits it.

### 5.3 Cut

A **cut** is a commit marking the cumulative end of a step.

For `k` steps:

\[
a_1 \prec a_2 \prec \dots \prec a_k
\]

The final cut `a_k` is the aggregate top.

### 5.4 Prefix

The cumulative prefix at cut `a_i` is:

\[
P_i = \operatorname{Ancestors}(a_i) \setminus U
\]

where `Ancestors(a_i)` includes `a_i`.

### 5.5 Step

The first step is:

\[
S_1 = P_1
\]

For later steps:

\[
S_i = P_i \setminus P_{i-1}
\]

Every step MUST contain at least one commit:

\[
S_i \neq \varnothing
\]

A step may contain one or many commits. Step count and commit count are independent.

### 5.6 Body

The staircase body is:

\[
B = P_k = \bigcup_{i=1}^{k} S_i
\]

The body is the complete unintegrated dependency-closed commit region introduced by the staircase.

### 5.7 Staircase

A linear staircase is:

\[
\mathcal{S} = (U; a_1, a_2, \dots, a_k)
\]

subject to ordered cuts, nonempty steps, dependency closure, and one aggregate top.

The same top commit may define different staircases when the integration context or lower cuts differ.

### 5.8 One-step staircase

A one-step staircase has one cut:

\[
\mathcal{S} = (U; a_1)
\]

It exists only if:

\[
\operatorname{Ancestors}(a_1) \setminus U \neq \varnothing
\]

A branch already contained in the integration set is not an empty one-step staircase.

### 5.9 Staircase family

A **staircase family** is a connected dependency structure containing several valid linear paths, for example:

```text
           ui
          /
core ----<
          \
           cli
```

`core -> ui` and `core -> cli` are distinct staircases sharing a prefix. Incomparable cuts MUST NOT be silently linearized.

### 5.10 Materialization

A staircase is **materialized** when all current cuts exist and form the intended ancestry chain.

A managed staircase may remain meaningful while stale, diverged, incomplete, archived, or interrupted. An implicit staircase is necessarily materialized and structurally clean.

### 5.11 Merge commits

Cuts MUST form a linear ancestry chain, but the commit region between cuts MAY contain merges.

Step membership is determined by cumulative set differences, not first-parent traversal alone.

A rewrite involving merge commits MUST either:

* Preserve merges under an explicit policy,
* Flatten them under an explicit policy, or
* Refuse when the resulting dependency or review decomposition is ambiguous.

### 5.12 Dependency closure

If a commit belongs to the body, every ancestor required to reach the integration set MUST belong to either the body or integration set. No staircase commit may depend on an unaccounted local commit outside both.

### 5.13 Overlap, nesting, and shared prefixes

Distinct staircases MAY contain some of the same immutable commits or one body may be a prefix of another. Structural overlap alone does not merge their lineage, metadata, lifecycle, review associations, policies, or integration contexts.

The rules are:

1. Structurally equivalent implicit candidates collapse under Section 7. Non-equivalent decompositions or integration contexts remain distinct even when their bodies overlap.
2. Two managed lineages MAY intentionally reference the same cut OID, but one active local ref MUST NOT be owned by more than one lineage.
3. Rewriting one staircase creates new commits and moves only its owned refs. Another staircase retaining old OIDs remains unchanged and may later require an explicit rebase or restack; it is not silently rewritten.
4. A command whose planned ref, worktree, provider, or landing mutation would affect another managed lineage MUST report the overlap and require an explicit multi-lineage plan or refuse.
5. Shared-prefix intent that must propagate one conceptual step across several paths SHOULD be represented as a managed staircase family rather than as coincidentally overlapping lineages.
6. Nested staircase names have no structural meaning. Hierarchical refnames are organizational only.
7. Verification evidence and provider review identity remain scoped to their exact subject and association; overlap does not transfer either.

---

## 6. Workspace and provider architecture

### 6.1 Workspace

A **Staircase workspace** is the configuration scope containing one or more Git repositories participating in a common development environment.

A workspace MAY be:

* One ordinary Git worktree.
* One bare Git repository.
* A multi-repository workspace.
* A nested workspace.
* A provider-identified virtual workspace.

A workspace record contains:

```text
workspace ID
workspace kind
canonical root or provider-native locator
participating repositories
capability bindings
binding provenance
discovery fingerprint
last successful validation
```

Automatically created workspace records MUST be user-local and non-versioned. They MUST NOT be written into source repositories or provider-owned metadata by default.

### 6.2 Core fallback

The core always supplies a standalone fallback:

```text
workspace             = core.git
project-mapping       = core.git
integration-context   = core.git
transport             = git
```

Missing review or verification providers do not prevent local discovery, inspection, reshaping, rebase, restack, verification commands that have local profiles, storage, archive, or transport.

### 6.3 Capability classes

The initial capability classes are:

```text
workspace
project-mapping
integration-context
workspace-hints
repository-routing
review
review-identity
verification
review-transport
transport
landing
```

Capabilities remain separate even when one provider implements several of them.

### 6.4 Trusted provider discovery

Providers eligible for automatic bootstrap MUST be installed in trusted system or user locations, bundled with Staircase, explicitly registered, or explicitly named by the user.

The core MUST NOT execute arbitrary `PATH` programs merely because their names resemble providers.

A passive probe MUST:

* Be read-only.
* Avoid network access and authentication.
* Avoid Git ref or configuration mutation.
* Avoid workspace-manager mutation.
* Avoid repository hooks and repository-supplied executables.
* Avoid shell evaluation of repository data.
* Complete within bounded time and output size.
* Return schema-validated evidence.

### 6.5 Bootstrap protocol

Every command performs the following bootstrap protocol before command-specific behavior:

1. Resolve the current directory, worktree, Git common directory, object format, `HEAD` state, and active Git operation.
2. Locate and validate an existing workspace record.
3. If needed, invoke trusted passive workspace probes.
4. Add the built-in `core.git` fallback candidate.
5. Select an explicit or proven specialized workspace, otherwise use the fallback.
6. Pass typed workspace facts and hints to dependent capability probes.
7. Bind each unambiguous capability independently.
8. Persist eligible automatic bindings with locking, temporary-file publication, atomic replacement, and compare-and-swap.
9. Continue the original command.

The fallback candidate MUST NOT create ambiguity with a specialized provider that proves membership in a larger workspace.

When specialized workspace claims conflict, read-only repository-local commands MAY continue in temporary `core.git` mode. Workspace-wide commands MUST fail until selection is explicit.

When review-provider discovery is ambiguous, the workspace binding MAY still be persisted. Only review-dependent commands fail.

### 6.6 Bootstrap controls

The core SHOULD support:

```console
git staircase --no-bootstrap <command>
git staircase --no-configure <command>
git staircase --workspace <workspace-id> <command>
git staircase --workspace-provider <provider> <command>
git staircase --review-provider <provider> <command>
git staircase --provider-profile <profile> <command>
git staircase --workspace-mode=single-git <command>
```

`--no-bootstrap` disables probing and configuration.

`--no-configure` permits invocation-local probing but persists nothing.

`--workspace-mode=single-git` ignores enclosing multi-repository candidates for the invocation.

### 6.7 Provider protocol

A provider protocol MUST distinguish facts, hints, inferences, and requirements.

Core operations SHOULD include:

```text
describe
probe-workspace
probe-capability
resolve-project
resolve-integration-context
resolve-repository-route
validate-binding
```

Capability-specific operations MAY include:

```text
review-plan
review-create
review-upload
review-query
review-reconcile
verification-query
transport-plan
landing-plan
landing-execute
```

Provider data MUST be typed, bounded, schema-validated, and treated as untrusted input. Paths, OIDs, refs, and URLs MUST be independently normalized or validated by the core.

### 6.8 Binding precedence and revalidation

Binding provenance is one of:

```text
explicit
profile
auto-discovered
inherited
default
```

Explicit and profile bindings MUST NOT be silently replaced.

Automatic bindings MAY be replaced only when the replacement is unambiguous, passive, locally evidenced, and does not unexpectedly add network or mutation behavior.

Revalidation is required when relevant workspace metadata, provider versions, project membership, remotes, manifests, roots, or provider evidence changes.

### 6.9 Integration-context resolution

For a selected staircase, the integration context is resolved in this order:

1. Explicit command input.
2. Managed staircase structure.
3. Interrupted Staircase operation state.
4. Explicit worktree or repository configuration.
5. Bound integration-context provider.
6. Applicable branch upstream configuration.
7. Eligible detached `HEAD`.
8. Unique compatible remote-default evidence.
9. Unresolved.

A weaker source MUST NOT overwrite a stronger source.

Detached `HEAD` is eligible as an exact integration anchor when it resolves to a commit, is not controlled by a transient Git operation, is not clearly the staircase work tip, and is compatible with the graph.

The literal expression `HEAD` is never persisted as moving intent. Its full resolved OID is persisted.

### 6.10 Separation of integration and review

The following values may differ and MUST remain separately typed:

```text
workspace checkout anchor
integration anchor
symbolic integration target
review destination
review transport ref
landing destination
current HEAD
```

A provider may supply several of these, but it MUST NOT collapse them into one field.

---

## 7. Discovery and canonical implicit staircases

### 7.1 Discovery pipeline

Discovery is a normalization pipeline:

```text
raw discovery evidence
        ↓
normalized candidates
        ↓
structural equivalence classes
        ↓
canonical implicit staircases
        ↓
human names and machine selectors
```

Only canonical implicit staircases may be listed, selected, shown, adopted, archived, or mutated.

### 7.2 Raw evidence

Raw evidence MAY come from:

* Local branches.
* Remote-tracking refs.
* Sequential branch families.
* Managed or provider-supplied cuts.
* Branch upstreams.
* Workspace integration contexts.
* Explicit tips or cut lists.
* Review-provider associations.
* Cached observations.
* Detached `HEAD`.

Raw evidence has no user-facing identity and MUST NOT receive independent structural keys.

### 7.3 Candidate normalization

A normalized candidate contains at least:

```text
repository identity
object format
integration-context identity
ordered cuts
active cuts
step decomposition
materializing refs
aliases
discovery provenance
relationship to integration context
```

Normalization MUST:

1. Resolve revision expressions to full typed OIDs.
2. Resolve symbolic integration locators to exact anchors.
3. Remove lower cuts already integrated.
4. Reject empty steps.
5. Verify cut ancestry and dependency closure.
6. Canonically order unordered evidence.
7. Separate names and provenance from structure.
8. Preserve enough evidence to explain rejection or distinction.

### 7.4 Canonical structural identity

For an implicit linear staircase, canonical identity is:

\[
C = (R, O, I, A)
\]

where:

* `R` is repository identity.
* `O` is object format.
* `I` is canonical integration-context identity.
* `A` is the ordered sequence of active cut OIDs.

Names, aliases, refs, checkout state, review titles, and discovery provenance are not structural identity.

Repository identity MUST distinguish different object databases even if they contain equal OIDs.

### 7.5 Structural equivalence

Two candidates are equivalent if and only if repository identity, object format, integration context, and ordered active cuts are equal.

Equivalent candidates become one canonical staircase. Their refs, aliases, symbolic locators, provider hints, caches, and provenance are merged.

Equal aggregate tops are insufficient. Candidates with equal tops but different integration anchors or lower cuts remain distinct.

### 7.6 Structural key

Each canonical implicit staircase receives one deterministic key:

```text
implicit@<digest>
```

The digest MUST use an explicit algorithm, versioned canonical serialization, and domain separation. It MUST cover repository identity, object format, integration-context identity, and ordered active cuts.

Traversal order and redundant evidence MUST NOT affect the key.

Human output MAY abbreviate the digest only to a prefix unique in the current selection scope. Machine output MUST use the full key.

### 7.7 One-step discovery

A one-step candidate is valid only when its body is nonempty.

A branch selected solely as the symbolic integration target MUST NOT also appear as work unless it contains commits outside the selected integration set.

A branch named `main` MAY be work when it is genuinely ahead of its integration context. Branch spelling does not decide membership.

### 7.8 Names and aliases

An implicit staircase MAY have one deterministic display name and several aliases.

Canonical display-name precedence is:

1. A complete conforming sequential-layout tip name.
2. An explicitly selected primary branch name.
3. A unique local branch at the aggregate top.
4. The lexically first eligible local branch short name.
5. The structural key.

Alias matching selects the same canonical staircase when unambiguous.

Distinct staircases MAY share a display name. Neither may be hidden.

### 7.9 Families

Discovery MUST preserve incomparability. It MAY report a family and its individual linear paths. Selecting a family where a linear staircase is required MUST fail unless a path is supplied.

### 7.10 Caching

A discovery cache MAY contain normalized structure, aliases, provenance, relevant ref OIDs, and a configuration fingerprint.

A cache is only evidence. A stale cache entry MUST NOT coexist as a second staircase beside freshly discovered equivalent structure.

Caches MUST be invalidated when relevant refs, objects, integration configuration, workspace evidence, or discovery policy changes.

### 7.11 Listing contract

`git staircase list` lists canonical active managed and canonical implicit staircases. It MUST NOT list raw evidence.

Each listed entry represents exactly one selectable object.

When a bare name is displayed without qualification, that name MUST resolve uniquely under unchanged state.

When several distinct staircases share a name, the primary listing MUST display a structural-key or lineage qualification and the facts that distinguish them.

The following behavior is forbidden under unchanged state:

```console
$ git staircase list
feature  1 step  clean  (implicit)

$ git staircase archive feature
error: selector is ambiguous
```

### 7.12 Referential integrity and state changes

A selector displayed by one command MUST identify the same canonical staircase later while relevant repository and configuration state remains unchanged.

If state changes between listing and mutation, the later command MAY fail, but it MUST say that selection changed or is no longer unique. It MUST NOT present a preexisting canonicalization defect as user ambiguity.

### 7.13 Mutation snapshot

After selecting an implicit staircase, a mutating command MUST bind to an immutable discovery snapshot containing:

```text
structural key
repository identity
integration context
ordered cuts
materializing refs
expected ref OIDs
discovery-configuration fingerprint
```

Before publication, the command MUST verify that cuts, refs, context, and key still match and that no conflicting managed record appeared.

The command MUST NOT silently reselect another staircase with the same human name.

### 7.14 Discovery diagnostics

Ambiguity diagnostics MUST show for each candidate:

```text
structural key
integration context or anchor
ordered cuts or complete cut summary
top OID
materializing refs
```

They SHOULD also show names, aliases, and merged provenance.

If two structural keys have equal repository identity, context, and ordered cuts, the implementation has an internal canonicalization error. It MUST NOT report a user ambiguity.

`git staircase list --diagnostics` SHOULD report counts of raw evidence, normalized candidates, canonical staircases, and collapsed duplicates.

---

## 8. Implicit and managed behavior

### 8.1 Governing rule

An operation may leave a staircase implicit when its intended post-operation meaning is fully reconstructible from commits, ancestry, refs, integration context, discovery rules, and explicit arguments.

An operation MUST adopt before preserving information that would otherwise be lost or become ambiguous.

### 8.2 Capabilities of implicit staircases

An implicit staircase MAY be used for:

* Discovery and listing.
* Show, status, graph, log, diff, steps, and commits.
* Revision-derived identity queries.
* Verification of an exact current revision.
* Split or join when the final boundaries remain discoverable.
* Complete reorder, move, drop, rebase, or restack when the final result is immediately clean and discoverable.
* Aggregate or stepwise landing when no durable lineage, review mapping, policy, ledger, or recovery state is retained.
* Invocation-local draft materialization.
* Explicit deletion of the refs that currently materialize it.

An implicit staircase has no lineage ID or stable step IDs.

### 8.3 States requiring management

The following require a managed staircase:

* Stale, diverged, or incomplete intended structure.
* Stable lineage identity.
* Stable step identity.
* Persistent name independent of branch names.
* Metadata-only cuts or unnamed steps.
* Discovery overrides.
* Persistent policies.
* Persistent review associations.
* Lineage-relative verification history.
* Continuity across partial landing.
* Persistent draft attachment or snapshot association.
* Durable interrupted state after the active operation relinquishes control.
* Transport of lineage, policy, metadata, or review mappings.

### 8.4 Automatic adoption

When a command requires durable state, it SHOULD adopt automatically unless `--no-adopt` is supplied.

Human output MUST state that adoption occurred, why, and the new lineage or name.

Machine output MUST include:

```json
{
  "adopted": true,
  "adoption_reason": "<stable-code>",
  "lineage_id": "<uuid>"
}
```

`--adopt` forces adoption before an operation that could otherwise remain implicit.

`--no-adopt` fails before any mutation when adoption would be required.

### 8.5 Atomicity of adoption and triggering operation

Adoption and the triggering mutation form one logical operation.

If failure occurs before durable mutation, provisional adoption state SHOULD be removed.

If failure leaves a state that requires remembered intent, the managed record and recovery state MUST remain and MUST identify the interrupted, stale, or incomplete condition.

### 8.6 Explicit adoption

`git staircase adopt` records at least:

```text
lineage ID
ordered stable step IDs
integration context
current cuts
owned and materializing refs
current structure revision
```

Adoption does not inherently create commits, reviews, layout policy, archive state, or draft attachment.

### 8.7 Delete versus implicit refs

`git staircase delete <implicit-selector>` MUST fail because no managed record exists.

Deleting the refs that expose an implicit staircase requires an explicit option such as:

```console
git staircase delete <selector> --delete-materializing-refs
```

This operation MUST NOT adopt first.

---

## 9. Identities, names, and selectors

### 9.1 Identity layers

The core distinguishes:

```text
canonical staircase name
canonical staircase refname
lineage ID
step ID
step ordinal
cut OID
structure revision OID
metadata revision OID
lifecycle revision OID
record revision OID
implicit structural key
body identity
decomposition identity
outcome identity
patch-series identity
provider review identity
exact provider review revision
```

No one identifier substitutes for all others.

### 9.2 Managed name and ref

A managed staircase has zero or one canonical name.

A named active staircase uses:

```text
refs/staircases/<name>
```

The ref points to the current record tree, not the aggregate top commit.

The qualified shorthand is:

```text
staircases/<name>
```

### 9.3 Lineage ID

A lineage ID is an opaque lowercase UUID generated at adoption. It remains stable across rename, rebase, restack, amendment, reorder, partial landing with continuity, and structure or record revision changes.

It MUST NOT be derived from a name, OID, path, patch, provider review, or current content.

### 9.4 Step ID and ordinal

Each managed conceptual step has an opaque stable UUID.

A step ID survives rebase, restack, amendment, commit squashing within the step, branch rename, and reorder.

An ordinal is the current one-based position. `feature:2` selects the current second step and is not stable across insertion, split, join, drop, reorder, or partial landing.

### 9.5 Derived identities

The core MAY expose typed digests:

```text
body:<algorithm>:<digest>
decomposition:<algorithm>:<digest>
outcome:<algorithm>:<digest>
patch-series:<algorithm>:<digest>
```

These are not Git OIDs unless stored as Git objects under a separately specified representation.

Normalization rules for patch-derived identities MUST be versioned and explicit, including whitespace, renames, binary files, modes, and conflict-resolution content.

### 9.6 Accepted Git syntax

Where a command expects a commit, it SHOULD accept any unambiguous Git expression that resolves to a commit, including full or abbreviated OIDs, refs, tags that peel to commits, `HEAD`, reflog selectors, ancestry expressions, `git describe` output, and commit-message searches.

The expression MUST be resolved immediately and type-checked as a commit. Only the full resulting OID is authoritative.

A ref-valued input SHOULD preserve both its full refname and current OID.

Tree or path expressions, index-stage expressions, revision ranges, refspecs, and pathspecs MUST be rejected unless the option explicitly accepts that type.

Pathspecs MUST follow `--` when a command also accepts staircase selectors.

### 9.7 Typed selectors

Canonical typed selectors are:

```text
--name <exact-name>
--ref <full-staircase-ref>
--id <lineage-id>
--structural-key <implicit-key>
--record <record-revision-oid>
--structure <structure-revision-oid>
--step-id <step-id>
--top <commit-ish>
```

Typed selectors still undergo validation but do not participate in cross-type guessing.

`--name` does not resolve a collision between distinct staircases sharing that name.

### 9.8 Bare selector algorithm

For a bare selector, the command independently considers:

1. Managed staircase name.
2. Standard Git revision expression.
3. Canonical implicit display name or alias.
4. Recognizable full or abbreviated structural key.
5. Command-specific provider review selector, when explicitly allowed by that command.

Equivalent interpretations collapse. One distinct result succeeds. Several distinct results fail with complete diagnostics.

A managed staircase and an implicit candidate describing the same lineage and exact current structure SHOULD be represented only as the managed staircase.

### 9.9 Name validation

A canonical staircase name is valid only when:

```console
git check-ref-format "refs/staircases/<name>"
```

accepts it.

The core MUST NOT silently lowercase, trim, Unicode-normalize, slash-collapse, slugify, or strip numeric suffixes.

Names are exact, case-sensitive refname byte sequences. Implementations SHOULD warn about portability hazards involving case or Unicode normalization.

Hierarchical names are organizational only and do not imply staircase ancestry or containment.

### 9.10 Name uniqueness and rename

Creating a name MUST assert that the destination ref does not exist and is not reserved by an archived staircase.

Renaming changes the public refname only. It does not change lineage, steps, cuts, structure, metadata, lifecycle, record, or derived outcome identities.

Rename MUST atomically verify the old ref, create the new ref at the same record OID, and delete the old ref.

Unnaming removes the public ref while preserving the internal lineage record.

### 9.11 Snapshot tags

An immutable human name for an exact record revision MAY be an annotated tag under:

```text
refs/tags/staircase/<snapshot-name>
```

The tag points to the record tree or a versioned snapshot object that identifies it. Snapshot tags do not move unless explicitly force-replaced.

The canonical creation command is:

```console
git staircase tag <snapshot-name> <selector> [--message <text>] [--sign]
```

The selector is resolved to one exact record revision before the tag object is written. If the selector is a moving staircase name, publication MUST verify that it still points to the planned record revision. Tag creation MUST fail when the destination tag exists unless explicit force replacement is requested. Force replacement MUST display the old and new target record revisions and use an expected-old-value lease.

A snapshot tag names an immutable revision; it is not the mutable staircase name, does not reserve that name, does not establish lineage, and does not change structure, metadata, lifecycle, or record revisions.

### 9.12 Step selectors and step identity

A **step selector** is either:

```text
<staircase-selector>:<current-ordinal>
--step-id <stable-step-id>
```

The ordinal form is positional and may identify a different conceptual step after split, join, reorder, drop, insertion, or partial landing. The step-ID form is available only for managed staircases and identifies the conceptual step across supported rewrites and renumbering.

Canonical inspection and identity commands are:

```console
git staircase show <step-selector>
git staircase id <step-selector> --kind=step
git staircase diff <staircase-selector> --step <ordinal-or-step-id>
```

`show-step` and `step-id` are not separate canonical commands; their meanings are covered by `show` and `id` with typed step selection.

### 9.13 Canonicalization command

The core SHOULD provide:

```console
git staircase rev-parse <selector>
```

with typed projections such as:

```console
git staircase rev-parse --ref <selector>
git staircase rev-parse --lineage <selector>
git staircase rev-parse --record <selector>
git staircase rev-parse --structure <selector>
git staircase rev-parse --top <selector>
git staircase rev-parse --step <selector>
git staircase rev-parse --json <selector>
```

---

## 10. Persistent records and storage

### 10.1 Persistent layers

A managed staircase separates:

```text
structure
user-facing metadata
lifecycle
provider and operational observations
```

Provider observations MAY be referenced by structure or stored in a separate cache according to their semantics, but cached remote state MUST NOT silently redefine local structure.

### 10.2 Structure descriptor

The canonical structure descriptor is a versioned deterministic Git blob containing, as applicable:

```text
object format
lineage ID
integration context
ordered step IDs
cut OIDs
materializing refs
owned refs
structural state
branch-layout policy
verification and landing policies that affect interpretation
review mappings that are part of managed structure
parent structure revision
```

It MUST exclude the canonical staircase name because rename is a ref operation.

### 10.3 Metadata blob

The canonical metadata blob contains:

```text
title
description
labels
links
per-step metadata keyed by step ID
namespaced extension metadata
creation and modification provenance
```

Unknown namespaced fields MUST be preserved by core rewrites.

Metadata MUST NOT store derived facts such as current branch names, commit count, verification result, review approval state, mergeability, current worktree dirtiness, or provider availability as authoritative fields.

Recommended limits are:

```text
title                 4 KiB
description           1 MiB
individual label      1 KiB
individual link URI   16 KiB
complete metadata     4 MiB
```

NUL bytes are forbidden. Terminal control characters MUST be escaped safely.

### 10.4 Lifecycle blob

The lifecycle blob contains:

```text
active or archived state
ordered lifecycle events
archive reason
name reservation
retention policy
lifecycle provenance
archive restoration data or reference
```

### 10.5 Record tree

The record revision is a canonical Git tree:

```text
<record tree>
├── structure
├── metadata
├── lifecycle
└── archive-manifest      # archived only
```

The tree OID is the record revision OID.

### 10.6 Active refs

For a named active staircase:

```text
refs/staircases/<name>                     -> record tree
refs/staircase-state/<lineage-id>/record   -> same record tree
```

For an unnamed active staircase, only the internal record ref is required.

Current cuts that require retention MUST also be reachable through direct refs such as:

```text
refs/staircase-state/<lineage-id>/steps/<step-id>
```

Textual OIDs inside blobs do not create object reachability.

### 10.7 One logical read

A reader MUST:

1. Resolve one current record ref once.
2. Read the immutable record tree.
3. Read its immutable components.
4. Use the full OIDs in that snapshot.

It MUST NOT assemble one logical staircase read by racing across independently mutable step refs.

### 10.8 Canonical serialization

Every persistent serialization MUST be versioned, deterministic, locale-independent, unambiguous, ordered independently of map iteration, explicit about object format, and explicit about absent versus empty fields.

Timestamps appear only when semantically part of the relevant revision.

### 10.9 Revision effects

The identity effects are:

| Change | Lineage | Structure | Metadata | Lifecycle | Record |
|---|---:|---:|---:|---:|---:|
| Edit title or description | same | same | changes | same | changes |
| Add label or link | same | same | changes | same | changes |
| Rename canonical ref | same | same | same | same | same |
| Amend, rebase, or restack | same | changes | same | same | changes |
| Split or join | same | changes | may change | same | changes |
| Change structural policy | same | changes | same | same | changes |
| Archive or unarchive | same | same | same | changes | changes |
| Change archive reason | same | same | same | changes | changes |
| Refresh only cached provider observation | same | same | same | same | usually same |

### 10.10 Strict record-level compare-and-swap

Every persistent mutation MUST compare the complete current record revision.

For a named active staircase, both:

```text
refs/staircases/<name>
refs/staircase-state/<lineage-id>/record
```

MUST equal the expected record OID and MUST move together to the same new record OID.

For an unnamed active staircase, the internal record ref is the CAS unit.

For an archived staircase, the archive record ref is the CAS unit.

A metadata-only command MUST NOT compare only the metadata OID. A concurrent structure, lifecycle, policy, provider-association, or metadata change causes failure.

The stable machine error code is:

```text
concurrent-record-update
```

Core commands MUST NOT automatically merge concurrent metadata. An explicit merge workflow MAY exist, but it must expose the result, preserve unknown fields, and use a new current record as its CAS base.

### 10.11 Mutation publication order

A persistent mutation MUST:

1. Resolve the selected lineage and expected record exactly once for planning.
2. Read all immutable components.
3. Construct the complete logical post-operation state.
4. Write new commits, blobs, trees, and recovery objects.
5. Acquire required operation locks.
6. Begin a ref transaction.
7. Verify every affected ref against its expected old value or nonexistence.
8. Update reachability refs.
9. Update the internal record ref.
10. Update the public name ref, if any.
11. Commit the ref transaction.
12. Complete non-ref state under an operation journal.

A ref MUST NOT point to an object that has not been written successfully.

### 10.12 Locks, journals, and physical atomicity

Git ref transactions do not necessarily include worktree symbolic `HEAD`, index state, Git configuration, provider mutations, or filesystem state.

A multi-surface operation MUST use:

* A repository or lineage operation lock.
* A durable versioned operation journal.
* Complete prevalidation.
* Expected old values.
* Resume and abort behavior where feasible.
* Explicit reconciliation where remote outcome is uncertain.

The core MUST NOT claim physical atomicity across surfaces that Git cannot transact together. It MUST guarantee either consistent completion or deterministic recovery.

### 10.13 Reflogs

Public staircase refs, internal record refs, active step refs, archive refs, and recovery refs SHOULD have descriptive reflog entries where supported.

The implementation MUST use Git ref APIs or plumbing and MUST NOT manipulate presumed loose-ref or reflog files directly. Ref renames and permutations SHOULD record enough old and new identity to make recovery intelligible, but byte-for-byte reflog continuity across renamed refs is not required.

### 10.14 Record validation and legacy forms

Every object reached through a Staircase record ref MUST be type-checked, schema-checked, and integrity-checked before use. A blob, tree, tag, or commit that merely occupies the expected ref namespace is not automatically a valid record.

An implementation that encounters a supported legacy structure-only descriptor MAY interpret it as:

```text
structure: the existing descriptor blob
metadata: an empty canonical metadata blob
lifecycle: an active canonical lifecycle blob
```

The first persistent mutation MAY upgrade it to the current record-tree representation. The original structure blob OID remains the structure revision OID. Unsupported or malformed versions fail read-only inspection with diagnostics and MUST NOT be rewritten automatically.

---

## 11. Branches, ownership, and sequential primary layout

### 11.1 Branches materialize cuts

A branch is a movable ref to one commit. A staircase is a higher-level object containing an integration context, ordered cuts, steps, body, and optional persistent identity and policy.

A cut may have zero, one, or several refs. Refs are materialization and naming evidence, not intrinsic step identity.

### 11.2 Owned and unowned refs

A managed staircase may own refs. Ownership grants permission to update, rename, archive, or delete a ref only under the recorded policy and expected old OID.

Ownership is not inferred solely from:

* Name prefix.
* Numeric suffix.
* OID equality.
* Upstream configuration.
* Reflog similarity.
* Provider association.

An implicit staircase may infer temporary ownership only through a complete, unambiguous discovery convention.

Other refs at the same cuts are aliases or independent refs and MUST remain untouched unless explicitly adopted as owned.

### 11.3 Sequential primary-branch profile

The optional `sequential-v1` profile assigns one owned primary branch per active step using a branch-layout base `F`.

For `N` steps, the names are:

```text
F-1
F-2
...
F-(N-1)
F
```

The bottom step receives `F-1`. The tip receives bare `F`. A one-step staircase uses only `F`.

Formally, for zero-based position `i`:

\[
B(i,N,F) =
\begin{cases}
F & i = N-1 \\
F\text{-}(i+1) & 0 \le i < N-1
\end{cases}
\]

Sequential names communicate current position. Stable step IDs follow conceptual steps.

The branch-layout base is independent of the canonical staircase name.

### 11.4 Base selection

The base is selected from:

1. Explicit `--base`.
2. Stored layout base.
3. Exact unsuffixed tip name of a recognized implicit sequential layout.
4. Exact designated tip branch name for a newly created layout.
5. Otherwise unresolved.

The core MUST NOT silently strip a trailing numeric suffix. Explicit migration MAY propose suffix stripping after showing the result and checking all collisions:

```console
git staircase layout set <selector> --primary-branches=sequential --infer-base=strip-numeric-suffix
```

The proposed base and complete destination layout MUST be shown before publication.

Every generated full refname MUST pass Git refname validation as part of the complete set. Prefix conflicts are forbidden.

### 11.5 Recognition of an implicit sequential staircase

An implicit staircase conforms only when:

* It is linear and materialized.
* The tip has one selected local primary branch named `F`.
* Every lower cut has exactly the expected `F-1` through `F-(N-1)` branch.
* Every expected branch points to the corresponding cut.
* The base and primary mapping are unique.
* Additional refs do not create a competing primary mapping.

The inferred ownership exists only for the current operation.

### 11.6 Global renumbering

After any operation that changes step count or order, the implementation MUST compute the entire final branch mapping before mutation.

It MUST classify each old owned primary branch as unchanged, updated, renamed, deleted, or replaced, then validate every destination and worktree effect.

Renumbering MUST be expressed as one ref permutation where supported, not a sequence of high-level branch renames.

### 11.7 Shape effects

The default layout effects are:

* **Split:** the upper child retaining the original terminal cut retains the old step ID. A new lower child gets a new step ID. All positions are renumbered.
* **Join:** the later step ID survives by default. The boundary branch is deleted. Upper steps shift down.
* **Reorder:** step IDs move with conceptual work. The new tip receives bare `F`.
* **New tip step:** the former tip becomes `F-N`; the new tip receives `F`.
* **Drop:** the dropped step ID is retired and upper steps shift down after restack.
* **Partial landing:** remaining active steps are renumbered from the new integration context only after an explicit staircase operation confirms landing.
* **Rebase, restack, amend, or commit-message edit:** names do not change when step order and count remain the same.

### 11.8 Collisions

A destination occupied by another owned source ref in the same permutation is not a collision.

A destination occupied by an unowned ref is a collision even when it points to the desired OID.

A generic `--force` MUST NOT bypass ownership.

Every source and destination MUST be protected by expected old state. Concurrent movement aborts the operation and requires replanning.

### 11.9 Worktrees follow steps

Before renaming or deleting an owned branch, the implementation MUST inspect every linked worktree.

A worktree attached to a surviving conceptual step SHOULD follow that step to its new branch name.

For a retired step:

* Join defaults to the surviving joined step.
* Drop refuses unless a destination or detached behavior is explicit.
* Partial landing requires explicit handling when no corresponding active step remains.

A worktree MUST NOT silently switch conceptual steps merely because another step inherits the old branch spelling.

### 11.10 Branch configuration follows steps

Affected `branch.<name>.*` configuration MUST be associated with the stable step ID and permuted to the step's destination name.

Unowned destination configuration is a collision and MUST NOT be merged automatically.

Ref and configuration updates require a journal because they may not share one physical transaction.

### 11.11 Layout state and commands

Layout state is separate from structural state:

```text
clean
    every active step has the expected primary branch at the expected cut

dirty
    structure is valid but primary names or mappings do not match policy

blocked
    normalization is prevented by collisions, worktrees, invalid names,
    concurrent changes, configuration conflicts, or unsupported updates
```

Commands include:

```console
git staircase layout show <selector>
git staircase layout check <selector>
git staircase layout set <selector> --primary-branches=sequential --base <F>
git staircase layout normalize <selector>
git staircase layout rename <selector> --base <F>
git staircase layout branch <step-selector> --name <local-branch>
git staircase layout unset <selector> --primary-branches
```

`layout rename` changes the base for the complete sequential layout and computes every resulting primary branch name before mutation.

`layout branch` assigns or renames the one primary local branch for a selected step without changing staircase structure, stable step identity, or other aliases. It requires management unless ownership and the resulting implicit layout remain unambiguous for the complete operation. It MUST reject an occupied unowned destination and MUST move applicable worktree and branch configuration with the conceptual step.

`normalize` repairs derived layout without changing intended staircase structure.

---

## 12. Worktree drafts and materialization

### 12.1 Three planes

The model separates:

```text
committed staircase
index
worktree
```

Only commits belong to staircase history. The index and worktree are worktree-scoped draft overlays.

### 12.2 Worktree and basis

A worktree includes its `HEAD`, index, filesystem, worktree-specific Git state, and active Git operation.

A draft belongs to one worktree and one exact **draft basis**, normally the full current `HEAD` commit OID.

The literal name `HEAD` is not persistent identity.

### 12.3 Index and worktree state

When the index has only resolved stage-zero entries, it may define an exact candidate tree OID.

The staged delta is:

```text
draft-basis tree -> index
```

The unstaged tracked delta is:

```text
index -> tracked worktree
```

Untracked and ignored sets are distinct categories.

A candidate tree OID is not a commit OID, step identity, parentage, message, author, or representation of unstaged content.

### 12.4 Draft tuple

A live draft contains:

```text
basis OID
index state
tracked worktree state
untracked set
ignored set
optional attachment and intent
```

It is mutable and ephemeral.

### 12.5 Draft intent

Intent is one of:

```text
unassigned
extend-step
new-step
rewrite-step
```

`rewrite-step` is invalid without a concrete operation such as amend, fixup, squash, or fold.

### 12.6 Automatic attachment

A draft may attach automatically only when its exact basis OID equals one unique selected step cut.

Branch-name similarity, ancestry, patch similarity, or proximity are insufficient.

A detached worktree at a cut may attach by exact OID. If the same OID is both a plausible integration anchor and cut, stronger context or explicit selection is required.

Several matching steps leave the draft unassigned.

### 12.7 Persistent attachment

Invocation-local intent does not inherently require management.

Persistent attachment requires a managed staircase, stable step identity where applicable, a worktree identity, and an expected basis OID. Attaching persistently to an implicit staircase automatically adopts it.

If `HEAD` changes independently, the attachment becomes stale. The core MUST NOT silently reinterpret the draft against the new basis.

### 12.8 Draft classification

A draft may be:

```text
clean
staged-only
unstaged-only
partially-staged
untracked
conflicted
transient-operation
submodule-dirty
```

These flags may coexist where meaningful, such as partially staged plus untracked.

An index containing stages 1, 2, or 3 is conflicted and does not define one candidate tree.

### 12.9 Draft inspection

Commands include:

```console
git staircase draft status
git staircase draft show
git staircase draft diff --staged
git staircase draft diff --unstaged
git staircase draft diff --combined
git staircase draft diff --untracked
git staircase draft diff --ignored
git staircase draft attach <selector> --mode=<intent>
git staircase draft detach
git staircase draft snapshot
git staircase draft restore <snapshot-id>
git staircase draft materialize <selector> [--amend|--fixup|--new-step|--fold-into <step>] [-m <message>]
```

When persistent or invocation-local attachment already supplies intent, `draft materialize` may omit the corresponding intent option. Conflicting intent supplied by the attachment and command line is an error.

Combined diff is informational and does not preserve staging boundaries.

### 12.10 Materialization source

By default, materialization commits the exact current index and leaves unstaged and untracked content uncommitted.

It MUST NOT reconstruct staged content from whole worktree files.

The command must resolve parent, tree, message, author, committer, signing policy, and required commit trailers before creating the commit.

No staged changes is an error unless an empty commit is explicitly requested.

A conflicted index cannot be materialized.

### 12.11 Extend an existing step

For `extend-step`:

1. The selected cut MUST equal the draft basis.
2. The exact index becomes a new commit above that cut.
3. The step cut advances.
4. Upper steps MUST be restacked before clean completion, or the staircase MUST be adopted before leaving them stale.
5. Unstaged work remains unstaged as closely as ordinary Git commit semantics permit.

### 12.12 Create a new step

For `new-step`:

1. The predecessor cut MUST equal the basis.
2. The index becomes the first commit of the new step.
3. A managed staircase creates a new step ID.
4. A new cut is introduced.
5. Layout is recomputed when active.
6. The new step is the tip unless another topology is explicit.

### 12.13 Rewrite a step

A rewrite materialization MUST define:

* Replaced commits.
* Surviving step ID.
* Commit-message formation.
* Replay of upper steps.
* Review-identity preservation or retirement.
* Preservation of unstaged state.

Examples:

```console
git staircase draft materialize <selector> --amend
git staircase draft materialize <selector> --fixup
git staircase draft materialize --fold-into <step-selector>
```

### 12.14 Explicit broader inclusion

The core MUST NOT silently stage unstaged or untracked content.

`--all-tracked` MUST use a temporary alternate index and preserve the user's real index until publication succeeds.

Untracked files require `--include-untracked` or explicit path selection.

Ignored files require `--include-ignored`; a generic `--all` MUST NOT include them.

### 12.15 Draft verification

Verification subjects are typed:

```text
draft-index
draft-tracked-worktree
draft-snapshot
```

Index verification records basis OID, index tree OID, profile, environment, and result.

Working-draft verification also records inclusion policy, selected untracked paths, ignored exclusions, and relevant filter or attribute context.

Draft evidence does not automatically verify a future commit. Promotion is allowed only after proving exact tree equivalence and satisfying policy requirements.

### 12.16 Draft snapshots

A durable snapshot MUST preserve separately:

```text
basis
index candidate
unstaged tracked overlay
selected untracked content
selected ignored content
submodule policy
attachment and intent
```

One flattened patch is not lossless.

Restore MUST stop on overwrite collisions and retain all recovery objects.

An implementation MAY use stash machinery only when it preserves the required distinctions.

### 12.17 Dirty worktree rewrites

Read-only operations may run with any draft state.

A rewrite affecting the current basis, branch, index, or files MUST refuse by default when staged or unstaged changes would be invalidated.

`--preserve-draft` MUST:

1. Capture staged state exactly.
2. Capture unstaged tracked state separately.
3. Capture only explicitly selected untracked content.
4. Refuse unsupported dirty nested repositories unless recursive preservation is explicit.
5. Journal the operation.
6. Perform the rewrite.
7. Reapply to the corresponding surviving conceptual step or basis.
8. Restore staging boundaries.
9. Retain recovery state if restoration is incomplete.

A command MUST NOT claim success when staging boundaries were lost.

### 12.18 Multiple worktrees

Every worktree has independent `HEAD`, index, draft, attachment, and active operation state.

Two worktrees may hold competing drafts against the same cut. Materializing one may stale the other.

Default status shows the current worktree. `--all-worktrees` shows all known drafts.

### 12.19 Index corner cases

The implementation MUST respect:

* Partial staging.
* Intent-to-add.
* Unmerged stages.
* Sparse checkout and `skip-worktree`.
* `assume-unchanged` as an optimization, not intent.
* Git file modes and symbolic links.
* Clean/smudge filters, line-ending conversion, working-tree encoding, and attributes.
* Staged submodule gitlinks versus dirty nested worktrees.

It MUST use Git index and conversion semantics rather than raw filesystem comparisons.

### 12.20 Active Git operations

During merge, rebase, cherry-pick, revert, bisect, sequencer replay, or an interrupted Staircase operation, the index and worktree belong to that operation.

Staircase read-only commands may inspect without disturbance.

A new mutation MUST refuse, resume the owning Staircase operation, or explicitly integrate with the active Git operation.

Conflict-resolution state from a Staircase rewrite is not an ordinary user draft and MUST be restored separately from any preserved draft.

### 12.21 Draft identity and change detection

A live draft MAY have a worktree-local generation token for concurrent-change detection. It is not persistent public identity.

A resolved index tree OID is an exact Git identity for staged tree content and MUST be labeled as a tree OID, never a commit OID.

A cache MAY use a typed worktree fingerprint such as:

```text
worktree-draft:<algorithm>:<digest>
```

Its serialization and inclusion policy MUST be versioned. It is not portable identity, a Git OID, lineage, or stable evidence across attribute, filter, sparse-checkout, encoding, or repository changes.

A durable draft snapshot has its own UUID or immutable snapshot-object identity. That identity remains distinct from any later commit materialized from the snapshot.

### 12.22 Draft-consuming publication and recovery

Immediately before publishing a draft-consuming operation, the implementation MUST verify:

* `HEAD` still equals the planned basis.
* The index still equals the planned index state.
* Relevant worktree content and selected untracked content have not changed.
* The selected record or structural key remains current.
* Every affected owned ref retains its expected old value.

A mismatch causes replanning or abort before publication.

If a process creates commits or trees but fails before publishing Staircase refs, those objects are recovery objects, not successful materialization. Operation refs or a journal MUST identify them, and the original draft MUST remain recoverable until publication is confirmed.

No operation may silently discard draft content, attachment, staging boundaries, or a recovery snapshot. If exact restoration is incomplete, the command remains interrupted or completes with an explicit recovery-required state and identifies affected paths or hunks.

---

## 13. State model

### 13.1 Structural state

A managed staircase has one primary structural state:

```text
clean
stale
diverged
incomplete
partially-landed
interrupted
```

`clean` means intended cuts form a valid chain and authoritative refs agree.

`stale` means lower work changed while upper steps still depend on an older predecessor.

`diverged` means several incompatible candidates claim one intended step or authoritative refs disagree.

`incomplete` means an intended step has no resolvable current cut.

`partially-landed` means a lower prefix is integrated while upper steps remain active.

`interrupted` means a journaled operation has not completed or been aborted.

Ambiguity in discovery is not a managed structural state. It is a selection condition among candidates.

### 13.2 Implicit state

An actionable implicit staircase is materialized and structurally clean. It may be verified or unverified, but it cannot be called stale, diverged, incomplete, or a continuing partially landed lineage without management.

### 13.3 Verification state

Verification is orthogonal to structure. Recommended aggregate states are:

```text
passed
pending
failed
blocked
stale
incomplete
unknown
queued
```

`stale` means evidence no longer applies to the exact subject, integration anchor, profile, policy, or environment.

### 13.4 Review synchronization state

For each provider mapping, recommended local/remote synchronization states are:

```text
not-created
not-uploaded
current
local-newer
remote-newer
diverged
retargeted
closed
merged
abandoned
identity-ambiguous
upload-unknown
```

Provider-specific states remain separate fields and MUST NOT be flattened into a generic success flag prematurely.

### 13.5 Lifecycle and layout

Lifecycle (`active` or `archived`) and layout (`clean`, `dirty`, or `blocked`) are independent of structural and verification state.

A staircase may be structurally clean, layout-dirty, verification-stale, and active at the same time.

---

## 14. General mutation protocol

### 14.1 Plan before mutation

Every mutating command MUST:

1. Bootstrap and resolve typed context.
2. Resolve one canonical selector.
3. Read one immutable structure or record snapshot.
4. Inspect worktrees, drafts, active Git operations, refs, and configuration relevant to the operation.
5. Determine whether adoption is required.
6. Build the complete logical post-operation structure.
7. Build the complete ref, worktree, configuration, provider, and recovery plan.
8. Validate collisions, permissions, leases, and policy.
9. Expose the plan in `--dry-run` mode.
10. Write immutable objects.
11. Publish local refs and record state against expected old values.
12. Perform nonlocal operations from exact immutable OIDs.
13. Reconcile actual results.
14. Complete or retain the journal.

### 14.2 Dry-run

A dry-run MUST perform all safe resolution, discovery, validation, and planning available without mutation.

It MUST NOT create durable lineage, refs, commits, configuration, provider reviews, pushes, or lifecycle events.

It SHOULD show adoption, branch, worktree, review, verification, and landing consequences.

### 14.3 Conflict pauses

A Staircase-owned rewrite that encounters a content conflict MUST:

* Stop before publishing final structure.
* Retain original and partially rewritten cuts under operation refs.
* Record the exact step and commit being replayed.
* Mark the worktree as operation-resolution state.
* Provide exact continuation and abort commands.

Canonical commands are:

```console
git staircase continue
git staircase abort
git staircase operation show
```

`continue` validates that conflicts are resolved and relevant changes are staged, then resumes the owning operation.

`abort` restores the pre-operation refs, worktree basis, draft state, and configuration as far as recorded, and reports any residual state.

A new unrelated mutation MUST refuse while an operation is active.

### 14.4 Empty replays

A replay may become empty because its change is already present in the new predecessor.

The command MUST stop unless policy or an explicit option selects one of:

```text
drop-empty-step
retain-empty-commit
fold-identity-into-another-step
abort
```

Dropping retires the step ID. Retaining an empty commit keeps a nonempty commit set but records that its tree delta is empty. Provider policies may forbid empty review commits.

### 14.5 External Git operation

If the index is controlled by an external Git operation, Staircase MUST NOT take ownership automatically.

`git staircase status` MUST identify the operation and its owner when known, then recommend the owning Git or workspace-manager continuation command.

After the external operation completes, `workspace refresh`, `status`, `rebase`, `restack`, or `reconcile` may be used as appropriate.

---

## 15. Inspection and selection commands

### 15.1 Discover

```console
git staircase discover [--onto <commit-ish>] [--top <commit-ish>]
git staircase discover --steps <cut-1>,<cut-2>,...
git staircase discover --families
git staircase discover --range <revision-range>
```

`discover` reports canonical candidates and performs no adoption.

### 15.2 List

```console
git staircase list
git staircase list --implicit
git staircase list --managed
git staircase list --archived
git staircase list --all
git staircase list --families
git staircase list --strict
git staircase list --diagnostics
```

Default listing shows active managed and implicit staircases. Archived staircases require `--archived` or `--all`.

Listing is best-effort per candidate and MUST NOT require one repository-wide integration anchor.

### 15.3 Show and status

```console
git staircase show <selector>
git staircase show <selector> --graph
git staircase show <selector> --steps
git staircase show <selector> --commits
git staircase show <selector> --ids
git staircase show <selector> --verification
git staircase status [<selector>] [--all-worktrees]
```

`show` describes the selected committed staircase. A draft appears only in a separately labeled section or hypothetical view.

`status` reports local structure, layout, lifecycle, worktree drafts, operation state, and cached provider synchronization. It is distinct from `review status`.

### 15.4 Diff and history

```console
git staircase diff <selector> [--step <ordinal-or-id>] [--] [<pathspec>...]
git staircase log <selector>
git staircase graph <selector>
git staircase commits <selector>
git staircase steps <selector>
```

Per-step diffs are predecessor cut to current cut. Aggregate diff is integration anchor or set to top under the selected diff policy.

---

## 16. Structural command semantics

### 16.1 Adopt

The core has no empty-staircase `create` operation because every staircase step MUST be nonempty. A new staircase begins by adopting existing discoverable history, appending or splitting committed history into steps, or materializing a staged draft into a first nonempty step.

```console
git staircase adopt <selector> [--name <name>] [--onto <commit-ish>]
git staircase adopt --steps <cut-1>,<cut-2>,... [--name <name>] [--onto <commit-ish>]
```

Adoption requires one canonical materialized staircase or one explicitly supplied valid chain.

It creates a random lineage UUID, stable step UUIDs, structure, empty or supplied metadata, active lifecycle state, record refs, and cut-retention refs.

If a requested name is invalid, occupied, or archived-reserved, adoption fails. Automatic adoption may create an unnamed managed staircase rather than invent a misleading name.

Adoption MUST NOT silently claim ownership of ambiguous refs.

### 16.2 Split

```console
git staircase split <step-selector> --at <commit-ish> [--branch <local-branch>]
git staircase split <step-selector> --at <commit-ish> --no-ref
git staircase split <step-selector> --at <commit-ish> --keep-id=upper|lower
```

The split point MUST be strictly inside the selected step. It MUST NOT equal the predecessor cut, selected cut, or a commit outside the step.

A pure split changes decomposition without rewriting commits.

Default managed identity behavior gives the original step ID to the upper child ending at the old cut and gives a new ID to the inserted lower child.

`--no-ref` requires management because the new cut exists only in persistent structure.

A branch supplied for a new cut MUST be a valid unoccupied local ref or an explicitly leased owned ref.

### 16.3 Join

```console
git staircase join <lower-step> <upper-step>
git staircase join <lower-step> <upper-step> --delete-boundary-ref
git staircase join <lower-step> <upper-step> --keep-boundary-ref
git staircase join <lower-step> <upper-step> --keep-id <step-id>
```

Only adjacent steps may be joined.

The boundary cut is removed. The body and aggregate outcome remain unchanged for a pure join.

Default identity behavior retires the lower step ID and retains the upper step ID.

Keeping a ref that discovery would otherwise recognize as a cut requires managed discovery override state.

Only owned boundary refs may be deleted automatically.

### 16.4 Append and new step

```console
git staircase append <selector> --commits <revision-range>
git staircase append <selector> --new-step --branch <local-branch> --commits <revision-range>
```

Appending to the tip preserves step count. Creating a new step adds a cut and may remain implicit when the final boundary is discoverable.

The range MUST resolve to an ordered dependency-compatible commit sequence above the current top. Arbitrary unrelated history is rejected.

Materializing a staged draft is the preferred command when commits do not yet exist.

### 16.5 Move changes between steps

```console
git staircase move <selector> --from <step> --to <step> <commit-ish>...
git staircase move <selector> --from <step> --to <step> --patch <path-or-hunk-selection>
```

The command rewrites the affected step range and restacks descendants.

It MUST validate that moved commits or patches do not introduce dependencies on work remaining in a later step.

If a move would create an empty source step, the command stops for explicit drop, join, or empty-commit policy.

Stable step IDs remain with the conceptual review units selected by the plan, not with original OIDs.

### 16.6 Reorder

```console
git staircase reorder <selector> --steps <ordinal-or-step-id-list>
```

The list MUST be a permutation of all active steps unless an explicitly named partial-reorder operation is introduced later.

Reorder generally replays patches and may conflict. It preserves step IDs with conceptual changes and recomputes all cuts, ordinals, primary branches, worktree attachments, and branch configuration.

A complete successful reorder may leave an implicit staircase implicit when the final result is discoverable. Any persistent intermediate stale state requires adoption.

### 16.7 Drop

```console
git staircase drop <step-selector> --restack
git staircase drop <step-selector> --leave-descendants-stale
```

Drop removes the selected step's changes, retires its step ID, and normally replays all descendants.

`--leave-descendants-stale` requires management or automatic adoption.

The command MUST display provider review identities that will be retired, superseded, closed, or left untouched. Provider mutation is never implied by local drop unless an explicit provider option is given.

### 16.8 Rebase

```console
git staircase rebase <selector> [--onto <commit-ish>]
git staircase rebase <selector> --from <step-selector> [--onto <commit-ish>] [--leave-upper-steps-stale]
```

Without `--onto`, the command uses the currently resolved integration anchor under the precedence rules.

A complete rebase replays the active body onto the new anchor, reconstructs every cut, preserves stable lineage and step IDs when managed, updates owned refs, and records the exact anchor used.

A partial rebase that intentionally leaves upper steps stale requires management.

Review identities MAY remain stable while exact review revisions become local-newer and verification becomes stale.

The command MUST distinguish:

```text
new integration anchor
symbolic integration target
review destination
```

### 16.9 Restack

```console
git staircase restack <selector> [--from <step-selector>] [--onto <commit-ish>]
git staircase restack --steps <cut-ref-1>,<cut-ref-2>,... [--onto <commit-ish>]
```

Restack repairs upper dependencies after a lower step changed.

For a managed staircase, it uses stable intended order and predecessor identity.

For an implicit staircase that is already stale, the user must supply the intended chain explicitly because the stale relationship is not discoverable.

A successful restack reconstructs cuts, updates owned refs and layout, marks review mappings local-newer, and invalidates verification tied to old exact revisions.

### 16.10 Normalize

```console
git staircase normalize <selector>
```

`normalize` repairs derived representation, such as primary branch layout, cut-retention refs, or cached local mappings, without changing intended step order or patches.

If normalization would require choosing among incompatible structures, it fails as ambiguity rather than becoming a reshape operation.

### 16.11 Rename and name management

```console
git staircase rename <old-selector> <new-name>
git staircase unname <selector>
git staircase name <implicit-selector> <name>
```

`name` on an implicit staircase requires adoption because the name persists independently of the current refs.

`rename` changes only the public staircase ref and lifecycle audit event. It does not rename primary branches unless `layout rename` is separately invoked.

### 16.12 Delete

```console
git staircase delete <managed-selector> [--keep-owned-branches]
git staircase delete <managed-selector> --delete-owned-branches
git staircase delete --archived <archived-selector>
git staircase delete <implicit-selector> --delete-materializing-refs
```

Delete removes selected Staircase refs and records, not Git objects directly.

The command MUST separately describe effects on:

* Managed record refs.
* Owned local branches.
* Snapshot tags.
* Archive refs.
* Draft attachments and snapshots.
* Provider reviews and remote branches.

External review deletion or abandonment requires explicit provider-specific options.

Deleting an active or archived staircase with attached drafts requires explicit detach, retarget, snapshot, or abort disposition.

### 16.13 Persistent discovery overrides

Discovery overrides record intended membership or boundary interpretation that cannot be reconstructed safely from current refs and ancestry alone. They therefore require a managed staircase and change its structure revision.

Canonical commands are:

```console
git staircase discovery show <selector>
git staircase discovery include-ref <selector> <full-refname>
git staircase discovery exclude-ref <selector> <full-refname>
git staircase discovery add-cut <selector> <commit-ish>
git staircase discovery ignore-cut <selector> <commit-ish-or-full-refname>
git staircase discovery clear <selector> <override-id>
```

The commands have these meanings:

```text
include-ref
    Treat the exact ref as corroborating or materializing the selected staircase
    when its resolved commit is structurally compatible.

exclude-ref
    Exclude the exact ref from discovery evidence for the selected lineage without
    deleting or moving the ref.

add-cut
    Persist an exact commit as a step boundary even when no discoverable ref names it.

ignore-cut
    Persist that an otherwise eligible ref or commit is not a boundary for this
    staircase. It does not remove the commit or ref from Git.
```

Every override MUST store canonical full refnames or full OIDs, provenance, and the expected record revision. The implementation MUST reject an override that would create an empty step, violate ancestry, absorb unrelated history, claim ownership implicitly, or make the selected managed structure internally inconsistent.

Overrides affect only the selected lineage. They do not globally change discovery of another staircase that observes the same refs or commits.

### 16.14 Persistent policy

A persistent policy is durable semantic configuration attached to one managed staircase. Invocation-local flags do not persist policy.

Canonical commands are:

```console
git staircase policy show <selector>
git staircase policy set <selector> <key>=<value>...
git staircase policy unset <selector> <key>...
```

Core policy keys MAY include versioned discovery, verification, review-mapping, landing, merge, retention, and layout policies. Provider-defined keys MUST be namespaced and accepted only when the bound provider declares their schema.

A policy that affects interpretation, verification validity, review mapping, landing behavior, retention, or structural operations is stored in structural state and changes the structure and record revisions. Purely descriptive information belongs in metadata and MUST NOT masquerade as policy.

Policy updates MUST validate the complete resulting policy set before publication, use full-record compare-and-swap, and report any verification or provider state made stale by the change. Unknown unnamespaced keys and values outside the declared schema MUST be rejected.

---

## 17. Verification

### 17.1 Verification is typed evidence

Verification is not a structural property. Evidence applies to one exact subject and context.

Core subject kinds include:

```text
structure-aggregate
structure-prefix
commit
draft-index
draft-tracked-worktree
draft-snapshot
provider-review-revision
provider-test-merge
provider-merge-group
landed-revision
```

Provider-specific subjects may extend this list through namespaced types.

### 17.2 Aggregate and prefix verification

Aggregate verification evaluates the result of applying the top to the integration context.

Prefix verification evaluates every cumulative cut.

Commands include:

```console
git staircase verify <selector> --aggregate
git staircase verify <selector> --each-prefix
git staircase verify <selector> --profile <profile>
git staircase verify <selector> --draft=index
git staircase verify <selector> --draft=working
git staircase verify <selector> --provider <provider>
```

An invocation option does not persist policy. Setting a default verification policy requires management.

### 17.3 Verification profile

A profile may define:

```text
subject policy
build and test commands
static analysis
formatting checks
platform matrix
environment variables
tool versions
timeouts
allowed failures
provider evidence requirements
review requirements
```

Profile identity MUST be versioned or content-addressed so that evidence can be invalidated correctly.

### 17.4 Evidence key

Evidence MUST identify at least:

```text
subject type
exact subject OID or typed digest
integration anchor or exact base OID
profile identity
policy identity
environment identity
result
timestamp
execution or provider provenance
```

For a structure revision, description edits do not stale verification. Structure, anchor, profile, policy, subject, or relevant environment changes do.

### 17.5 Provider evidence

Providers MUST preserve distinctions among build checks, statuses, approvals, requested changes, mergeability, submit requirements, submittability, draft state, queue state, and exact remote revision.

A generic `passed` result is valid only when the configured Staircase policy is satisfied for the exact current subject.

Evidence for an older provider patch set, pull-request head, test merge, or merge group does not verify a rewritten local revision.

### 17.6 Cached evidence

`list` MAY display cached verification with an age marker but MUST NOT silently contact the network.

A network refresh requires an explicit verification, review-status, landing, or configured monitoring operation.

---

## 18. Review-provider contract

### 18.1 Core and provider responsibilities

The core defines review intent, exact local source revisions, step identity, mapping policy, durable associations, operation planning, and reconciliation requirements.

A review provider defines provider routes, native review identities, native revision identity, publication mechanics, server state, and provider-specific constraints.

Review publication uses:

```console
git staircase review create <selector>
git staircase review upload <selector>
```

A review command selector may identify a complete staircase or one managed step when the selected mapping supports step-scoped review operations. Step-scoped output and mutation MUST remain anchored to the enclosing staircase record revision.

Staircase record transport uses `push` and `fetch`, never `review upload`.

### 18.2 Review mapping policy

A provider mapping MUST be explicit. Generic mapping classes include:

```text
per-commit
per-step
aggregate
stacked
cumulative
provider-native
```

The provider MUST report when a topology cannot represent the requested semantics honestly.

A step may map to zero, one, or several provider reviews. One review may include several commits. The core MUST NOT assume a universal one-to-one mapping.

### 18.3 Stable review identity and exact revision

A provider association contains:

```text
provider identity and route
stable native review identity
exact current native review revision
associated staircase lineage and step or aggregate subject
mapping policy
last observed remote state
```

Stable review identity may survive local rewrite. Exact native revision changes when the provider's reviewed commit or equivalent changes.

A provider-native number or branch name is not assumed globally unique.

### 18.4 Persistent association and adoption

Creating or retaining a durable review association requires a managed staircase.

A provider MAY support:

```console
git staircase review upload <implicit-selector> --ephemeral
```

for a one-time publication with no retained mapping. The output MUST state that identity continuity is not recorded.

### 18.5 Review plan

```console
git staircase review plan <selector> [--mapping <policy>]
```

The plan MUST contain exact local OIDs, route, destination, mapping, existing identities, create/update/no-op actions, expected remote OIDs or revisions, verification consequences, and provider requirements.

Planning MAY inspect remote state only when the command permits network access.

A plan MUST NOT silently rewrite commits to satisfy provider conventions. Required normalization is a separate explicit operation or displayed rewrite plan.

### 18.6 Review create

`review create` establishes native review identities according to the selected mapping.

A provider may combine creation with initial publication internally, but the command result MUST distinguish identities created from exact revisions uploaded.

Partial or uncertain creation requires a journal and reconciliation. It MUST NOT be repeated blindly.

### 18.7 Review upload

`review upload` publishes exact immutable commit OIDs or provider-native immutable revisions.

Before mutation it MUST verify:

* The selected record or structural key remains current.
* Source refs still point to planned OIDs.
* Review route and destination remain valid.
* Expected remote leases still hold.
* Mapping policy is satisfied.
* Provider-required commit metadata is valid.

A moving local branch MUST NOT be re-resolved after plan approval without lease verification.

### 18.8 Remote head names and local layout

Provider remote branch or review locators are a separate naming layer from local sequential primary branches.

Local renumbering MUST NOT destroy stable provider review identity. A managed association records the local step ID and provider locator independently.

### 18.9 Review status

```console
git staircase review status <selector>
git staircase review show <selector>
git staircase review open <step-selector>
```

`review status` may contact the provider and MUST compare exact local and remote revisions.

It reports provider-native fields plus generic synchronization state. It MUST NOT conflate review approval, checks, mergeability, and submittability.

### 18.10 Reconcile

```console
git staircase review reconcile <selector>
```

Reconciliation resolves uncertain uploads, remote-newer state, external retargeting, deleted remote branches, closed reviews, changed mappings, and provider-side mutations whose outcome is unknown.

It MUST query by canonical provider identity and route, never by a weak title or branch-name guess.

No destructive local or remote resolution is selected silently.

### 18.11 Draft-aware review planning

```console
git staircase review plan <selector> --include-draft
```

This MAY show a hypothetical topology after materializing the current staged draft. It MUST label it hypothetical and MUST identify unstaged, untracked, and ignored content that is excluded.

Review providers operate on immutable commits by default. A combined materialize-and-upload workflow must explicitly materialize and publish local structure before remote mutation.

### 18.12 Provider uncertainty

When a network result is uncertain, the command MUST:

* Preserve the exact operation plan.
* Mark affected items `upload-unknown` or an equivalent typed state.
* Avoid assuming success or failure.
* Provide `review reconcile` as the next action.
* Avoid blind repetition.

### 18.13 Attach and detach existing reviews

A managed staircase may associate an already-existing provider review without creating or uploading it:

```console
git staircase review attach <step-or-staircase-selector> \
    --provider <provider> --review <provider-native-selector>
git staircase review detach <step-or-staircase-selector> \
    --provider <provider> [--review <provider-native-selector>]
```

`review attach` MUST resolve the provider-native selector to one canonical review identity and route, establish the exact currently reviewed revision when available, validate compatibility with the selected mapping subject, and publish the association through full-record compare-and-swap. If network validation is unavailable, the association MUST remain explicitly provisional and MUST NOT satisfy review or verification policy.

A review may not be attached to two incompatible local subjects under one mapping unless the provider and mapping policy explicitly support that relationship. Weak evidence such as title similarity, branch spelling, or a numeric identifier without its provider scope is insufficient.

`review detach` removes only the local persistent association by default. It does not close, abandon, delete, retarget, or otherwise mutate the remote review unless a provider-specific explicit option requests and plans that action.

---

## 19. Landing

### 19.1 Landing policies

A managed staircase may declare:

```text
aggregate-only
stepwise-only
aggregate-or-stepwise
required prefix verification
required review approvals
required destination
allowed merge methods
merge-commit policy
autosquash policy
queue policy
```

Invocation-local landing options do not persist policy.

### 19.2 Aggregate landing

```console
git staircase land <selector> --aggregate [--method <method>]
```

Aggregate landing integrates the complete active body as one provider or Git operation.

The command MUST record the exact destination before and after, actual graph result, method, and which local commits became reachable.

### 19.3 Stepwise landing

```console
git staircase land <selector> --stepwise [--method <method>]
git staircase land <selector> --through <step-selector> [--method <method>]
```

`--through` performs stepwise landing of the inclusive bottom prefix ending at the selected active step.

Steps land bottom to top. A later step MUST NOT land before its active predecessors.

After each step, the operation MUST:

1. Resolve the exact new destination OID.
2. Determine the actual landing method and graph.
3. Mark the landed step integrated or patch-integrated.
4. Determine whether upper steps remain valid descendants.
5. Restack upper steps when required.
6. Update provider revisions with leases.
7. Retarget dependent reviews when required.
8. Re-evaluate diffs, approvals, checks, and policy.
9. Retire provider branches only when no dependency uses them.
10. Continue only when the next step is current and permitted.

Stepwise landing is a journaled managed operation whenever continuity, reviews, or recovery state are retained.

### 19.4 Partial landing

After a lower prefix lands:

* The integration context advances.
* Landed steps leave the active body.
* Remaining steps are recomputed relative to the new context.
* Layout is renumbered only after landing is confirmed.
* Verification and review state are recalculated.

An implicit remainder may be rediscovered as a shorter staircase. Preserving the original lineage requires management.

### 19.5 Merge-method consequences

The core MUST inspect the actual resulting graph.

A merge commit may preserve reviewed commits as ancestors.

Squash or rebase landing may create new destination commits while leaving original lower commits in upper branches. Upper steps normally require restack before their review diffs or landing remain valid.

The provider's reported method is evidence, not a substitute for graph reconciliation.

### 19.6 Queue and auto-landing state

Queue, auto-merge, submit-ready, and equivalent provider states are remote operational state, not staircase identity.

Any change to head, base, dependency chain, or queue group may invalidate that state.

The core MUST not treat a chain of dependent reviews as independent queue entries unless the provider explicitly supports that topology.

### 19.7 Uncertain landing

An uncertain landing result requires exact destination and provider reconciliation before retry.

The operation journal MUST preserve pre-landing destination OID, planned subject, provider identity, and all observed responses.

---

## 20. Archive and unarchive

### 20.1 Archive meaning

```console
git staircase archive <selector> [--reason <text>] [--dry-run]
```

Archive preserves lineage, structure, metadata, lifecycle history, provider associations, owned-ref restoration information, and required object reachability while removing Staircase-owned active refs from ordinary active namespaces.

Archive is local and offline by default. It does not close reviews, abandon changes, delete remote branches, cancel verification, modify manifests, or push archive refs.

### 20.2 Archive namespace

An archived staircase uses:

```text
refs/staircase-archive/<lineage-id>/record
refs/staircase-archive/<lineage-id>/steps/<step-id>
refs/staircase-archive/<lineage-id>/owned/<ref-id>
```

Stable IDs are used in archive paths. Former refnames are stored in the archive manifest.

The active public and internal record refs MUST not exist while archived.

### 20.3 Archive manifest

The immutable manifest records:

```text
archive event ID
lineage ID
archive time and optional actor
reason
previous active record OID
canonical name
layout profile and base
owned refs and expected OIDs
archive retention refs
branch configuration snapshots
worktree observations
draft disposition
provider disposition
name reservation policy
```

### 20.4 Archive preconditions

Archive MUST:

1. Resolve one canonical staircase.
2. Adopt an implicit staircase exactly once if needed.
3. Determine ownership separately from staircase identity.
4. Acquire operation locks.
5. Verify the current record and owned refs.
6. Inspect every worktree and active operation.
7. Compute the complete ref and configuration migration.
8. Validate archive collisions.
9. Journal the operation.

Archive refuses during an active rebase, merge, cherry-pick, revert, unresolved Staircase mutation, or uncertain provider mutation unless an explicitly supported durable snapshot first resolves ownership of that state.

A managed staircase may be archived while clean, stale, diverged, incomplete, or partially landed.

### 20.5 Porcelain visibility

After archive, Staircase-owned active refs MUST leave:

```text
refs/staircases/
refs/heads/
refs/staircase-state/
```

They therefore disappear from ordinary active Staircase listing and local branch listing.

Unowned aliases, remote-tracking refs, and ordinary snapshot tags remain unless explicitly and validly handled.

Archive is porcelain invisibility, not confidentiality. Explicit full-ref enumeration may reveal archive refs and objects.

### 20.6 Worktrees during archive

A clean worktree attached to an owned branch is detached at the same exact commit by default before that branch is removed.

A dirty attached worktree blocks archive unless the user chooses:

```console
git staircase archive <selector> --snapshot-drafts
git staircase archive <selector> --detach-dirty-worktrees
git staircase archive <selector> --leave-worktrees
```

`--leave-worktrees` is valid only when no worktree remains symbolically attached to a branch being removed.

Conflicted worktrees block archive unless their owning operation is resolved or explicitly snapshotted by a supported mechanism.

### 20.7 Branch configuration and reflogs

Owned `branch.<name>.*` configuration is captured and removed so a later unrelated branch cannot inherit it.

Ref changes and configuration changes require a journal.

Archive guarantees reachability through archive refs, not byte-for-byte migration of historical branch reflogs.

### 20.8 Name reservation

An archived canonical name is reserved by default.

```console
git staircase archive release-name <selector>
```

releases the name without deleting the staircase or changing lineage.

Former branch names are restoration preferences, not enforceable reservations.

### 20.9 Operations while archived

Allowed operations include:

```text
show
list --archived
metadata inspection and editing
structure inspection and diff
verification of immutable revisions
export and transport
unarchive
explicit delete
```

Structural mutation, materialization, review mutation, upload, and landing are prohibited by default. The command MUST require explicit unarchive and MUST NOT silently reactivate.

Metadata edits while archived update the archive record and remain archived.

### 20.10 Unarchive

```console
git staircase unarchive <selector> [options]
```

Default restoration attempts to recreate the canonical name, active internal refs, owned primary branches and aliases, branch configuration, layout policy, and draft attachments or snapshots according to the manifest.

It does not reattach worktrees by default.

Options include:

```console
git staircase unarchive <selector> --name <new-name>
git staircase unarchive <selector> --branch-base <new-base>
git staircase unarchive <selector> --branches=exact
git staircase unarchive <selector> --branches=rename
git staircase unarchive <selector> --branches=none
git staircase unarchive <selector> --adopt-existing-branches
git staircase unarchive <selector> --reattach-worktrees
```

### 20.11 Unarchive collisions

An active staircase-name collision with another lineage fails unless another name is supplied.

An existing unowned branch at the expected OID is still independent. It requires explicit adoption after worktree, configuration, ownership, tracking, and provider checks.

An existing branch at another OID MUST NOT be overwritten.

`--branches=none` creates an active branchless managed staircase using internal cut refs.

Configuration sections are not merged automatically.

Worktree reattachment occurs only when the worktree still exists, is detached, remains at the expected OID, has compatible draft state, and would follow the same conceptual step.

### 20.12 Idempotency

Archiving an already archived lineage and unarchiving an already active lineage are successful no-ops when selection is unambiguous. They do not create duplicate events.

An existing interrupted lifecycle journal must be resumed, aborted, or reconciled before another lifecycle operation begins.

### 20.13 Provider and remote state

Archive preserves provider associations but performs no provider or remote mutation by default. Remote-tracking refs remain observations maintained by fetch configuration and may continue to expose related names.

After unarchive, a provider association MUST be revalidated before review upload, verification refresh, queue mutation, or landing. A closed review, deleted remote branch, changed route, inaccessible repository, or remote-newer revision is reported as provider state and does not invalidate local lineage.

Explicit provider-side archive actions, when supported, require a separate plan and MUST state whether they close, abandon, relabel, retarget, or delete remote entities.

### 20.14 Garbage collection and retention

Archive retention refs MUST make every required record, cut, commit, metadata object, lifecycle object, manifest, and selected recovery object reachable to Git. Textual OIDs in blobs are insufficient.

Archive has no automatic expiration. A retention policy MAY propose pruning, but deletion MUST identify which objects would lose their final known Staircase ref and MUST honor ordinary Git reachability, reflog, cruft, and garbage-collection behavior without promising immediate object destruction.

### 20.15 Delete remains separate

```console
git staircase delete --archived <selector>
```

may remove the final retention refs after displaying what remains reachable elsewhere.

Deleting refs does not guarantee immediate object deletion and MUST NOT be described as secure erasure.

---

## 21. Metadata

### 21.1 Fields

A managed staircase may have:

```text
human title
multiline description
labels
links
per-step titles, descriptions, labels, and links
namespaced extension fields
creation and update provenance
```

Per-step metadata is keyed by stable step ID.

Labels are exact, case-sensitive, unordered, and duplicate-free.

Links contain a stable link ID, relationship, URI, and optional display fields. A URL resembling a review does not create review identity.

### 21.2 Commands

```console
git staircase describe <selector>
git staircase describe <selector> --edit
git staircase metadata show <selector>
git staircase metadata edit <selector>
git staircase metadata set-title <selector> <title>
git staircase metadata add-label <selector> <label>
git staircase metadata remove-label <selector> <label>
git staircase metadata add-link <selector> --relation <relation> --url <uri>
git staircase metadata show-step <step-selector>
git staircase metadata edit-step <step-selector>
```

Editing title, description, labels, or links changes metadata and record revisions only.

### 21.3 Editor concurrency

An edit session reads one expected record OID. On publication it compares that complete record.

If another process rebases, archives, changes policy, updates review association, or edits metadata, the edit fails with `concurrent-record-update` and applies nothing.

A retry MUST reload the new record and require a new user decision. It MUST NOT blindly replay serialized old metadata.

---

## 22. Transport

### 22.1 Staircase transport

```console
git staircase push [<remote>] [<selector>...]
git staircase fetch [<remote>]
git staircase push --include-archived
git staircase fetch --include-archived
```

These commands transport Staircase records, refs, metadata, archive state, and required objects. They do not publish provider reviews.

### 22.2 Explicit refspecs

Custom Staircase refs are not assumed to travel under ordinary branch or tag refspecs.

A transport implementation MUST use explicit valid refspecs or a Staircase-aware protocol for:

```text
refs/staircases/*
refs/staircase-state/*
refs/staircase-archive/*
```

Fetched remote state SHOULD live under remote-tracking Staircase namespaces and remain an observation until imported or activated.

### 22.3 Lineage and name collisions

Equal lineage with equal record agrees.

A known record descendant MAY fast-forward according to policy.

Incompatible record histories under one lineage are divergence and MUST NOT be assigned a new lineage silently.

Equal names with different lineages are name collisions, not identity matches.

### 22.4 Remote atomicity

A push changing several refs SHOULD request atomic push.

If the remote cannot provide required atomicity, the command MUST refuse or use a descriptor-last recoverable protocol and clearly report partial-publication risk.

### 22.5 Reachability

Transport MUST include all objects needed to inspect, verify, resume, restore, or unarchive the selected records. Textual OIDs alone are insufficient.

---

## 23. Output, diagnostics, and exit behavior

### 23.1 Output modes

Commands that enumerate or inspect state SHOULD support:

```text
human
porcelain
json
```

For `list` the options are:

```console
git staircase list --human
git staircase list --porcelain
git staircase list --json
```

They are mutually exclusive. Human is the default.

Other commands SHOULD use the same mode names rather than inventing incompatible format flags.

### 23.2 Empty list

After all explicit filters, the **matching set** contains the canonical staircases to print. Unresolved discovery evidence is diagnostic, not a matching staircase.

When the matching set is empty:

* Human mode writes exactly `No staircases.` followed by one newline.
* Porcelain mode writes zero bytes to standard output.
* JSON mode writes exactly `[]` followed by one newline.
* Exit status is `0` unless strict diagnostics fail.

### 23.3 JSON top-level types

`git staircase list --json` always emits a JSON array.

Workspace bootstrap, warnings, and diagnostics MUST NOT change the top-level type.

A typical managed entry includes full values:

```json
{
  "kind": "managed",
  "lifecycle": "active",
  "name": "feature",
  "refname": "refs/staircases/feature",
  "lineage_id": "c2e9e7bb-b803-4970-b268-cb0aa51758d2",
  "record_revision_oid": "<full-oid>",
  "structure_revision_oid": "<full-oid>",
  "metadata_revision_oid": "<full-oid>",
  "lifecycle_revision_oid": "<full-oid>",
  "integration_context": {
    "kind": "single-anchor",
    "anchor_oid": "<full-commit-oid>",
    "symbolic_target_ref": "refs/remotes/example/main"
  },
  "structural_state": "clean",
  "layout_state": "clean",
  "steps": [
    {
      "id": "<uuid>",
      "ordinal": 1,
      "cut_oid": "<full-commit-oid>",
      "primary_branch": "refs/heads/feature-1"
    }
  ]
}
```

A typical implicit entry includes:

```json
{
  "kind": "implicit",
  "structural_key": "implicit@<full-digest>",
  "canonical_name": "feature",
  "aliases": [],
  "integration_context": {
    "kind": "single-anchor",
    "anchor_oid": "<full-commit-oid>"
  },
  "cuts": ["<full-commit-oid>"],
  "top_oid": "<full-commit-oid>",
  "materializing_refs": ["refs/heads/feature"]
}
```

### 23.4 Human abbreviations

Human output MAY abbreviate OIDs, lineage IDs, step IDs, and structural keys only enough to remain unique in the displayed scope.

Every identifier MUST have an unambiguous label, for example:

```text
lineage ID:             c2e9e7bb
structure revision:     4f91cc2a
record revision:        a83de019
cut commit:             771fe2c4
implicit structural key: implicit@13a2fd80
```

### 23.5 Standard error

Machine-readable standard output MUST not be contaminated by bootstrap announcements or progress prose.

Normal progress SHOULD be suppressed in porcelain and JSON modes.

Warnings may appear on standard error only when they affect completeness or correctness. Structured diagnostics SHOULD be available through `--diagnostics=json` or an equivalent typed channel.

Human mode may print a one-time automatic workspace-configuration announcement to standard error when configuration changes.

### 23.6 Strict listing

```console
git staircase list --strict
```

returns nonzero when unresolved or ambiguous discovery candidates exist, even if the canonical matching set is empty or nonempty.

Output formatting still follows the selected mode. JSON may print `[]` and return nonzero with separate diagnostics.

### 23.7 Ambiguity output

An ambiguous selected command MUST:

* Return nonzero.
* Perform no adoption, record write, ref change, worktree change, or provider mutation.
* Show every viable candidate unless explicitly truncated with a count.
* Show the structural dimension that differs.
* Suggest the same command with one disambiguating typed selector per candidate.

For an ambiguous `archive`, suggestions MUST be `archive` commands, not only inspection commands.

### 23.8 Stable error codes

Machine output SHOULD use stable codes including:

```text
selector-not-found
selector-ambiguous
selection-changed
discovery-integrity-error
integration-context-unresolved
invalid-cut-chain
empty-step
ref-ownership-ambiguous
ref-collision
worktree-blocked
draft-basis-stale
conflicted-index
active-git-operation
operation-in-progress
operation-conflict
concurrent-record-update
provider-unbound
provider-route-incomplete
provider-authentication-unavailable
remote-newer
remote-diverged
remote-outcome-unknown
verification-stale
landing-blocked
archived-mutation
unarchive-collision
transport-nonatomic
```

Provider-specific codes MUST be namespaced or included as provider details beneath a stable core code.

### 23.9 Exit statuses

The initial exit-status classes are:

```text
0   success, including defined idempotent no-op
1   usage or validation failure
2   selection or ambiguity failure
3   concurrent local state change or lease failure
4   operation conflict requiring resolution
5   provider, network, or authentication failure
6   verification or landing policy failure
7   internal integrity failure
```

Implementations MAY refine these statuses, but machine-readable error codes are authoritative.

### 23.10 Idempotent no-ops

A successful no-op MUST be distinguishable from a state transition in structured output.

An ambiguous selector never becomes a no-op.

---

## 24. Security and trust

### 24.1 Input handling

Selectors, refs, provider values, paths, URLs, metadata, review titles, labels, reviewer names, and server responses are untrusted data.

They MUST NOT be:

* Evaluated as shell code.
* Interpolated into shell command strings.
* Split on whitespace.
* Treated as filesystem paths unless the option requires a path.
* Expanded as globs unless the option explicitly accepts a pattern.

Commands accepting arbitrary names MUST support `--` to terminate option parsing.

Batch plumbing SHOULD be NUL-delimited where Git supports it.

### 24.2 Git refs and objects

The implementation MUST use Git ref and object APIs or plumbing. It MUST NOT assume refs or reflogs are loose files.

Every OID MUST be typed and checked against the repository object format. Implementations MUST NOT assume SHA-1 length.

A ref update MUST use an expected old value or nonexistence lease.

### 24.3 Providers

Passive provider discovery MUST not:

* Fetch or contact services.
* Prompt for authentication.
* Read or copy credentials unnecessarily.
* Run presubmit.
* Upload work.
* Install hooks.
* Modify manifests, workspace metadata, remotes, Git config, or refs.

Provider credentials MUST NOT be stored in Staircase records, refs, commit messages, or workspace discovery records.

### 24.4 Remote mutations

Force or non-fast-forward publication MUST be explicit, lease-protected, and tied to a known stable provider identity.

Uncertain remote outcomes require reconciliation, not blind retry.

### 24.5 Metadata rendering

Metadata and provider text may contain control characters, terminal escapes, misleading links, or markup. Human clients MUST render it safely.

Archive is not a secrecy mechanism, and hidden or custom refs are not an authorization boundary.

---

## 25. Canonical command surface

The core command vocabulary is:

| Command | Meaning |
|---|---|
| `discover` | Find canonical candidates from current structure. |
| `list` | List canonical staircases and families in scope. |
| `show` | Inspect one staircase. |
| `status` | Report local structure, layout, lifecycle, drafts, and operations. |
| `graph`, `log`, `diff`, `steps`, `commits` | Inspect history and decomposition. |
| `rev-parse`, `id` | Resolve typed identities, including stable managed step identity. |
| `tag` | Create an immutable annotated snapshot name for one exact record revision. |
| `adopt` | Convert implicit structure into managed state. |
| `split` | Insert a step boundary. |
| `join` | Remove an adjacent boundary. |
| `append` | Add commits to the tip or add a new tip step. |
| `move` | Transfer commits or patches between steps. |
| `reorder` | Change step order. |
| `drop` | Remove one step's changes. |
| `rebase` | Replay selected staircase work onto a new integration anchor. |
| `restack` | Repair upper dependencies after lower work changed. |
| `normalize` | Repair derived representation without changing intended work. |
| `discovery` | Inspect or edit persistent discovery overrides for a managed staircase. |
| `policy` | Inspect or edit persistent typed policy. |
| `continue`, `abort` | Resume or abort the active Staircase operation. |
| `operation show` | Inspect the active operation journal. |
| `layout` | Manage primary branch layout. |
| `draft` | Inspect, attach, snapshot, restore, verify, or materialize worktree drafts. |
| `verify` | Evaluate a typed subject under a profile. |
| `review` | Plan, create, attach, detach, upload, inspect, or reconcile provider reviews. |
| `land` | Integrate reviewed work into a landing destination. |
| `metadata`, `describe` | Inspect or edit user-facing metadata. |
| `archive`, `unarchive` | Change managed lifecycle state. |
| `delete` | Remove selected managed or explicitly selected materializing refs. |
| `push`, `fetch` | Transport Staircase state. |
| `workspace` | Inspect and maintain workspace bindings. |
| `provider` | Inspect provider readiness and diagnostics. |

The following aliases are not canonical and MUST NOT be introduced as synonyms:

```text
track            for adopt
indent           for split
unindent         for join
restore          for unarchive
remove           for drop or delete
sync             for rebase or restack
commit-draft     for materialize
upload           for review upload
create           for adopt, append --new-step, or draft materialize
step create      for append --new-step or split
show-step        for show with a step selector
step-id          for id <step-selector> --kind=step
rename-step      for layout branch
include          for discovery include-ref
ignore-cut       for discovery ignore-cut
```

### 25.1 Workspace commands

```console
git staircase workspace show
git staircase workspace discover
git staircase workspace providers
git staircase workspace refresh
git staircase workspace doctor
git staircase workspace configure
git staircase workspace forget
```

### 25.2 Provider diagnostics

```console
git staircase provider <provider> doctor
```

This reports applicability, binding, route readiness, authentication requirements, supported capabilities, stale evidence, and rejected candidates without exposing secrets.

### 25.3 Identity command

```console
git staircase id <selector> --kind=lineage
git staircase id <selector> --kind=structure
git staircase id <selector> --kind=record
git staircase id <selector> --kind=body
git staircase id <selector> --kind=decomposition
git staircase id <selector> --kind=outcome
git staircase id <selector> --kind=patch-series
git staircase id <selector> --kind=nominal
git staircase id <step-selector> --kind=step
```

Requesting lineage or stable step identity for an implicit staircase triggers adoption unless `--no-adopt` is supplied. Nominal identity is a display or selector identity only; it is not lineage and may change when refs or naming evidence change.

---

## 26. Normative invariants

A conforming implementation preserves all of the following.

### 26.1 Structural invariants

1. Cuts are strictly ordered by ancestry.
2. Every step contains at least one commit.
3. The body is dependency-closed relative to the integration set.
4. A staircase has one aggregate top.
5. Given integration context and ordered cuts, decomposition is deterministic.
6. Incomparable cuts are separate staircases or family paths.

### 26.2 Discovery invariants

1. Raw evidence is not a staircase.
2. Equivalent candidates collapse.
3. Equal top is not sufficient for equivalence.
4. One canonical implicit staircase has one structural key.
5. One listed entry is one selectable object.
6. Unique displayed names resolve uniquely under unchanged state.
7. Genuine same-name candidates are disambiguated during listing.
8. Empty bodies are not synthetic one-step staircases.
9. Structural-key differences must be explainable.
10. A mutating command remains bound to its selected key.

### 26.3 Identity invariants

1. Refs name evolving things.
2. OIDs name immutable Git objects.
3. Lineage is not content-derived.
4. Step identity is not ordinal position.
5. Full refs and full typed OIDs are persisted.
6. Implicit labels are not promoted accidentally.
7. Rename does not change lineage or record content.
8. Provider review identity and exact review revision remain distinct.

### 26.4 Persistence invariants

1. Structure, metadata, lifecycle, and record revisions remain distinct.
2. One logical read uses one immutable record.
3. Every persistent mutation compares the complete expected record.
4. CAS conflicts fail without last-write-wins.
5. New objects exist before refs point to them.
6. Required commits have real reachability refs.
7. Multi-surface operations are consistently completed or deterministically recoverable.

### 26.5 Branch and worktree invariants

1. Only owned refs are mutated automatically.
2. Sequential names follow positions; step IDs follow conceptual work.
3. The tip alone receives the bare layout base.
4. Numbering is contiguous from `1`.
5. Complete destination planning precedes mutation.
6. Worktrees follow conceptual steps, not inherited spellings.
7. Branch configuration follows conceptual steps.
8. Dirty work is refused, preserved losslessly, or retained in recovery state.

### 26.6 Draft invariants

1. Drafts are worktree-scoped and basis-specific.
2. Index and worktree remain distinct.
3. A draft is not a committed step.
4. The index is the default materialization source.
5. Untracked and ignored content require explicit inclusion.
6. Exact basis equality governs automatic attachment.
7. A conflicted index is not a candidate tree.
8. Draft verification is typed and not silently promoted.
9. Nested repository state remains nested.
10. No draft content or attachment is silently lost or reassigned.

### 26.7 Provider and landing invariants

1. Capability bindings are typed and independently replaceable.
2. Passive discovery is offline and nonmutating.
3. Integration context and review destination remain distinct.
4. Review mapping is explicit.
5. Upload plans use immutable exact revisions and leases.
6. Uncertain remote results are reconciled.
7. Verification applies to exact subjects.
8. Upper dependent reviews do not land before lower dependencies.
9. Actual landing graph determines restack and retarget behavior.
10. Provider state does not silently redefine local structure.

### 26.8 Lifecycle invariants

1. Metadata is not structure.
2. Lifecycle is not structure.
3. Archived owned branches leave ordinary active namespaces.
4. Archive is not secrecy.
5. Archive preserves deterministic restoration reachability.
6. Archive is local and offline by default.
7. Dirty worktrees are not discarded.
8. Unarchive never overwrites an unowned branch.
9. Archived names are reserved by default.
10. Archive is reversible; delete is separate.

### 26.9 Output invariants

1. Human empty list says `No staircases.`.
2. Porcelain empty list writes zero bytes.
3. JSON empty list writes `[]`.
4. Machine output is not contaminated by bootstrap prose.
5. All machine identifiers are full and typed.
6. Ambiguity diagnostics include every candidate and usable command-specific remedies.

---

## 27. Provider accommodation requirements

Separate provider specifications may specialize behavior, but a provider conforming to the core MUST satisfy these contracts.

### 27.1 Workspace providers

A workspace provider may supply:

```text
workspace identity and root
project identity and path
exact checkout anchor
moving integration candidates
review endpoint hints
review destination hints
transport hints
```

These values remain separately typed. A workspace provider does not thereby become a review provider. Hints are evidence inputs to another capability probe; they do not bind that capability or authorize network access or mutation.

It MUST NOT mutate the workspace during passive bootstrap.

### 27.2 Repository-routing providers

A repository-routing provider may distinguish and resolve:

```text
local Git repository
hosted base repository
hosted head repository
fetch route
push route
review-publication route
landing destination repository
```

It MUST NOT assign authority from conventional remote names such as `origin` or `upstream`. It MUST preserve provisional locator identity separately from network-confirmed stable repository identity, and it MUST expose ambiguity when several remotes or hosted repositories remain plausible.

Repository routing does not by itself choose an integration context, review destination, or landing policy. Those decisions consume route evidence through their own typed capabilities.

### 27.3 Review providers

A review provider must define:

```text
canonical provider identity
route identity
stable review identity scope
exact native review revision
supported mapping topologies
publication and lease model
remote-state reconciliation
provider verification fields
landing capabilities
uncertain-outcome behavior
```

It MUST identify when one provider-native topology cannot represent an incremental staircase honestly.

### 27.4 Verification providers

A verification provider must identify exact subject, base, profile, policy, result fields, freshness conditions, and evidence provenance.

It MUST preserve distinctions required by the provider rather than flattening all state into one Boolean.

### 27.5 Transport providers

A transport provider must distinguish review publication from Staircase-state transport and must declare atomicity, lease, reachability, and partial-publication behavior.

### 27.6 Landing providers

A landing provider must report the actual destination transition and resulting graph or provider-native equivalent. It must not assume the planned merge method actually occurred.

### 27.7 Provider-specific extensions

Provider-specific fields in records or metadata MUST be namespaced and preserved when unknown.

A provider extension MUST NOT silently change core meaning without an explicit capability or policy definition.

---

## 28. Summary definition

A staircase is a finite filtration of a dependency-closed region of a Git commit graph, interpreted relative to a typed integration context, whose successive differences form ordered nonempty review steps.

An implicit staircase is a canonical reconstruction of current structure.

A managed staircase adds stable lineage, stable step identities, persistent policy, metadata, lifecycle, review associations, and recovery for states that current Git ancestry cannot express.

The core design rules are:

> Prefer reconstructible Git structure over metadata, but persist intent before it would be lost.

> Listing is a promise of addressability.

> Step IDs follow the work. Sequential branch names follow the shape.

> The index proposes history. The worktree proposes changes to that proposal. Only commits become staircase history.

> Metadata explains the staircase. Structure defines it. Lifecycle determines whether it is active.

> Plan against immutable revisions, publish with leases, and reconcile uncertainty instead of guessing.

---
# Appendix A: User journeys

## A.1 Conventions used in the transcripts

This appendix is normative for command names, operation boundaries, selector behavior, and the information that output must communicate.

Concrete OIDs, UUIDs, review numbers, paths, timestamps, and provider-native identifiers are illustrative. An implementation may use different abbreviations and spacing in human output, but it MUST preserve every material fact shown. JSON and porcelain modes remain governed by their schemas rather than by the visual layout below.

Lines beginning with `$` are commands. Unless a line is explicitly labeled as an ordinary Git or workspace-manager command, it is a `git staircase` command.

When a command writes informational bootstrap or warning text to standard error, the transcript labels it `[stderr]`. Normal human output is labeled `[stdout]` only where the distinction matters.

The sample repository uses SHA-1 abbreviations for readability. Implementations MUST support the repository's actual object format and MUST use full OIDs in machine output and persistent records.

---

## A.2 Journey 1: A difficult `repo` and Gerrit review cycle

### A.2.1 Starting state

A developer is in project `platform/payments` inside a `repo` workspace. The project checkout began detached at manifest commit `a100000`. Three local branches materialize a sequential staircase:

```text
payments-1 -> b110000  Add ledger model
payments-2 -> c120000  Route writes through ledger
payments   -> d130000  Add migration and tests
```

Each step contains one Gerrit review commit with a valid, distinct Change-Id. No Staircase record exists yet.

The first command bootstraps workspace, integration-context, review, and verification capability bindings from passive local evidence, then continues the requested listing:

```console
$ git staircase list
[stderr] Configured Staircase workspace:
[stderr]   workspace:            repo
[stderr]   project:              platform/payments
[stderr]   integration-context:  repo
[stderr]   review:               gerrit
[stderr]   verification:         gerrit
[stderr]
[stdout] payments  3 steps  clean  sequential  (implicit)
```

The listing is a promise that `payments` uniquely selects the displayed canonical implicit staircase while relevant state is unchanged.

The developer inspects it:

```console
$ git staircase show payments
payments (implicit)
  structural key: implicit@6a22c4781f8d
  integration anchor: a100000
  symbolic integration target: refs/remotes/m/main
  state: clean
  steps: 3

  1  payments-1  b110000  1 commit  Add ledger model
  2  payments-2  c120000  1 commit  Route writes through ledger
  3  payments    d130000  1 commit  Add migration and tests

  review provider: gerrit
  review destination: review.example.com/platform/payments refs/heads/main
  review identities: not associated
```

The developer wants stable review and step identity, so explicitly adopts the staircase:

```console
$ git staircase adopt payments
Adopted 'payments'.
  lineage: 7c1f287e-0b7b-48ec-8901-758c73030f5c
  steps: 3
  name: payments
  primary branch layout: sequential
  structure revision: 241ea87
  record revision: 88ad41c
```

The developer asks for a nonmutating review plan. Gerrit-specific details are supplied by the review provider, but the core still identifies exact local subjects and the mapping policy:

```console
$ git staircase review plan payments --mapping per-commit
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a100000
  structure revision: 241ea87
  mapping: one review per commit

  step  commit    stable review key                         action
  1     b110000   I1111111111111111111111111111111111111111 create
  2     c120000   I2222222222222222222222222222222222222222 create
  3     d130000   I3333333333333333333333333333333333333333 create

No remote mutation performed.
```

`review create` establishes durable review associations. In this route, creation is completed by the first publication, so the command records pending identities without pushing:

```console
$ git staircase review create payments
Prepared 3 review identities for 'payments'.
  step 1: I1111111111111111111111111111111111111111  pending first upload
  step 2: I2222222222222222222222222222222222222222  pending first upload
  step 3: I3333333333333333333333333333333333333333  pending first upload

record revision: 88ad41c -> 9bf5a72
Remote publication is still required.
```

The exact planned commits are uploaded:

```console
$ git staircase review upload payments
Uploaded 'payments' to Gerrit.
  destination: review.example.com/platform/payments refs/heads/main
  structure revision: 241ea87

  step  commit    change  patch set  result
  1     b110000   48101   1          created
  2     c120000   48102   1          created
  3     d130000   48103   1          created

Associated 3 reviews.
record revision: 9bf5a72 -> 31f477a
```

A status query proves that remote observations correspond to the exact current local commits:

```console
$ git staircase review status payments
payments
  provider: gerrit
  destination: platform/payments refs/heads/main

  step  local     review  remote revision  review state       verification
  1     b110000   48101   b110000          needs review       pending
  2     c120000   48102   c120000          needs review       pending
  3     d130000   48103   d130000          needs review       pending
```

### A.2.2 Review round one: amend the bottom step and resolve two restack conflicts

Reviewers request a schema change in step 1. The developer checks out the bottom step in a linked worktree and records rewrite intent:

```console
$ git staircase draft attach payments:1 --mode=rewrite-step
Attached current worktree draft.
  staircase: payments
  step: 1
  step ID: f6ee1b5f-01ae-4971-8699-2bb915a260ec
  intent: rewrite-step
  expected basis: b110000
```

After editing, the developer stages only the intended schema files with ordinary Git:

```console
$ git add ledger/schema.proto ledger/model.cc
```

The draft status preserves the distinction between staged and unstaged content:

```console
$ git staircase draft status
current worktree draft
  attached to: payments step 1
  intent: rewrite-step
  basis: b110000
  staged: 2 paths
  unstaged: 1 path
  untracked: 0 paths
  conflicts: none
```

The developer amends step 1. Upper steps must be replayed. Step 2 conflicts because it edited the old ledger API:

```console
$ git staircase draft materialize payments:1 --amend
Materialized staged draft into step 1.
  old cut: b110000
  new cut: b211000
  unstaged worktree changes preserved

Restacking upper steps...
  step 2 of 3: conflict
  source cut: c120000
  new predecessor: b211000
  conflicted paths:
    ledger/writer.cc

Operation: restack 53e2f58d
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase and draft, run:
  git staircase abort
```

The active operation owns the conflict state. Other structural commands refuse to start:

```console
$ git staircase reorder payments --steps 1,3,2
error: Staircase operation 53e2f58d is active
  kind: restack
  state: conflict
  next: git staircase continue
  abort: git staircase abort
```

The developer resolves and stages `ledger/writer.cc`, then continues:

```console
$ git add ledger/writer.cc
$ git staircase continue
Resolved step 2.
  old cut: c120000
  new cut: c221000

Restacking upper steps...
  step 3 of 3: conflict
  source cut: d130000
  new predecessor: c221000
  conflicted paths:
    ledger/migration_test.cc

Operation: restack 53e2f58d
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase and draft, run:
  git staircase abort
```

A second resolution completes the operation atomically:

```console
$ git add ledger/migration_test.cc
$ git staircase continue
Restack complete for 'payments'.

  step  old cut   new cut
  1     b110000   b211000
  2     c120000   c221000
  3     d130000   d231000

  primary branches updated: 3
  review revisions: local-newer for 3 reviews
  verification: stale
  structure revision: 241ea87 -> 17ef0c1
  record revision: 31f477a -> d0834ba
```

The unstaged file remains unstaged. It was not swept into the amended review commit:

```console
$ git staircase draft status
current worktree draft
  attached to: payments step 1
  intent: rewrite-step
  basis: b211000
  staged: 0 paths
  unstaged: 1 path
  untracked: 0 paths
  conflicts: none
```

Uploading creates new patch sets while preserving review identities:

```console
$ git staircase review upload payments
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b211000   48101   2          updated
  2     c221000   48102   2          updated
  3     d231000   48103   2          updated

record revision: d0834ba -> f5be6f2
```

### A.2.3 Review round two: rebase the whole staircase and conflict at nonadjacent steps

The manifest branch advances from `a100000` to `a300000`. A workspace refresh obtains the new exact integration candidate without changing staircase commits:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  previous integration anchor: a100000
  current integration anchor: a300000
  symbolic integration target: refs/remotes/m/main

Affected active staircases:
  payments  behind integration anchor  verification stale
```

The developer rebases the complete staircase. Step 1 conflicts first:

```console
$ git staircase rebase payments
Rebasing 'payments'.
  old integration anchor: a100000
  new integration anchor: a300000
  structure revision: 17ef0c1

  step 1 of 3: conflict
  source cut: b211000
  conflicted paths:
    ledger/schema.proto

Operation: rebase 3d86fd02
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase, run:
  git staircase abort
```

After the first resolution, step 2 applies cleanly but step 3 conflicts:

```console
$ git add ledger/schema.proto
$ git staircase continue
Resolved step 1.
  new cut: b311000

  step 2 of 3: applied
  new cut: c321000

  step 3 of 3: conflict
  source cut: d231000
  conflicted paths:
    ledger/migration.cc
    ledger/migration_test.cc

Operation: rebase 3d86fd02
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase, run:
  git staircase abort
```

The second resolution completes the rebase:

```console
$ git add ledger/migration.cc ledger/migration_test.cc
$ git staircase continue
Rebase complete for 'payments'.
  integration anchor: a100000 -> a300000

  step  old cut   new cut
  1     b211000   b311000
  2     c221000   c321000
  3     d231000   d331000

  primary branches updated: 3
  review revisions: local-newer for 3 reviews
  verification: stale
  structure revision: 17ef0c1 -> 8eb860d
  record revision: f5be6f2 -> bfa8427
```

The developer runs local prefix verification before publication:

```console
$ git staircase verify payments --each-prefix --profile presubmit
Verification profile: presubmit
  integration anchor: a300000
  structure revision: 8eb860d

  prefix  subject   result  duration
  1       b311000   passed  02:11
  2       c321000   passed  03:04
  3       d331000   passed  04:38

Result: passed
Evidence recorded for 3 exact subjects.
```

Uploading advances all three Gerrit patch sets:

```console
$ git staircase review upload payments
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b311000   48101   3          updated
  2     c321000   48102   3          updated
  3     d331000   48103   3          updated
```

### A.2.4 A `repo sync` conflict, owned by the external rebase

Later, the developer runs `repo sync` in the project's baseline worktree. The workspace manager starts an ordinary Git rebase for a local baseline-only commit and stops on a conflict:

```console
$ repo sync platform/payments
error: could not apply e400000... Preserve local test fixture
CONFLICT (content): Merge conflict in testdata/accounts.json
```

While that rebase is active, Staircase reports it but does not seize or reinterpret it:

```console
$ git staircase status payments
payments
  state: clean
  active Staircase operation: none

current worktree
  active Git operation: rebase
  owner: external
  likely initiator: repo sync
  conflicted paths:
    testdata/accounts.json

Staircase structural mutation is blocked in this worktree.
Resolve with ordinary Git and continue or abort the external rebase:
  git add <resolved-paths>
  git rebase --continue
  git rebase --abort
```

The developer resolves it using ordinary Git:

```console
$ git add testdata/accounts.json
$ git rebase --continue
Successfully rebased and updated detached HEAD.
$ repo sync platform/payments
Fetching: 100% (1/1), done in 0.842s
```

The resulting manifest checkout anchor is now `a500000`. The staircase refs were not rewritten by `repo sync`, so a refresh reports that the staircase needs rebasing rather than silently changing it:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  previous integration anchor: a300000
  current integration anchor: a500000

Affected active staircases:
  payments  behind integration anchor  verification stale
```

Rebasing now conflicts in the middle step only:

```console
$ git staircase rebase payments
Rebasing 'payments'.
  old integration anchor: a300000
  new integration anchor: a500000

  step 1 of 3: applied
  new cut: b511000

  step 2 of 3: conflict
  source cut: c321000
  conflicted paths:
    ledger/writer.cc

Operation: rebase 04eab708
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase, run:
  git staircase abort
```

The developer resolves and finishes:

```console
$ git add ledger/writer.cc
$ git staircase continue
Resolved step 2.
  new cut: c521000

  step 3 of 3: applied
  new cut: d531000

Rebase complete for 'payments'.
  integration anchor: a300000 -> a500000
  primary branches updated: 3
  review revisions: local-newer for 3 reviews
  verification: stale
  structure revision: 8eb860d -> f8c365a
```

### A.2.5 Review round three: split the middle review without losing existing review identities

A reviewer asks that the API transition and call-site migration be reviewed separately. The middle step currently contains two commits after an earlier cleanup. The developer inserts a cut at commit `c515000`:

```console
$ git staircase split payments:2 --at c515000
Split step 2 of 'payments'.
  new lower step ID: 1af93789-a8b0-4f34-9057-d49ac00a7746
  surviving upper step ID: 4566f9ac-52a6-48ae-84c1-35ea8cd9b0da
  steps: 3 -> 4

  position  branch      cut
  1         payments-1  b511000
  2         payments-2  c515000
  3         payments-3  c521000
  4         payments    d531000

  existing review 48102 remains with the surviving upper step
  new lower step has no review identity
  structure revision: f8c365a -> 8b02a65
```

The provider plan exposes exactly one new review and three updates. It does not duplicate the old Change-Id:

```console
$ git staircase review plan payments --mapping per-commit
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a500000
  structure revision: 8b02a65

  step  commit    review  action
  1     b511000   48101   update
  2     c515000   none    create; Change-Id required
  3     c521000   48102   update
  4     d531000   48103   update

No remote mutation performed.
```

The developer adds a Change-Id to the new commit through the configured commit-message workflow. This ordinary Git amendment moves the owned lower boundary branch and makes the managed upper relationship stale until Staircase restacks it:

```console
$ git switch payments-2
Switched to branch 'payments-2'
$ git commit --amend
[payments-2 c516000] Separate API transition
```

The developer then restacks, prepares, and uploads the new review association:

```console
$ git staircase restack payments --from payments:2
Restack complete for 'payments'.
  changed steps: 2..4
  review revisions: local-newer for 3 existing reviews
  new review identities required: 1

$ git staircase review create payments:2
Prepared review identity for 'payments' step 2.
  stable review key: I4444444444444444444444444444444444444444
  pending first upload

$ git staircase review upload payments
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b511000   48101   4          updated
  2     c516000   48144   1          created
  3     c522000   48102   4          updated
  4     d532000   48103   4          updated
```

Finally, the provider reports that every exact current revision satisfies review and verification policy:

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'
  provider: gerrit
  structure revision: 92f55fd

  step  review  exact revision  code review  presubmit  submit requirements
  1     48101   b511000         approved     passed     satisfied
  2     48144   c516000         approved     passed     satisfied
  3     48102   c522000         approved     passed     satisfied
  4     48103   d532000         approved     passed     satisfied

Result: passed for the exact current review revisions.
```

This journey demonstrates why the core separates:

```text
workspace synchronization from Staircase rebase
external Git operation ownership from Staircase operation ownership
stable review identity from exact patch-set revision
stable step identity from positional branch naming
local verification evidence from provider verification evidence
```

---

## A.3 Journey 2: Reshape a discovered multi-branch stack without adopting it

### A.3.1 Starting state

A standalone repository has integration anchor `origin/main` at `1000000` and branches:

```text
feature-1 -> 1100000
feature-2 -> 1200000
feature   -> 1300000
```

The staircase is fully discoverable from ancestry and the complete sequential layout.

```console
$ git staircase list
feature  3 steps  clean  sequential  (implicit)
```

The developer inspects exact cuts:

```console
$ git staircase steps feature
feature (implicit)
  1  feature-1  1100000  2 commits
  2  feature-2  1200000  3 commits
  3  feature    1300000  1 commit
```

The second step contains a useful internal commit `1180000`. The developer splits there. Because the new cut is represented by the sequential branch layout, no durable information is needed:

```console
$ git staircase split feature:2 --at 1180000
Split step 2 of implicit staircase 'feature'.
  steps: 3 -> 4

  position  branch     cut
  1         feature-1  1100000
  2         feature-2  1180000
  3         feature-3  1200000
  4         feature    1300000

The resulting staircase remains implicit.
```

The developer reorders the two middle review units. Staircase captures the complete old shape, rewrites all affected commits, updates all four branches as one logical operation, and leaves a discoverable final shape:

```console
$ git staircase reorder feature --steps 1,3,2,4
Reorder complete for implicit staircase 'feature'.

  old position  new position  old cut   new cut   branch
  1             1             1100000   1100000   feature-1
  3             2             1200000   1210000   feature-2
  2             3             1180000   1220000   feature-3
  4             4             1300000   1310000   feature

The resulting staircase remains implicit.
```

The final listing still has no lineage because none was required:

```console
$ git staircase show feature
feature (implicit)
  structural key: implicit@c8bdfe2132d4
  integration anchor: 1000000
  state: clean
  steps: 4
  lineage: none
```

This workflow is possible with raw Git, but coordinating replay, four branch movements, collision checks, checked-out worktrees, rollback, and final renumbering normally requires a brittle hand-built script.

---

## A.4 Journey 3: Create a metadata-only cut and trigger automatic adoption

A one-branch implicit staircase has six commits but should be reviewed as two steps. The developer wants no additional local branch at the boundary:

```console
$ git staircase show parser
parser (implicit)
  integration anchor: 2000000
  steps: 1
  cut: 2600000
  commits: 6
```

The split point is commit `2300000`. A metadata-only cut cannot be reconstructed from ordinary refs, so the command explains and performs automatic adoption before publishing the change:

```console
$ git staircase split parser:1 --at 2300000 --no-ref
Adopted implicit staircase 'parser'.
  reason: the requested cut would not be reconstructible from Git refs
  lineage: 2e6c78ea-5d2e-4f74-ac19-f6656120ca62

Split step 1 of 'parser'.
  new lower step ID: 4f8dd9a9-fdbe-480e-a213-eb7cfa0190ee
  surviving upper step ID: c3658f2e-c4b1-4961-ae9d-b4a36938ff91
  steps: 1 -> 2
  branch changes: none
  structure revision: 93fba84
  record revision: e02ce37
```

The lower step has no branch but is fully selectable by managed step identity or current ordinal:

```console
$ git staircase steps parser
parser
  lineage: 2e6c78ea-5d2e-4f74-ac19-f6656120ca62

  position  step ID                               primary branch  cut
  1         4f8dd9a9-fdbe-480e-a213-eb7cfa0190ee  none            2300000
  2         c3658f2e-c4b1-4961-ae9d-b4a36938ff91  parser          2600000
```

After the lower cut's commit is amended, the upper step becomes stale until restacked. That remembered relationship is another state raw Git cannot express by ancestry alone:

```console
$ git staircase rebase parser --from parser:1 --onto 2100000 --leave-upper-steps-stale
Rebased step 1 of 'parser'.
  old cut: 2300000
  new cut: 2310000
  upper steps left stale by explicit request

parser
  state: stale
  stale from: step 2
  next: git staircase restack parser
```

Restack later restores a clean materialization without changing lineage or the stable upper step ID:

```console
$ git staircase restack parser
Restack complete for 'parser'.
  step 2: 2600000 -> 2610000
  state: clean
  lineage unchanged: 2e6c78ea-5d2e-4f74-ac19-f6656120ca62
```

---

## A.5 Journey 4: Edit a lower step from another worktree while preserving partial staging

### A.5.1 Two worktrees, one managed staircase

A managed staircase `auth` has three steps and three linked worktrees:

```text
/work/auth-model  -> auth-1
/work/auth-api    -> auth-2
/work/auth-ui     -> auth
```

The developer is in `/work/auth-model`, where one file is partially staged and another has only unstaged experimentation:

```console
$ git staircase draft status
current worktree draft
  attached to: auth step 1
  intent: extend-step
  basis: 3110000
  staged: 2 paths
  unstaged: 2 paths
  partially staged paths: 1
  untracked: 1 path
  conflicts: none
```

The exact index, not whole worktree files, is materialized:

```console
$ git staircase draft materialize auth:1 -m "Handle token expiry skew"
Materialized staged draft into 'auth' step 1.
  parent: 3110000
  tree: 8e08d66
  new commit: 3120000
  new cut: 3120000

Preserved in current worktree:
  unstaged tracked changes: 2 paths
  untracked files: 1 path

Restacking upper steps...
  step 2: applied -> 3220000
  step 3: conflict
  conflicted paths:
    ui/session_view.ts

Operation: restack 95fa123b
```

The worktree for conceptual step 3 follows that step even though its old branch tip cannot yet be published. During the operation, its branch and worktree remain protected by temporary refs. After resolving the conflict:

```console
$ git add ui/session_view.ts
$ git staircase continue
Restack complete for 'auth'.

  step  old cut   new cut   primary branch       worktree
  1     3110000   3120000   auth-1               /work/auth-model
  2     3210000   3220000   auth-2               /work/auth-api
  3     3310000   3320000   auth                 /work/auth-ui

Current draft preserved:
  worktree: /work/auth-model
  basis: 3120000
  staged: 0 paths
  unstaged: 2 paths
  untracked: 1 path
```

A raw multi-worktree rebase would require the user to coordinate branch checkouts, temporary refs, index snapshots, worktree branch attachments, and failure recovery by hand.

---

## A.6 Journey 5: Land the bottom step, rebase the remainder, and preserve conceptual identity

A four-step managed staircase `search` has passed verification. The landing provider supports stepwise landing. Only step 1 is approved for immediate integration.

The developer previews the exact plan:

```console
$ git staircase land search --through search:1 --dry-run
Landing plan for 'search'
  integration anchor: 4000000
  structure revision: 4ae1f82
  method: provider-selected stepwise
  landing prefix: step 1

  land:
    step 1  cut 4100000  step ID 7bd0fb5e-...

  retain and restack:
    step 2  cut 4200000  step ID 51e00ae4-...
    step 3  cut 4300000  step ID b958c97c-...
    step 4  cut 4400000  step ID d8c0ccf4-...

No local or remote mutation performed.
```

The provider lands step 1 as commit `4110000`. Staircase verifies the actual destination transition, advances the integration context, removes the landed step from the active body, and restacks the remaining conceptual steps:

```console
$ git staircase land search --through search:1
Landed prefix of 'search'.
  landed steps: 1
  previous integration anchor: 4000000
  resulting integration anchor: 4110000
  provider outcome: confirmed

Restacked remaining steps:
  former position  current position  step ID       old cut   new cut
  2                1                 51e00ae4-...   4200000   4210000
  3                2                 b958c97c-...   4300000   4310000
  4                3                 d8c0ccf4-...   4400000   4410000

Primary branch layout:
  search-1 -> 4210000
  search-2 -> 4310000
  search   -> 4410000

State: clean, partially landed
Lineage unchanged: 6943232f-c13f-48da-bcf7-a1d7160b5866
```

The old positional names changed, but the three surviving step IDs and review associations followed their conceptual work. Verification for old exact revisions is stale; lineage continuity is preserved.

```console
$ git staircase review status search
search
  state: partially landed
  active steps: 3

  step  step ID       review identity  local revision  state
  1     51e00ae4-...  review-202       4210000         local-newer
  2     b958c97c-...  review-203       4310000         local-newer
  3     d8c0ccf4-...  review-204       4410000         local-newer
```

This is the sort of operation where ordinary Git branch names become positional shrapnel. Staircase keeps the human layout tidy while stable IDs carry continuity.

---

## A.7 Journey 6: Archive a whole staircase, handle a name collision, and restore branchlessly

A completed experiment `vector-cache` owns three local branches and has an attached dirty draft in one linked worktree.

An initial archive attempt refuses because no draft disposition was specified:

```console
$ git staircase archive vector-cache
error: 'vector-cache' has an attached dirty draft
  worktree: /work/vector-cache
  staged: 1 path
  unstaged: 3 paths

Choose a disposition:
  git staircase archive vector-cache --snapshot-drafts
  git staircase archive vector-cache --detach-dirty-worktrees
```

The developer preserves a durable staged/unstaged snapshot:

```console
$ git staircase archive vector-cache --snapshot-drafts --reason "experiment complete"
Archived 'vector-cache'.
  lineage: d77bd7ac-fb13-4347-841c-9d9b6619ea8a
  archive record: refs/staircase-archive/d77bd7ac-fb13-4347-841c-9d9b6619ea8a/record
  draft snapshots created: 1
  owned branches archived: 3
    refs/heads/vector-cache-1
    refs/heads/vector-cache-2
    refs/heads/vector-cache
  remote changes: none
  canonical name reserved: vector-cache
```

Ordinary branch porcelain no longer shows the owned branches:

```console
$ git branch --list 'vector-cache*'
```

The default Staircase list also excludes it:

```console
$ git staircase list
No staircases.
```

Archived state remains inspectable:

```console
$ git staircase list --archived
vector-cache  3 steps  archived  experiment complete
```

Months later, an unrelated branch now occupies `refs/heads/vector-cache`. Unarchive refuses to overwrite it:

```console
$ git staircase unarchive vector-cache
error: cannot restore 'vector-cache'
  destination ref exists and is not owned by the archived staircase:
    refs/heads/vector-cache -> 9f90000

Choose one:
  git staircase unarchive vector-cache --branches=none
  git staircase unarchive vector-cache --branch-base <new-base>
  rename or remove the conflicting branch, then retry
```

The developer restores the managed staircase without active local branches:

```console
$ git staircase unarchive vector-cache --branches=none
Unarchived 'vector-cache' without primary branches.
  lineage: d77bd7ac-fb13-4347-841c-9d9b6619ea8a
  steps restored: 3
  owned branches restored: 0
  retained cuts: 3
  draft snapshots available: 1
  state: clean
```

A new layout can be materialized later without touching the unrelated branch:

```console
$ git staircase layout set vector-cache --primary-branches=sequential --base vector-cache-v2
Applied sequential primary branch layout.
  vector-cache-v2-1 -> 7100000
  vector-cache-v2-2 -> 7200000
  vector-cache-v2   -> 7300000
```

---

## A.8 Journey 7: Resolve two distinct implicit staircases with the same human name

Two independent candidate chains both derive the name `cleanup`. They share the same top commit but have different integration anchors and lower cuts. Same top is not structural equivalence.

```console
$ git staircase list
cleanup@3f9444c2  2 steps  clean  (implicit)
  integration anchor: 8100000
  cuts: 8200000, 8400000

cleanup@e2b1a971  3 steps  clean  (implicit)
  integration anchor: 8050000
  cuts: 8150000, 8300000, 8400000
```

The bare name is correctly ambiguous:

```console
$ git staircase show cleanup
error: staircase name 'cleanup' is ambiguous

  cleanup@3f9444c2
    structural key: implicit@3f9444c2e3d4c189...
    integration anchor: 8100000
    cuts: 8200000, 8400000

  cleanup@e2b1a971
    structural key: implicit@e2b1a971a9ec7994...
    integration anchor: 8050000
    cuts: 8150000, 8300000, 8400000

Use one of the displayed structural keys.
```

A typed structural selector is deterministic:

```console
$ git staircase show --structural-key implicit@3f9444c2e3d4c189...
cleanup (implicit)
  structural key: implicit@3f9444c2e3d4c189...
  integration anchor: 8100000
  steps: 2
  top: 8400000
```

Archiving that implicit staircase automatically adopts only the selected canonical object:

```console
$ git staircase archive --structural-key implicit@3f9444c2e3d4c189...
Adopted implicit staircase 'cleanup'.
  lineage: 76eb7f96-bb10-4223-a6fd-c8dcf68f9701

Archived 'cleanup'.
  owned branches archived: 2
  remote changes: none
```

The other `cleanup` candidate remains active and implicit. Duplicate discovery evidence for either candidate would have been collapsed before listing and could never have created this ambiguity.

---

## A.9 Journey 8: Extract one path from a shared-prefix staircase family

The graph contains a common `core` prefix and two incomparable children:

```text
                 ui -> 9300000
                /
base -> core -> 9200000
                \
                 cli -> 9400000
```

Staircase refuses to flatten incomparable tips into one linear staircase:

```console
$ git staircase discover --families
platform-family  2 paths  shared prefix 1 step  (implicit)

  path platform-ui:
    core  9200000
    ui    9300000

  path platform-cli:
    core  9200000
    cli   9400000
```

A path selector is required for a linear operation:

```console
$ git staircase show platform-family
error: 'platform-family' is a staircase family, not a linear staircase

Select a path:
  git staircase show platform-ui
  git staircase show platform-cli
```

The developer adopts the family because shared-prefix ownership must survive future rewrites:

```console
$ git staircase adopt platform-family
Adopted staircase family 'platform-family'.
  family ID: c2dc40ad-5734-4a93-a95c-234be430a93b
  shared step ID: 6a303058-4220-4362-8bf8-1e1476206ba1
  paths: 2
```

A correction to the shared `core` step is made once, then both paths are restacked independently. One conflicts while the other applies cleanly:

```console
$ git staircase draft materialize platform-family:core --amend
Materialized staged draft into shared step 'core'.
  old cut: 9200000
  new cut: 9210000

Restacking path platform-ui...
  step ui: applied -> 9310000

Restacking path platform-cli...
  step cli: conflict
  conflicted paths:
    cli/renderer.cc

Operation: family-restack 2e8c2ba7
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
```

After resolution, Staircase publishes both paths while retaining one shared conceptual prefix:

```console
$ git add cli/renderer.cc
$ git staircase continue
Family restack complete for 'platform-family'.
  shared core: 9200000 -> 9210000
  platform-ui tip: 9300000 -> 9310000
  platform-cli tip: 9400000 -> 9410000
  paths clean: 2
```

Raw Git can represent this graph, but it has no first-class place to say that one rewritten prefix is intentionally shared by two review paths and must be propagated to both as one operation.

---

## A.10 Journey 9: Reconcile an uncertain review upload instead of duplicating reviews

A network connection drops after a review upload. The transport cannot determine whether the remote accepted the final two revisions:

```console
$ git staircase review upload routing
warning: review upload outcome is uncertain
  operation: review-upload 44a10bce
  definitely accepted: step 1
  uncertain: steps 2, 3
  definitely rejected: none

The upload plan and exact source revisions were preserved.
Do not repeat the upload blindly.
Next:
  git staircase review reconcile routing
```

`review status` distinguishes unknown publication outcome from an ordinary pending review:

```console
$ git staircase review status routing
routing
  step 1: synchronized
  step 2: upload-unknown
  step 3: upload-unknown
```

Reconciliation queries by stable provider review identity and exact revision. It discovers that step 2 was accepted and step 3 was not:

```console
$ git staircase review reconcile routing
Reconciled review upload operation 44a10bce.

  step  planned revision  remote observation  result
  1     a110000           a110000             already confirmed
  2     a120000           a120000             accepted
  3     a130000           absent               not published

Safe retry plan contains step 3 only.
```

A subsequent upload uses the reconciled expected remote state:

```console
$ git staircase review upload routing
Uploaded 'routing'.
  step 3: a130000  created
  unchanged steps skipped: 2
```

This closes a particularly sharp Git workflow trap: uncertainty is stored as a recoverable operation state, not converted into accidental duplicate reviews.

---

# Appendix B: Automatic-adoption decision table

The following table is normative. An operation automatically adopts an implicit staircase only when its requested result cannot remain fully reconstructible from Git objects, refs, the integration context, discovery policy, and invocation arguments.

| Operation | Remains implicit when | Requires adoption when |
| --- | --- | --- |
| `discover`, `list`, inspection | Always; they observe current structure | Never solely for observation |
| Revision-derived identity | Identity is computed from exact current structure | A lineage ID or stable step ID is requested |
| `verify` | Evidence is keyed only by exact subject, anchor, profile, policy, and environment | Persistent lineage-relative policy or history is recorded |
| `split` | New cut is represented by an unambiguous discoverable ref/layout | Cut exists only in Staircase metadata, or stable child identity must persist |
| `join` | Removed boundary also ceases to be discoverable | Boundary ref remains but must be ignored, or metadata must preserve retired identity |
| Append commit to tip step | Final structure remains discoverable | Persistent review/policy association must be updated by lineage |
| Add a new step | New cut has a discoverable ref | New cut is metadata-only |
| `reorder` | Complete rewrite finishes atomically into a clean discoverable layout | Intermediate stale state, persistent identity, or provider mapping must survive |
| `move` changes between steps | Complete final decomposition remains discoverable | Stable step identity or a nonmaterialized intermediate state is needed |
| `rebase` complete staircase | Final chain and cuts remain discoverable and no continuity is requested | Stable identity, provider associations, or stale upper state must survive |
| `restack` clean implicit chain | Intended chain is supplied explicitly and final result is discoverable | The stale relationship itself must be remembered after command completion |
| `archive` | Never | Archive is persistent lifecycle state, so implicit selection is adopted first |
| Persistent name, description, labels, links | Never | Always |
| Persistent discovery override | Never | Always, because the intended inclusion or boundary is not wholly reconstructible |
| Persistent policy | Never | Always, because policy must remain attached across invocations and rewrites |
| Immutable snapshot tag | The target is already an exact managed record revision | An implicit staircase must first be adopted to obtain a durable record target |
| Persistent draft attachment | Never | Always, because it names stable lineage/step intent |
| Invocation-local draft materialization | Result remains discoverable and no persistent intent is needed | Materialization creates metadata-only steps or persistent stale state |
| Review association | Provider association is purely recomputed observation and not retained | Stable remote review identity is durably associated with lineage or steps |
| Partial landing | Remaining upper work is simply rediscovered as a new implicit staircase | Continuity with the pre-landing staircase must be preserved |
| `delete` implicit materializing refs | User explicitly selects exact refs and no Staircase record is needed | A managed lifecycle or ownership record must first be created to perform the requested semantics |

Automatic adoption MUST be reported in human output and represented as a structured event in machine output. `--no-adopt` causes a command that requires adoption to fail before mutation with the reason and a reconstructible alternative when one exists.

---

# Appendix C: Corner-case resolution table

This table summarizes required resolutions for frequently ambiguous or failure-prone cases. More detailed normative rules appear in the main body.

| Case | Required behavior |
| --- | --- |
| Local integration branch points exactly at its integration anchor | Do not discover an empty one-step staircase |
| Local branch is ahead of its integration anchor, even when named `main` | It may be a valid one-step staircase; spelling does not exempt it |
| Two discovery sources find the same repository, integration context, and ordered cuts | Collapse them into one canonical implicit staircase and merge provenance |
| Two candidates share a top but have different lower cuts | Keep both; same top is insufficient for equivalence |
| Two candidates share cuts but use exact symbolic targets resolving to the same anchor | Collapse them, retaining both symbolic locators as provenance |
| Distinct repositories contain identical OIDs | Keep them distinct through repository identity and object format |
| Two distinct canonical staircases derive the same name | List both with unique structural-key qualification; the bare name is ambiguous |
| `list` prints a bare unique name, then another command sees hidden duplicate raw evidence | Nonconforming; all commands select from the same canonicalized set |
| Abbreviated structural keys collide | Extend displayed prefixes until unique; machine output always uses full keys |
| A selector can mean a managed name, Git revision, and implicit name | Resolve all interpretations, collapse semantic equality, otherwise report every distinct candidate |
| A suggested ambiguity remedy is itself ambiguous | Nonconforming; suggest typed selectors or full structural/lineage identifiers that actually disambiguate |
| Revision range supplied where one commit is required | Reject it as a set, not silently reinterpret it |
| Blob or tree expression supplied where a commit is required | Reject after type checking |
| A cut would create an empty step | Reject before mutation |
| Candidate body is empty after removing integrated cuts | Do not list an active implicit staircase |
| Incomparable tips have similar names | Report a family or separate staircases; never silently linearize them |
| Two managed staircases overlap or one is a prefix of another | Keep lineage and policy separate; enforce exclusive ref ownership and require explicit coordination for any mutation that would affect both |
| Merge commit admits several parent paths | Require explicit mainline or decomposition policy when it affects the staircase |
| Lower step rewritten and upper branch still descends from old cut | This can be `stale` only for a managed staircase; an implicit view sees separate candidates unless the chain is explicitly supplied to `restack` |
| Several refs point to one cut | Treat them as aliases or materializing refs; ownership remains separate |
| Persistent include/exclude or cut override conflicts with current ancestry | Reject the override or report the managed staircase inconsistent; never coerce unrelated history into the lineage |
| The same ref is included by discovery overrides in two lineages | Observation may overlap, but ownership remains exclusive and mutations require explicit coordination |
| Snapshot tag destination already exists | Fail unless force replacement is explicit, leased, and reports old and new exact record revisions |
| Branch name looks sequential but ancestry/layout is incomplete | Do not infer sequential ownership from the suffix alone |
| Destination sequential branch exists and is unowned | Refuse before rewriting commits or refs |
| Sequential renaming contains cycles such as `F-1 -> F-2`, `F-2 -> F-1` | Plan final names and publish through temporary refs in one transaction, never pairwise overwrite |
| A primary branch is checked out in another worktree | Move that worktree with the conceptual step when safe, or fail before publication with the exact blocker |
| A non-primary alias points at a moved cut | Leave it unchanged unless explicitly owned |
| Branch configuration belongs to a conceptual step that is renumbered | Move applicable configuration with the step; never copy blindly to unrelated aliases |
| Dirty worktree exists during a structural rewrite | Preserve it exactly, snapshot it, or fail before mutation; never silently stage or discard it |
| File is partially staged | Commit the exact index only; preserve the unstaged part |
| Index has unmerged entries | Active operation owns the conflict; ordinary materialization is forbidden |
| Ignored files exist | Exclude by default and do not mark the draft dirty solely for their presence unless policy says otherwise |
| Untracked files exist | Report separately; include only through an explicit option and policy |
| Sparse checkout hides paths changed by a rewrite | Operate on Git trees/indexes, not visible files alone; warn before checkout-dependent actions |
| Clean/smudge filters or line-ending conversion apply | Treat the index/tree as authoritative for staged content; do not reconstruct from worktree bytes |
| Dirty submodule exists | Report nested state separately; containing repository cannot silently snapshot or commit submodule worktree changes |
| External merge, rebase, cherry-pick, or `repo sync` rebase is active | Report and defer to the external operation; do not convert it into a Staircase operation |
| Staircase operation is interrupted | Preserve operation refs, expected record revision, draft snapshots, and deterministic `continue`/`abort` paths |
| Process dies after objects are written but before refs move | Objects may remain unreachable; authoritative refs remain old, and retry/recovery is safe |
| Process dies after some refs move | Nonconforming when the backend supports transactions; otherwise operation journal and recovery MUST make partial publication explicit |
| Metadata is edited while structure changes concurrently | Full-record compare-and-swap fails; do not merge or overwrite silently |
| Named active record's public and internal refs disagree | Treat as consistency failure; do not choose one by precedence |
| Archived metadata is edited | Update only archived record refs; do not reactivate the staircase |
| Archive sees unowned aliases into the staircase | Leave them and warn; OID equality is not ownership |
| Archive sees active Git/Staircase operation | Refuse until completed, aborted, or converted to durable recovery state |
| Archive sees dirty attached draft | Require explicit snapshot, detach, retarget, or discard disposition |
| Unarchive destination branch is occupied | Refuse, choose new layout, or restore branchlessly; never overwrite unowned refs |
| Archived canonical name is reused | Reservation policy governs; default is to preserve the archived name reservation |
| Remote review branch or review record exists during local archive | Leave remote state unchanged unless explicit provider options request otherwise |
| Network drops during review upload | Record uncertain outcome and require identity-based reconciliation before retry |
| Provider says checks passed for an older revision | Do not treat the current local or remote revision as verified |
| Provider review identity survives a rebase | Preserve stable association while marking exact revision local-newer and verification stale |
| Existing remote review is attached by a weak title, branch, or unscoped number | Reject; require one canonical provider identity and route, or retain an explicitly provisional association that satisfies no policy |
| Detaching a review association | Remove local association only by default; remote abandonment or deletion requires a separate explicit provider mutation |
| Provider topology cannot represent incremental steps honestly | Refuse or label the actual cumulative/aggregate topology; never pretend it is incremental |
| Review creation and initial upload are inseparable for a provider | Provider may combine them internally, but output and stored state MUST make both semantic effects explicit |
| Staircase transport and review publication target the same server | Still use distinct commands and plans; one moves Staircase records, the other provider review revisions |
| Landing result differs from planned merge method | Reconcile actual destination graph before changing local staircase state |
| Lower prefix lands while upper steps remain | Advance integration context, remove integrated work, preserve managed continuity when requested, and restack remaining steps |
| Landing outcome is uncertain | Preserve the plan and mark landing unknown; do not restack or delete local state based on a guess |
| Integration target ref moves during planning | Use the exact resolved anchor and compare expected state before publication; replan on mismatch |
| Provider bootstrap evidence is ambiguous | Bind only unambiguous capabilities; provider-independent commands continue with safe fallback where possible |
| Passive provider probe would require network or repository code execution | It is ineligible for automatic bootstrap |
| Workspace disappears or project mapping changes | Invalidate the relevant automatic binding, retain records, and require re-resolution before consequential operations |
| Empty list in human mode | Print `No staircases.` followed by newline |
| Empty list in porcelain mode | Emit zero bytes |
| Empty list in JSON mode | Emit `[]` followed by newline |

---

# Appendix D: Minimal end-to-end conformance scenarios

An implementation claiming conformance SHOULD automate at least the following black-box scenarios. These are compact acceptance tests complementing the richer journeys above.

## D.1 Empty repository view

```console
$ git staircase list
No staircases.

$ git staircase list --porcelain
```

The second command emits zero bytes.

```console
$ git staircase list --json
[]
```

## D.2 Canonical duplicate collapse

Given two refs and two discovery paths that normalize to the same integration anchor and ordered cuts:

```console
$ git staircase list --json
[
  {
    "kind": "implicit",
    "canonical_name": "feature",
    "aliases": ["feature-copy"],
    "structural_key": "implicit@<full-digest>",
    "cuts": ["<full-cut-1>", "<full-cut-2>"],
    "materializing_refs": [
      "refs/heads/feature",
      "refs/heads/feature-copy"
    ]
  }
]
```

There is exactly one array entry.

## D.3 Full-record concurrency rejection

Process A opens a metadata edit against record `R1`. Process B rebases the staircase and publishes record `R2`. Process A then tries to save:

```console
$ git staircase metadata edit auth
error: staircase changed while metadata was being edited
  expected record revision: R1
  current record revision: R2
  metadata was not published
```

The implementation may preserve A's edited temporary file, but MUST NOT silently apply it to `R2`.

## D.4 Transactional split and branch renumber

Given `feature-1`, `feature-2`, and `feature`, splitting step 1 yields a complete four-branch layout or no published change. There is no successful state in which the structure has four steps but owned primary branches still express three.

## D.5 Draft preservation on abort

A partially staged draft exists before a conflicting rebase. After:

```console
$ git staircase abort
Aborted rebase operation <operation-id>.
  staircase restored to record revision: <old-record>
  owned refs restored: <count>
  current worktree draft restored:
    staged: <count> paths
    unstaged: <count> paths
    untracked: <count> paths
```

The index tree and worktree content match their pre-operation snapshots exactly, subject only to documented filesystem limitations that were diagnosed before the operation began.

## D.6 Archive porcelain invisibility

After successful archive, no Staircase-owned active branch remains under `refs/heads/`, no public active name remains under `refs/staircases/`, and the lineage remains reachable under `refs/staircase-archive/<lineage-id>/`.

## D.7 Provider revision freshness

A provider reports approval and passing checks for remote revision `P1`, while local review source is `P2`:

```console
$ git staircase verify auth --provider <provider>
Provider verification for 'auth'
  local review revision: P2
  verified provider revision: P1
  result: stale
```

The command exits nonzero under a policy requiring current-revision verification.