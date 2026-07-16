# Addendum E: `repo` Workspace, Project-Mapping, Integration-Context, and Workspace-Hints Provider

## 1. Status and scope

This addendum defines the Git Staircase provider for workspaces managed by the `repo` multi-repository tool.

It specializes the provider contracts in **Git Staircase Specification**, especially:

* Workspace and provider architecture.
* Integration-context resolution.
* Provider accommodation requirements.
* Output, recovery, and security requirements.

When Gerrit is used, this addendum composes with **Addendum F: Gerrit Review and Verification Provider**.
When monorepo worktrees are used, it composes with **Addendum G: Monorepo Worktree Management**.
The providers remain separate:

```text
repo
    workspace
    project-mapping
    integration-context
    workspace-hints

Gerrit
    review
    review-identity
    verification
    review-transport
```

The `repo` provider MAY implement:

```text
workspace
project-mapping
integration-context
workspace-hints
```

It MAY emit typed hints for:

```text
repository-routing
review
verification
review-transport
transport
```

A hint is evidence supplied to another capability probe. It does not bind that capability, authorize network access, or authorize mutation.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

---

## 2. Conceptual model

### 2.1 `repo` client

A **`repo` client** is one checked-out workspace rooted at a directory containing provider-owned `.repo` state and an effective manifest.

A client may contain:

* One or many Git projects.
* Local manifests.
* Included manifests.
* Submanifests.
* Multiple checkouts of the same manifest project.
* Projects pinned to commits, tags, branches, or other supported revision expressions.
* Projects checked out detached or attached to local development branches.

The `repo` client is a Staircase workspace candidate. It is not one Git repository and is not one staircase.

### 2.2 Effective manifest

The **effective manifest** is the fully resolved project configuration after applying all supported manifest semantics, including inheritance, includes, local manifests, project extensions, and submanifest ownership.

The provider MUST reason from the effective manifest. Reading one top-level XML file is insufficient.

### 2.3 Manifest project

A **manifest project** is one effective project entry. Its relevant identity and routing fields may include:

```text
project name
project path
manifest remote
manifest revision
translated local revision
upstream ref
destination branch
fetch URL
review endpoint
```

These fields remain separately typed. No field named merely `target` is conforming.

### 2.4 Project checkout

A **project checkout** is one local worktree corresponding to one manifest project entry.

The local checkout identity includes at least:

```text
workspace ID
project name
project path
Git common directory
```

Project name alone is not a local checkout identity because one client may check out the same project at several paths.

### 2.5 Workspace checkout evidence

**Workspace checkout evidence** is the exact commit currently materialized in a project worktree, together with facts needed to decide whether that commit is eligible as an integration anchor.

The provider may report the current full `HEAD` OID as exact checkout evidence. It MUST NOT automatically certify every current `HEAD` as integrated history.

### 2.6 Manifest intent and checkout state

The provider distinguishes:

```text
exact current checkout
manifest-declared revision
locally translated manifest revision
manifest upstream locator
review destination hint
```

A moving manifest ref may have advanced after the workspace was last synchronized. The current checkout can therefore be an exact historical anchor while the manifest ref expresses moving future intent.

---

## 3. Provider identity

The canonical provider name is:

```text
repo
```

Recommended descriptor:

```json
{
  "protocol_version": 1,
  "name": "repo",
  "capabilities": [
    "workspace",
    "project-mapping",
    "integration-context",
    "workspace-hints"
  ],
  "probe": {
    "passive": true,
    "network": false,
    "authenticates": false,
    "mutates_workspace": false,
    "executes_repository_hooks": false
  }
}
```

The implementation MAY be built into Staircase or installed in a trusted provider location. The core MUST NOT execute an arbitrary program from the repository or ordinary `PATH` merely because its name resembles a `repo` provider.

---

## 4. Non-goals

The `repo` provider does not:

* Upload changes for review.
* Interpret Gerrit Change-Ids.
* Create, update, abandon, submit, or query Gerrit changes.
* Interpret Gerrit labels, checks, submit requirements, or submittability.
* Run presubmit.
* Land changes.
* Make a `repo` topic equivalent to a staircase.
* Make one manifest project equivalent to one staircase.
* Coordinate one repository-local staircase across several Git projects.
* Infer that a branch named `main`, `master`, or `trunk` is authoritative.
* Infer that a Git remote named `origin` is authoritative.
* Require the current project to have an attached branch.
* Treat the current local branch tip as the workspace integration anchor.
* Run `repo sync` during bootstrap or refresh.
* Modify the active manifest, local manifests, or files under `.repo`.
* Install hooks.
* Place Staircase records or workspace configuration under `.repo`.
* Treat a manifest review endpoint as authenticated, reachable, or authorized.

A future explicitly invoked mutating provider may delegate to `repo` operations, but that behavior is outside this addendum.

---

## 5. Workspace identity and scope

### 5.1 User-local workspace record

Automatic bootstrap MAY create a user-local, non-versioned Staircase workspace record. It MUST be stored outside:

```text
source repositories
.repo
manifest repositories
project Git common directories
```

The record contains:

```text
workspace ID
provider = repo
canonical client-root locator
effective-manifest fingerprint
known project mappings
capability bindings and provenance
last successful validation
```

The workspace ID SHOULD be an opaque user-local UUID. It MUST NOT be derived solely from the absolute filesystem path.

### 5.2 Client-root locator

The canonical client-root path is a locator, not durable identity.

If a client is moved and the provider can prove continuity from its effective manifest identity and project mappings, the workspace record MAY update the locator while retaining the workspace ID.

If continuity is ambiguous, the provider MUST create or request selection of a distinct workspace record rather than merge two clients based on similar paths.

### 5.3 Nested clients and submanifests

Several `repo` client candidates may enclose the current directory.

The provider MUST choose the innermost candidate that:

1. Has a valid effective manifest.
2. Declares the current Git checkout as one project.
3. Owns that project after submanifest resolution.

Directory proximity alone is insufficient.

A submanifest may define the applicable client-root and manifest identity for one project even when a parent client encloses it physically.

### 5.4 Internal repositories

The following MUST NOT be treated as ordinary source projects during automatic discovery:

```text
.repo/repo
.repo/manifests
.repo/manifests.git
.repo/project-objects/*
.repo/projects/*
```

An explicit diagnostic or administrative command MAY inspect them, but they do not establish ordinary project membership.

### 5.5 Specialized provider and fallback

When the `repo` provider proves that the current Git repository belongs to a larger `repo` client, the built-in `core.git` fallback MUST NOT create workspace ambiguity.

If the `repo` claim is degraded or conflicting, repository-local read-only commands MAY continue in invocation-local `core.git` mode. Workspace-wide commands MUST fail until the workspace is selected or repaired.

---

## 6. Passive workspace probe

### 6.1 Probe requirements

The passive probe MUST:

* Use no network access.
* Perform no authentication.
* Perform no Git ref, configuration, index, or worktree mutation.
* Perform no manifest or `.repo` mutation.
* Execute no repository hook.
* Execute no repository-supplied provider binary or script.
* Complete within bounded time and output size.
* Return schema-validated typed evidence.

### 6.2 Candidate detection

A high-confidence candidate requires all of:

1. A recognizable enclosing `repo` client root.
2. A valid effective manifest or supported local metadata interface.
3. Proof that the current Git common directory or canonical worktree maps to exactly one effective project entry.
4. A stable project name and path.
5. No unresolved conflict with another specialized workspace claim.

A directory named `.repo` is weak evidence when project membership cannot be established.

### 6.3 Canonical paths

The provider MUST compare canonicalized forms of:

```text
current directory
current worktree root
current Git common directory
candidate client root
effective project path
provider-managed project Git directory
```

Symlink aliases MUST NOT register one physical checkout twice.

Canonicalization MUST preserve enough original spelling for user-facing diagnostics without using that spelling as identity.

### 6.4 Linked worktrees

A linked Git worktree outside the physical `repo` project path MAY still map to a manifest project through its Git common directory.

The provider MUST distinguish:

```text
manifest project checkout
linked worktree of that checkout
unrelated repository with similar history
```

A linked worktree inherits project identity and workspace hints from the owning common directory, but its `HEAD`, index, draft, and active Git operation remain worktree-specific.

### 6.5 Probe output

Illustrative output:

```json
{
  "provider": "repo",
  "claim": "authoritative",
  "confidence": "high",
  "facts": {
    "workspace": {
      "workspace_id": "3b88ab85-9912-444e-9675-fbde4f577ddb",
      "kind": "repo-client",
      "root": "/work/studio-main"
    },
    "project": {
      "name": "tools/vendor/example",
      "path": "tools/vendor/example",
      "git_common_dir": "/work/studio-main/.repo/projects/tools/vendor/example.git"
    }
  },
  "hints": [],
  "requirements": [],
  "evidence": [
    "valid effective manifest",
    "current Git common directory maps uniquely to one manifest project"
  ]
}
```

Facts, hints, inferences, and unmet requirements MUST be represented separately.

### 6.6 Conflicting candidates

If two specialized workspace candidates both prove ownership and neither is strictly more specific, automatic selection MUST fail.

Diagnostics MUST show:

```text
provider
candidate root
project mapping
claim strength
evidence
suggested --workspace or --workspace-provider selector
```

The provider MUST NOT choose by lexical path order, most recently used workspace, or remote naming convention.

---

## 7. Obtaining effective project metadata

### 7.1 Supported interfaces first

The provider SHOULD prefer supported, machine-readable `repo` interfaces supplied by the installed `repo` version.

When no suitable structured interface exists, the provider MAY use a controlled `repo forall` invocation for the current project.

Relevant environment fields commonly include:

```text
REPO_PROJECT
REPO_PATH
REPO_REMOTE
REPO_LREV
REPO_RREV
REPO_UPSTREAM
REPO_DEST_BRANCH
REPO_PROJECT_FETCH_URL
```

The provider MUST version-gate assumptions about which fields exist and what they mean.

### 7.2 Controlled `repo forall`

A controlled `repo forall` invocation MUST:

* Use a fixed provider-authored command body.
* Target only the current project where supported.
* Treat values as environment data, not as shell source.
* Avoid interpolating repository names, paths, URLs, or revisions into executable text.
* Emit NUL-delimited or equivalently unambiguous records.
* Enforce a timeout and output bound.
* Capture standard error for diagnostics.
* Avoid unrelated hooks or project commands.
* Fail closed when output is malformed or incomplete.

Because `repo forall -c` may use a shell, the fixed command body MUST contain no repository-controlled fragments.

### 7.3 Manifest parsing fallback

Direct manifest parsing MAY be used only as an explicit, versioned fallback.

It MUST account for applicable semantics, including:

* `<default>` inheritance.
* Per-project overrides.
* Remote overrides.
* `<extend-project>`.
* Local manifests.
* Includes.
* Submanifests.
* Project path changes.
* Revision-locked manifests.
* Duplicate project names at different paths.
* Effective destination-branch inheritance.

The parser MUST NOT assume that the top-level manifest file alone is effective state.

### 7.4 Missing executable

If the provider implementation needs the `repo` executable and it is unavailable:

* Passive probe MAY use independently validated local metadata when supported.
* Otherwise the candidate becomes degraded or inapplicable.
* Cached facts MUST NOT be treated as current without validation.
* The core MAY continue in `core.git` mode for repository-local operations.

### 7.5 Malformed metadata

Malformed, truncated, internally inconsistent, or unsupported metadata MUST produce a degraded result.

The provider MUST NOT guess project identity or integration context from directory layout alone.

---

## 8. Project mapping

### 8.1 Provider project identity

The provider project identity is the effective manifest project name.

Example:

```text
tools/vendor/google/alt-lang/soong
```

It is distinct from:

```text
filesystem path
Git remote name
fetch URL
review endpoint
Gerrit numeric change number
Staircase lineage ID
```

### 8.2 Project path

The provider records the effective project path relative to the applicable client root.

Project name and project path MAY differ. Both MUST be preserved.

### 8.3 Duplicate project checkouts

A manifest may contain:

```text
name = platform/common, path = device/a/common
name = platform/common, path = device/b/common
```

The provider MUST distinguish them by at least:

```text
workspace ID
project name
project path
Git common directory
```

Review hints MAY use the common project name, but local operations remain scoped to the selected checkout.

### 8.4 Nested Git repositories

An unmanifested nested Git repository inside a manifest project is not automatically the outer project.

The provider MUST map the current Git common directory, not merely an enclosing filesystem path. If no effective project entry owns that common directory, `project-mapping=repo` is inapplicable for that repository.

### 8.5 Project relocation

When local manifest changes move a project path while preserving manifest project identity and Git common-directory continuity, the mapping MAY update in place.

When the old and new checkout coexist, they are separate checkout identities even if they share project name and object content.

### 8.6 Project removed from the manifest

If the current repository ceases to be declared:

* Existing managed staircases remain valid local Git and Staircase objects.
* The project mapping becomes degraded.
* The checkout MUST NOT be reassigned to a similarly named project.
* Manifest-derived integration and review hints become stale.
* Repository-local commands MAY continue using `core.git` semantics.

---

## 9. Typed workspace and manifest outputs

### 9.1 Output categories

The provider MAY emit all of the following independently:

```text
workspace identity and root
project identity and path
exact current checkout evidence
manifest-declared revision locator
translated local revision locator and resolved OID
upstream locator and resolved OID
symbolic integration-target candidate
manifest remote name
fetch URL
review endpoint hint
review destination hint
transport hint
```

The provider MUST NOT collapse these into one branch, target, remote, or destination field.

### 9.2 Provenance

Each emitted value MUST include provenance sufficient for diagnostics and revalidation.

Illustrative shape:

```json
{
  "kind": "translated-manifest-revision",
  "locator": "refs/remotes/m/studio-main",
  "resolved_oid": "ba19eb864d...",
  "source": "REPO_LREV",
  "exact": false,
  "moving": true
}
```

A resolved moving ref has an exact observed OID, but the locator remains moving intent.

### 9.3 Manifest revision

The provider returns the manifest revision exactly as declared, commonly from:

```text
REPO_RREV
```

It may be:

* A plain branch name.
* A full refname.
* A tag.
* A full commit OID.
* Another supported revision expression.

The provider MUST preserve the raw locator and MUST NOT assume it is a branch.

### 9.4 Translated local revision

When available, the provider returns the locally usable translated revision, commonly from:

```text
REPO_LREV
```

The core MUST resolve it to a full commit OID before use.

The provider MUST preserve both:

```text
symbolic locator
observed exact OID
```

### 9.5 Upstream ref

When present, the provider returns the effective upstream locator, commonly from:

```text
REPO_UPSTREAM
```

An upstream locator is not automatically:

* The exact workspace checkout.
* The integration anchor.
* The review destination.
* The local branch upstream.
* The landing destination.

### 9.6 Manifest remote and fetch URL

The provider MAY return:

```text
effective manifest remote name
resolved fetch URL
known push URL
```

These are routing hints. They do not prove write permission or select a review provider.

The provider MUST NOT substitute a remote named `origin` when the effective manifest remote is different.

---

## 10. Current checkout evidence

### 10.1 Exact `HEAD`

When `HEAD` resolves to a commit, the provider MAY return its full OID as exact current checkout evidence.

Illustrative output:

```json
{
  "current_checkout": {
    "oid": "ba19eb864d...",
    "head_state": "detached",
    "source": "project-head",
    "active_git_operation": null
  }
}
```

The literal expression `HEAD` MUST NOT be persisted as identity or moving intent.

### 10.2 Detached synchronized checkout

A detached project `HEAD` is strong integration-anchor evidence when:

* No transient Git operation owns the worktree.
* The commit is compatible with the effective manifest candidates.
* It is not clearly one of the selected staircase cuts or aggregate top.
* The graph does not show unaccounted local work below the staircase.

The core performs final eligibility and selection under the consolidated specification.

### 10.3 Attached development branch

When the worktree is attached to a local development branch:

* The branch tip MUST NOT be reported as the workspace integration anchor merely because it is checked out.
* The provider still reports manifest-derived candidates.
* Branch upstream configuration remains a separate core evidence source.
* The current branch MAY be a staircase cut or aggregate top.

### 10.4 Detached local commit

A detached `HEAD` may contain local work above the manifest-selected history.

When the provider detects that current `HEAD` is ahead of or divergent from a manifest candidate:

* It MUST label `HEAD` as current checkout evidence, not certified baseline.
* It MUST report the graph relationship.
* It MUST preserve the manifest-derived candidate separately.
* It MUST NOT silently absorb the local commit into the integration set.

### 10.5 Detached review checkout

A downloaded Gerrit patch set or arbitrary historical checkout may also be detached.

The provider MUST return both current-checkout and manifest-derived evidence when both exist.

If current `HEAD` is also a staircase cut, review revision, or incompatible historical commit, the core MUST require stronger context or explicit `--onto` rather than treat it as integrated history.

### 10.6 Active Git operation

During rebase, merge, cherry-pick, revert, bisect, sequencer replay, or another active Git operation:

* Current `HEAD` and index may be transient.
* The provider MAY report them as worktree observations.
* It MUST mark them ineligible for automatic workspace-checkout anchoring unless the consolidated core explicitly supports that operation state.
* Staircase mutation MUST respect the external operation owner.

---

## 11. Integration-context capability

### 11.1 Provider returns candidates, not final truth

The `repo` provider supplies typed evidence to the core. It does not unilaterally select the integration context for every staircase.

The final core context may differ by staircase because candidate compatibility is graph-dependent.

`git staircase list` MUST remain best-effort per candidate. One unresolved project-wide anchor MUST NOT suppress otherwise resolvable staircases.

### 11.2 Candidate classes

The provider MAY return:

```text
exact manifest OID candidate
eligible detached checkout candidate
translated local revision candidate
declared manifest revision candidate
upstream candidate
```

Each candidate MUST identify:

```text
full observed OID when resolvable
symbolic locator when applicable
exact versus moving semantics
source and provenance
resolution time or fingerprint
graph relation to current checkout when known
confidence and eligibility notes
```

### 11.3 Candidate authority

Within the provider's evidence, the following rules apply:

1. An exact manifest revision OID is authoritative manifest intent.
2. An eligible detached checkout OID is exact workspace-state evidence and SHOULD be preferred as the current integration anchor when a moving manifest revision has advanced after checkout.
3. A translated local revision is preferred over attempting to reinterpret an unresolved raw manifest expression for local Git operations.
4. A raw manifest revision remains valuable symbolic intent even when it cannot be resolved locally.
5. An upstream ref remains a distinct locator and MUST NOT overwrite an exact manifest OID or eligible exact checkout.

Equivalent candidates resolving to the same OID collapse while retaining all provenance.

### 11.4 Revision-locked manifest

When the manifest revision is an exact commit OID:

* The exact OID MUST remain exact.
* An upstream ref MUST remain separate moving intent.
* The provider MUST NOT replace the pinned OID with the current upstream tip.
* If the checkout differs, the discrepancy MUST be reported.

### 11.5 Moving manifest advanced after checkout

When a translated manifest ref now resolves beyond the detached checkout:

```text
integration-anchor candidate:
  exact detached checkout OID

symbolic integration-target candidate:
  translated manifest ref

observed symbolic-target OID:
  newer commit
```

The exact checkout remains reproducible. The moving target indicates that the workspace may be behind.

### 11.6 Unresolved history

An `integration-context=repo` binding MAY remain valid while one project's candidate is unresolved.

Commands requiring an anchor MUST fail narrowly with:

```text
integration-context-unresolved
```

and show the candidate locators, resolution failures, and explicit remedies such as:

```console
git staircase <command> <selector> --onto <commit-ish>
git staircase workspace refresh
git staircase --workspace-mode=single-git <command> ...
```

### 11.7 Explicit override

An explicit command argument such as `--onto` outranks provider evidence for that invocation.

The provider MUST still expose the manifest context in diagnostics. It MUST NOT rewrite the manifest or persistent workspace binding merely because one command used an override.

### 11.8 No global synthetic merge base

The provider MUST NOT manufacture one project-wide integration anchor by merging or intersecting incompatible candidate histories.

A multi-anchor integration set requires the explicit core policy defined by the consolidated specification. Automatic `repo` discovery does not invent one.

---

## 12. Review, verification, and transport hints

### 12.1 Hint contract

A workspace hint contains:

```text
kind
value or locator
source
scope
confidence
normalization status
freshness fingerprint
```

Hints are untrusted provider input to another capability probe. The receiving provider MUST validate them independently.

### 12.2 Gerrit endpoint hint

An effective manifest remote may specify a Gerrit review locator used by `repo upload`.

The provider MAY emit:

```json
{
  "kind": "review-endpoint",
  "provider_hint": "gerrit",
  "locator": "review.example.com",
  "source": "manifest-remote-review",
  "scope": {
    "project": "platform/payments"
  }
}
```

The `repo` provider does not:

* Authenticate to the endpoint.
* Confirm reachability.
* Confirm that it is Gerrit.
* Select an account.
* Query server identity.
* Become the review provider.

### 12.3 Project hint for Gerrit

The effective manifest project name SHOULD be supplied as the Gerrit project-identity hint.

Project path MUST remain separate. The path is not substituted merely because it resembles the project name.

### 12.4 Destination-branch hint

Destination precedence inside `repo` metadata is:

1. Effective project `dest-branch`.
2. Effective inherited `dest-branch`.
3. Manifest revision fallback only when it is provably a branch locator that can be losslessly normalized.
4. Unresolved.

The provider MUST NOT emit an exact commit OID, tag, or arbitrary revision expression as a Gerrit destination branch.

Accepted normalizable examples include:

```text
main                 -> refs/heads/main
refs/heads/main      -> refs/heads/main
```

A remote-tracking ref MAY be normalized only when effective manifest semantics prove the corresponding branch name. It MUST NOT be stripped by string convention alone.

Illustrative hint:

```json
{
  "kind": "review-destination",
  "provider_hint": "gerrit",
  "ref": "refs/heads/main",
  "raw_value": "main",
  "source": "project-dest-branch",
  "scope": {
    "project": "platform/payments"
  }
}
```

### 12.5 Incomplete Gerrit route

The provider MAY emit a partial route:

```text
endpoint known
project known
destination unresolved
```

The Gerrit provider may bind as applicable but route-incomplete. Local Change-Id inspection remains possible. Network publication fails narrowly until the route is completed.

### 12.6 Transport hints

The provider MAY emit:

```text
manifest remote name
fetch URL
push URL when explicitly known
project name
review endpoint locator
```

These values help a routing or review-transport provider plan. They do not prove write permission and do not authorize `repo upload` or `git push`.

### 12.7 Verification hints

A manifest or workspace profile MAY identify a verification provider or presubmit profile by explicit trusted configuration.

A review endpoint alone is insufficient to bind verification. The Gerrit provider MUST independently declare and validate its verification capability.

---

## 13. Automatic binding and composition

### 13.1 Workspace binding

`workspace=repo` MAY be auto-bound when:

* The provider is trusted.
* Passive probe succeeds with high confidence.
* The current Git repository maps uniquely to one effective manifest project.
* No stronger explicit or profile workspace binding conflicts.

### 13.2 Project mapping binding

`project-mapping=repo` MAY be bound with the workspace capability when project identity and checkout identity are known.

A workspace binding may remain usable for other projects even if one project mapping becomes degraded.

### 13.3 Integration-context binding

`integration-context=repo` MAY be auto-bound when the provider can emit at least one valid candidate class or an actionable unresolved locator.

Binding means the provider is applicable. It does not mean every command can resolve one exact anchor.

### 13.4 Workspace-hints binding

`workspace-hints=repo` MAY be auto-bound whenever effective manifest metadata can be read safely.

This binding may produce zero review hints for projects whose manifest remote has no review metadata.

### 13.5 Gerrit composition

When `repo` emits all of:

```text
Gerrit-shaped endpoint hint
manifest project identity
normalized destination branch hint
```

an installed trusted Gerrit provider MAY be passively probed.

The `repo` provider MUST NOT directly set:

```text
review = gerrit
verification = gerrit
review-transport = gerrit
```

The Gerrit provider independently accepts, rejects, or degrades the route.

### 13.6 Binding provenance

Automatically created bindings use provenance:

```text
auto-discovered
```

Explicit and profile bindings MUST NOT be replaced silently.

A changed manifest MAY invalidate an auto-discovered binding, but it MUST NOT silently replace an explicit Gerrit project or destination.

### 13.7 No network during composition

Initial `repo` and Gerrit applicability composition remains offline.

The first network-requiring review command may validate server identity, authentication, project existence, destination, and upload endpoints under the Gerrit provider contract.

---

## 14. Bootstrap behavior

### 14.1 First command

Every Staircase command runs the consolidated bootstrap protocol.

For a first command inside a valid `repo` project, bootstrap may:

1. Identify the `repo` client.
2. Map the current Git checkout to a manifest project.
3. Bind `workspace`, `project-mapping`, `integration-context`, and `workspace-hints` independently.
4. Pass Gerrit-shaped hints to the Gerrit provider.
5. Persist eligible user-local bindings.
6. Continue the original command.

### 14.2 Human announcement

Human mode MAY print one configuration announcement to standard error when persistent automatic binding changes.

It MUST NOT contaminate porcelain or JSON standard output.

### 14.3 No-configure mode

With:

```console
git staircase --no-configure <command>
```

The provider MAY probe and bind invocation-locally but MUST persist no workspace record or binding changes.

### 14.4 No-bootstrap mode

With:

```console
git staircase --no-bootstrap <command>
```

The command uses only existing valid bindings and explicit arguments. It MUST NOT invoke the `repo` probe.

### 14.5 Single-Git mode

With:

```console
git staircase --workspace-mode=single-git <command>
```

The enclosing `repo` client is ignored for that invocation. No persistent `repo` binding is removed.

---

## 15. Manifest and workspace revalidation

### 15.1 Discovery fingerprint

The provider fingerprint SHOULD cover, directly or by stable digest:

```text
canonical client-root locator
effective manifest identity
manifest source and branch or revision identity
local manifest set
submanifest ownership
current project name and path
Git common directory
effective remote
effective revision
effective translated revision
effective upstream
effective destination branch
effective review endpoint
provider version
repo metadata-interface version
```

Large manifest content need not be copied into the workspace record.

### 15.2 Revalidation triggers

Revalidation is required when relevant evidence changes, including:

* `repo init` changes manifest source, branch, or manifest file.
* `repo sync` changes project checkout state or effective project mapping.
* Local manifests change.
* Includes or submanifests change.
* Project path or ownership changes.
* The current Git common directory changes.
* Effective remote, revision, upstream, destination, or review endpoint changes.
* The workspace moves.
* The `repo` or provider version changes materially.

### 15.3 Workspace refresh

```console
git staircase workspace refresh
```

is observational. It MAY:

* Re-run passive validation.
* Re-read effective manifest metadata.
* Re-resolve local candidate refs.
* Update user-local fingerprints and auto-discovered bindings.
* Report changed exact checkout and moving symbolic target OIDs.
* Mark cached hints or provider routes stale.

It MUST NOT run `repo sync`, rewrite staircase commits, move staircase refs, or upload reviews.

### 15.4 Effects on staircases

A changed workspace integration candidate does not silently rewrite a staircase.

After refresh, the core MAY report:

```text
behind integration anchor
integration context changed
review destination hint changed
provider route stale
verification stale
```

The user then selects an explicit Staircase operation such as:

```console
git staircase rebase <selector>
git staircase review reconcile <selector>
git staircase workspace doctor
```

### 15.5 Workspace moved

If continuity is proven:

* The workspace ID remains stable.
* The client-root locator changes.
* Path-dependent project mappings and fingerprints refresh.

If continuity cannot be proven, the provider MUST NOT merge records merely because project names and revisions match.

---

## 16. Interaction with `repo sync` and external Git operations

### 16.1 No implicit sync

The provider MUST never automatically run:

```text
repo init
repo sync
repo start
repo checkout
repo upload
repo abandon
repo prune
```

### 16.2 `repo sync` is externally owned

When `repo sync` starts a Git rebase, merge, checkout, or other sequencer operation in a project worktree, that operation is externally owned.

Staircase MUST NOT:

* Call `git staircase continue` for it.
* Call `git staircase abort` for it.
* Reinterpret its conflict state as a Staircase draft.
* Start an unrelated structural mutation in the affected worktree.

`git staircase status` SHOULD report the active Git operation and recommend the ordinary Git or workspace-manager continuation commands.

### 16.3 After sync

After the external operation completes, `workspace refresh` re-observes:

```text
current checkout OID
translated manifest revision
manifest revision
review hints
project mapping
```

Existing staircase cuts remain unchanged until an explicit rebase, restack, or reconcile operation moves them.

### 16.4 Sync removed or replaced a project

If sync removes, replaces, or remaps the current project:

* Existing Staircase refs and objects remain local where reachable.
* The old project mapping becomes stale or unavailable.
* No new project inherits the old staircase merely because it occupies the same path.
* Review associations retain their historical provider route until explicitly reconciled.

### 16.5 Multi-project sync

A workspace-wide `repo sync` may change several projects, but one repository-local staircase remains scoped to one Git repository.

A future workspace aggregate MAY coordinate refresh and rebase plans across projects. This addendum does not define such an aggregate.

---

## 17. Commands and diagnostics

### 17.1 Workspace commands

The consolidated workspace commands apply:

```console
git staircase workspace show
git staircase workspace discover
git staircase workspace providers
git staircase workspace refresh
git staircase workspace doctor
git staircase workspace configure
git staircase workspace forget
```

### 17.2 Provider doctor

```console
git staircase provider repo doctor
```

SHOULD report:

```text
provider applicability
workspace binding and provenance
client root
workspace ID
effective manifest status
current project identity and path
Git common directory
current checkout evidence
integration candidates and graph relationships
review and destination hints
fingerprint freshness
repo executable or metadata-interface availability
rejected workspace candidates
security or parsing failures
```

It MUST expose no credentials.

### 17.3 Provider project listing

```console
git staircase provider repo list-projects
```

MUST print the list of all project paths defined in the effective manifest, relative to the workspace root, one per line.

This list is used by monorepo worktree tools to reconstruct the workspace structure.

### 17.4 Workspace show

`workspace show` SHOULD distinguish:

```text
workspace checkout anchor
integration anchor selected by the core
symbolic integration target
review destination hint
bound review destination
current HEAD
```

It MUST NOT print one ambiguous `target` field.

### 17.5 Diagnostics vocabulary

Diagnostics SHOULD distinguish at least:

```text
repo provider not applicable
repo client found but project not mapped
several project mappings match
several specialized workspaces claim the checkout
effective manifest unavailable
repo executable unavailable
metadata interface unsupported
manifest parser fallback disabled
project removed from manifest
current checkout differs from exact manifest revision
detached checkout appears to contain local work
detached checkout appears to be a review revision
translated revision unresolved
integration context unresolved
review endpoint hint unavailable
review destination hint unavailable
review destination is not branch-shaped
Gerrit route incomplete
workspace fingerprint stale
external Git operation active
```

### 17.6 Stable core errors

Provider failures SHOULD appear beneath stable core codes such as:

```text
integration-context-unresolved
provider-unbound
provider-route-incomplete
active-git-operation
selection-changed
```

Provider-specific detail MAY use namespaced codes such as:

```text
repo.project-not-mapped
repo.manifest-invalid
repo.revision-unresolved
repo.destination-not-branch
```

### 17.7 Machine output

Machine-readable output MUST:

* Use full typed OIDs.
* Preserve facts, hints, and requirements separately.
* Keep standard output free of bootstrap prose.
* Use stable field names and versioned schemas.
* Escape manifest and path data safely.

---

## 18. Security and trust

### 18.1 Provider-owned metadata

Files under `.repo` and manifest content are untrusted data owned by another tool.

The provider MUST parse or query them without executing them.

### 18.2 Paths and URLs

Project names, project paths, remote names, revision strings, and URLs MUST be:

* Length-bounded.
* Parsed structurally where applicable.
* Passed as process arguments rather than shell fragments.
* Prevented from injecting command-line options.
* Escaped before terminal rendering.

### 18.3 Fixed command execution

When invoking `repo`, the provider MUST supply fixed arguments and fixed command bodies.

Repository or manifest values MUST NOT become executable shell syntax.

### 18.4 No credential capture

The provider MUST NOT read, copy, cache, or display credentials while discovering review or transport hints.

Authentication belongs to the receiving review or transport provider during an explicit network operation.

### 18.5 No bootstrap mutation

Passive discovery MUST NOT:

* Fetch.
* Sync.
* Checkout.
* Start or abandon branches.
* Upload.
* Install hooks.
* Modify Git config.
* Modify refs.
* Modify manifests.
* Modify `.repo`.

### 18.6 Cached evidence

Cached provider facts are observations. They MUST be invalidated or marked stale when their fingerprint changes.

Stale cache MUST NOT silently select a new integration anchor, project mapping, or review destination.

---

## 19. Corner cases

### 19.1 Revision-locked manifest with upstream

Given:

```text
revision = 1111111111111111111111111111111111111111
upstream = refs/heads/main
```

The provider returns:

```text
exact manifest candidate: 1111111...
symbolic upstream locator: refs/heads/main
```

The branch tip MUST NOT replace the exact manifest commit.

### 19.2 Manifest branch advanced after checkout

Given:

```text
detached HEAD: a100000
REPO_LREV: refs/remotes/m/main -> a300000
```

The provider returns both. The core may use `a100000` as exact current integration anchor and retain `refs/remotes/m/main` as symbolic integration target.

### 19.3 Local commit while detached

Given:

```text
manifest candidate: a100000
HEAD: e200000, descendant of a100000
```

The provider reports `e200000` as current checkout evidence and local-ahead discrepancy. It MUST NOT certify `e200000` as integrated history without stronger core evidence.

### 19.4 Attached branch with no upstream

The provider uses manifest-derived candidates. It MUST NOT fail merely because the current branch has no upstream and MUST NOT use its tip as the baseline.

### 19.5 Missing destination branch

If `dest-branch` is absent and `revision` is an exact OID or tag, the review destination remains unresolved. The provider MUST NOT invent `main`.

### 19.6 Multiple remotes

The provider uses the effective manifest remote and preserves other Git remotes as separate repository-local evidence. It MUST NOT prefer `origin` by name.

### 19.7 Duplicate project name

Two entries with the same project name at different paths remain distinct local checkout identities. A command in one path MUST NOT mutate refs in the other checkout.

### 19.8 Linked worktree outside the client root

A linked worktree may inherit workspace and project mapping through the common directory. Its current checkout evidence and active Git operation remain specific to that linked worktree.

### 19.9 Corrupt or incomplete manifest

The provider reports a degraded candidate and actionable diagnostics. It MUST NOT infer effective values by reading only directory names or remote names.

### 19.10 Bare or mirror client

A mirror client has no ordinary project worktree.

The provider MAY identify the workspace, but project-checkout integration evidence and worktree-local Staircase discovery are unavailable. Commands requiring a normal worktree fail narrowly.

### 19.11 Local manifest changes Gerrit route

A changed review endpoint or destination invalidates inherited Gerrit route evidence. Existing review associations remain historical and MUST NOT be silently retargeted.

### 19.12 Same path, different project after sync

If a manifest replaces project `A` with project `B` at the same path, the provider MUST treat this as a project-identity change. Existing Staircase lineages are not transferred automatically.

### 19.13 Unborn or empty project repository

If `HEAD` does not resolve to a commit, the provider may still map the project and provide manifest hints. It cannot supply an exact checkout anchor until a commit exists.

### 19.14 Partial clone or missing objects

A candidate ref may resolve syntactically while required commit objects are missing locally.

The provider reports the missing-object requirement. Passive bootstrap MUST NOT fetch automatically.

### 19.15 Object-format independence

The provider MUST accept the repository's actual object format and MUST NOT assume SHA-1 length. All persisted and machine-readable OIDs are full and typed.

---

## 20. Normative invariants

### 20.1 `repo` is a workspace provider

It does not thereby become a review, verification, or landing provider.

### 20.2 Project identity and path are separate

Neither silently replaces the other.

### 20.3 Checkout state and manifest intent are separate

Exact current checkout, manifest revision, translated revision, and upstream remain separately typed.

### 20.4 Detached `HEAD` is first-class evidence

It does not require a local branch name, but it is not automatically certified as integrated history.

### 20.5 Attached branch tip is not the baseline

The provider does not treat checked-out development work as workspace integration state.

### 20.6 Exact manifest OIDs remain exact

A moving upstream or translated ref does not replace them.

### 20.7 Review destination must be branch-shaped

An OID, tag, or arbitrary revision expression is not emitted as a Gerrit destination branch.

### 20.8 Review hints remain hints

The Gerrit provider independently validates and binds review semantics.

### 20.9 No `origin` assumption

The effective manifest remote governs manifest-derived routing evidence.

### 20.10 Effective manifest semantics are required

A shallow parse of one XML file is nonconforming.

### 20.11 Passive discovery is offline and nonmutating

No sync, fetch, upload, authentication, hook, or manifest mutation occurs.

### 20.12 `repo sync` operations retain external ownership

Staircase does not continue, abort, or absorb their conflict state automatically.

### 20.13 Workspace refresh is observational

It updates evidence and bindings but does not rewrite staircase history.

### 20.14 Project replacement does not transfer lineage

A new project at the same path does not inherit the old project's staircases or review route.

### 20.15 User-local workspace identity is not path identity

A client may move without changing workspace ID when continuity is proven.

### 20.16 Facts, hints, and requirements remain distinct

Provider output does not smuggle an inference into an authoritative fact.

---

## 21. Summary

The `repo` provider translates a multi-repository client into generic Staircase context:

```text
workspace identity and root
project identity and path
Git common-directory mapping
exact current checkout evidence
manifest revision
translated local revision
upstream locator
manifest remote and fetch hints
review endpoint hint
review destination hint
```

It does not perform review operations, synchronization, or landing.

The governing rule is:

> The `repo` provider explains which workspace and project contain the work, what exact checkout is present, and what the manifest intends. It does not decide what Gerrit means, and it does not rewrite the workspace to make its evidence convenient.

---

# Appendix A: User journeys

## A.1 Conventions used in the transcripts

This appendix is normative for command names, provider boundaries, external-operation ownership, and the material facts that output must communicate.

Concrete paths, OIDs, UUIDs, and review numbers are illustrative. Human output may differ in spacing and abbreviation, but it MUST preserve every material distinction shown. Machine output remains governed by the consolidated schemas and MUST use full typed OIDs.

Lines beginning with `$` are commands. Commands beginning with `repo` or plain `git` are explicitly ordinary workspace-manager or Git commands. All other relevant commands use `git staircase`.

One-time bootstrap announcements are labeled `[stderr]`. They MUST NOT appear on machine-readable standard output.

---

## A.2 Journey 1: First use in a detached `repo` checkout with Gerrit hints

### A.2.1 Starting state

The developer is in project `platform/payments` inside `/work/android`. The project is detached at the commit selected by the last sync:

```text
HEAD: a100000
REPO_PROJECT: platform/payments
REPO_PATH: platform/payments
REPO_LREV: refs/remotes/m/main
REPO_RREV: refs/heads/main
REPO_DEST_BRANCH: main
manifest review endpoint: review.example.com
```

Three local branches form a sequential implicit staircase:

```text
payments-1 -> b110000
payments-2 -> c120000
payments   -> d130000
```

The first command performs passive bootstrap and continues listing:

```console
$ git staircase list
[stderr] Configured Staircase workspace:
[stderr]   workspace:            repo
[stderr]   project:              platform/payments
[stderr]   integration-context:  repo
[stderr]   workspace-hints:      repo
[stderr]   review:               gerrit
[stderr]   verification:         gerrit
[stderr]
payments  3 steps  clean  sequential  (implicit)
```

The workspace provider's facts remain distinct:

```console
$ git staircase workspace show
workspace
  ID: 3b88ab85-9912-444e-9675-fbde4f577ddb
  provider: repo
  root: /work/android

current project
  name: platform/payments
  path: platform/payments
  Git common directory: /work/android/.repo/projects/platform/payments.git

checkout and integration
  current HEAD: a100000  detached
  workspace checkout anchor: a100000
  selected integration anchor: a100000
  symbolic integration target: refs/remotes/m/main
  observed symbolic-target OID: a100000

review hints
  provider hint: gerrit
  endpoint: review.example.com
  project: platform/payments
  destination: refs/heads/main

bound review route
  provider: gerrit
  validation: local evidence only
```

No network request occurred. The Gerrit provider independently accepted the three local hints.

The developer inspects the provider itself:

```console
$ git staircase provider repo doctor
repo provider: ready
  workspace claim: authoritative
  binding provenance: auto-discovered
  effective manifest: valid
  project mapping: unique
  current checkout: a100000 detached, eligible exact anchor
  manifest revision: refs/heads/main
  translated revision: refs/remotes/m/main -> a100000
  upstream: none
  review endpoint hint: review.example.com
  review destination hint: refs/heads/main
  network used: no
  workspace mutated: no
```

The review plan composes both providers without fusing them:

```console
$ git staircase review plan payments --mapping per-commit
Review publication plan for 'payments'
  workspace provider: repo
  review provider: gerrit
  project: platform/payments
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a100000
  mapping: one review per commit

  step  commit    action
  1     b110000   create
  2     c120000   create
  3     d130000   create

No remote mutation performed.
```

---

## A.3 Journey 2: A moving manifest branch advances after the workspace checkout

The developer has not synchronized recently. The checkout remains detached at `a100000`, but another local operation updated the manifest tracking ref to `a300000`:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  current checkout: a100000  detached
  translated manifest revision: refs/remotes/m/main -> a300000

Integration evidence:
  exact workspace checkout candidate: a100000
  symbolic integration target: refs/remotes/m/main
  observed symbolic-target OID: a300000
  relationship: checkout is ancestor of symbolic target by 27 commits

No Git commits or Staircase refs were changed.
```

The existing staircase remains based on the exact checkout until explicitly rebased:

```console
$ git staircase status payments
payments (implicit)
  integration anchor: a100000
  symbolic integration target: refs/remotes/m/main -> a300000
  state: clean
  workspace relation: behind symbolic integration target
  verification: stale
```

A dry run shows the exact proposed rewrite:

```console
$ git staircase rebase payments --dry-run
Rebase plan for implicit staircase 'payments'
  old integration anchor: a100000
  new integration anchor: a300000
  symbolic integration target: refs/remotes/m/main
  steps to replay: 3
  primary branches to update: 3
  adoption required: no, if the final sequential layout is published cleanly

No mutation performed.
```

The provider did not silently replace `a100000` with a moving ref when reading the existing structure.

---

## A.4 Journey 3: Resolve a `repo sync` conflict without surrendering operation ownership

The developer invokes the workspace manager directly:

```console
$ repo sync platform/payments
error: could not apply e400000... Preserve local test fixture
CONFLICT (content): Merge conflict in testdata/accounts.json
```

The active rebase belongs to `repo sync`, not Staircase:

```console
$ git staircase status payments
payments
  active Staircase operation: none

current worktree
  active Git operation: rebase
  owner: external
  likely initiator: repo sync
  conflicted paths:
    testdata/accounts.json

Staircase mutation is blocked in this worktree.
Continue or abort the external operation with ordinary Git:
  git add <resolved-paths>
  git rebase --continue
  git rebase --abort
```

A Staircase rewrite refuses rather than stealing the sequencer:

```console
$ git staircase rebase payments
error [active-git-operation]: an external rebase owns this worktree
  provider detail: repo.external-sync-operation
  next: resolve or abort the ordinary Git rebase
```

The developer resolves it with ordinary Git and reruns sync:

```console
$ git add testdata/accounts.json
$ git rebase --continue
Successfully rebased and updated detached HEAD.
$ repo sync platform/payments
Fetching: 100% (1/1), done in 0.842s
```

Refresh observes the resulting workspace state but does not rewrite the staircase:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  previous workspace checkout anchor: a100000
  current workspace checkout anchor: a500000
  symbolic integration target: refs/remotes/m/main -> a500000

Affected active staircases:
  payments  behind integration anchor  verification stale

No Staircase refs were moved.
```

The developer then starts a Staircase-owned rebase:

```console
$ git staircase rebase payments
Rebasing 'payments'.
  old integration anchor: a100000
  new integration anchor: a500000

  step 1 of 3: applied -> b510000
  step 2 of 3: conflict
  conflicted paths:
    ledger/writer.cc

Operation: rebase 04eab708
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To restore the pre-operation staircase, run:
  git staircase abort
```

The distinction is now reversed: this conflict belongs to Staircase, so `git staircase continue` is correct.

---

## A.5 Journey 4: A revision-locked manifest preserves the pinned commit

The effective project metadata is:

```text
REPO_RREV: 1111111111111111111111111111111111111111
REPO_LREV: 1111111111111111111111111111111111111111
REPO_UPSTREAM: refs/heads/release
REPO_DEST_BRANCH: release
HEAD: 1111111111111111111111111111111111111111
```

The provider reports exact and moving values separately:

```console
$ git staircase workspace show
checkout and integration
  current HEAD: 1111111  detached
  exact manifest revision: 1111111
  selected integration anchor: 1111111
  upstream locator: refs/heads/release
  upstream observed OID: 3333333

review hint
  destination: refs/heads/release
  source: project-dest-branch
```

Refreshing after the upstream branch advances does not change the pinned anchor:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  exact manifest revision: 1111111  unchanged
  upstream locator: refs/heads/release
  previous upstream OID: 3333333
  current upstream OID: 4444444
  selected integration anchor: 1111111  unchanged

No Staircase refs were changed.
```

A rebase without an explicit override remains pinned:

```console
$ git staircase rebase release-fix --dry-run
Rebase plan for 'release-fix'
  current integration anchor: 1111111
  resolved provider anchor: 1111111
  result: no integration-anchor change

No mutation performed.
```

To move intentionally, the developer supplies an exact override:

```console
$ git staircase rebase release-fix --onto 4444444 --dry-run
Rebase plan for 'release-fix'
  old integration anchor: 1111111
  new rewrite destination: 4444444
  manifest remains pinned at: 1111111
  persistent workspace binding changed: no

No mutation performed.
```

---

## A.6 Journey 5: An attached development branch is not mistaken for the baseline

The project is attached to branch `feature`:

```text
refs/remotes/m/main -> a100000
feature -> d130000
```

The branch itself is the aggregate top of a three-step staircase.

```console
$ git staircase workspace show
current worktree
  HEAD: d130000
  branch: refs/heads/feature

manifest integration evidence
  translated revision: refs/remotes/m/main -> a100000
  selected integration anchor: a100000

checkout classification
  attached development branch tip is not a workspace anchor
```

Listing discovers the work correctly:

```console
$ git staircase list
feature  3 steps  clean  sequential  (implicit)
```

The aggregate diff is against `a100000`, not against the checked-out branch tip:

```console
$ git staircase show feature
feature (implicit)
  integration anchor: a100000
  top: d130000
  steps: 3
  current HEAD equals staircase top: yes
```

A missing branch upstream does not break the provider:

```console
$ git config --unset-all branch.feature.remote
$ git config --unset-all branch.feature.merge
$ git staircase list
feature  3 steps  clean  sequential  (implicit)
```

Manifest evidence remains available independently of branch configuration.

---

## A.7 Journey 6: Two checkouts of the same manifest project remain distinct

A local manifest contains:

```text
project name="platform/common" path="device/a/common"
project name="platform/common" path="device/b/common"
```

The developer runs diagnostics in the first checkout:

```console
$ cd /work/android/device/a/common
$ git staircase provider repo doctor
repo provider: ready
  workspace ID: 3b88ab85-9912-444e-9675-fbde4f577ddb
  project name: platform/common
  project path: device/a/common
  Git common directory: /work/android/.repo/projects/device/a/common.git
  local checkout identity: unique
```

The second checkout has the same project name but a different checkout identity:

```console
$ cd /work/android/device/b/common
$ git staircase provider repo doctor
repo provider: ready
  workspace ID: 3b88ab85-9912-444e-9675-fbde4f577ddb
  project name: platform/common
  project path: device/b/common
  Git common directory: /work/android/.repo/projects/device/b/common.git
  local checkout identity: unique
```

A staircase named `cleanup` in each checkout is repository-scoped and not ambiguous from either project directory:

```console
$ git staircase list
cleanup  2 steps  clean  sequential  (implicit)
```

A review plan uses the shared Gerrit project identity but the selected local checkout's exact commits:

```console
$ git staircase review plan cleanup --mapping per-commit
Review publication plan for 'cleanup'
  workspace project name: platform/common
  workspace project path: device/b/common
  review project: platform/common
  source repository: /work/android/.repo/projects/device/b/common.git
  destination: refs/heads/main

No remote mutation performed.
```

The provider MUST NOT inspect or move the branches in `device/a/common` as part of this plan.

---

## A.8 Journey 7: A local manifest changes the Gerrit destination

Initially the effective destination is `refs/heads/main`, and a managed staircase has existing Gerrit associations on that branch.

A local manifest is edited to set:

```text
dest-branch="release-q4"
```

The next refresh detects the changed hint:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  review destination hint:
    previous: refs/heads/main
    current: refs/heads/release-q4
    source: project-dest-branch

Affected provider routes:
  gerrit route is stale for 1 active staircase

Existing review associations were not retargeted.
```

Status preserves historical identity:

```console
$ git staircase review status payments
payments
  provider: gerrit
  associated destination: refs/heads/main
  current workspace hint: refs/heads/release-q4
  synchronization: current on associated destination
  provider route: stale because workspace destination hint changed
  action required: explicit review-route reconciliation
```

A new plan refuses to silently reuse the old Change-Ids on another destination:

```console
$ git staircase review plan payments
error [provider-route-incomplete]: workspace review destination changed
  existing review branch: refs/heads/main
  proposed workspace hint: refs/heads/release-q4
  Gerrit review identity includes destination branch

Choose an explicit destination or reconcile the existing associations.
```

The `repo` provider supplied the changed evidence. The Gerrit provider owns the review-identity consequence.

---

## A.9 Journey 8: A detached review checkout requires explicit integration context

The developer downloads a Gerrit patch set into detached `HEAD`:

```text
manifest candidate: a100000
HEAD: f900000
f900000 is not a descendant of a100000 in the locally available graph
```

Diagnostics retain both observations:

```console
$ git staircase provider repo doctor
repo provider: ready with warnings
  manifest integration candidate: a100000
  current checkout: f900000 detached
  checkout classification: incompatible with manifest candidate
  possible causes:
    downloaded review revision
    historical checkout
    missing local objects

Automatic checkout anchoring: rejected
```

A discovery request relying on current `HEAD` fails narrowly. Unresolved evidence is diagnostic and is not listed as a canonical staircase:

```console
$ git staircase discover --top f900000
error [integration-context-unresolved]: detached HEAD is not eligible as an automatic anchor
  current HEAD: f900000
  manifest candidate: a100000

Retry with an explicit integration context:
  git staircase discover --top f900000 --onto a100000
```

The developer supplies exact intent:

```console
$ git staircase discover --top f900000 --onto a100000
Discovered 1 canonical staircase.

review-fix  1 step  clean  (implicit)
  structural key: implicit@a71c9d2e
  integration anchor: a100000
  top: f900000
  selection source: explicit --top and --onto

No adoption performed.
```

The returned structural key is immediately usable:

```console
$ git staircase show --structural-key implicit@a71c9d2e
review-fix (implicit)
  integration anchor: a100000
  top: f900000
  steps: 1
```

The explicit invocation does not modify the workspace binding or manifest.

---

## A.10 Journey 9: `repo` is unavailable, so repository-local work degrades cleanly

The workspace was copied to a minimal environment without the `repo` launcher. Local Git metadata and staircase branches remain intact.

```console
$ git staircase provider repo doctor
repo provider: degraded
  enclosing .repo metadata: found
  repo executable: unavailable
  effective manifest validation: unavailable
  cached workspace fingerprint: stale
  project mapping: not current
  network used: no

Repository-local fallback is available.
```

A repository-local listing continues under `core.git` when its integration context can be resolved from explicit Git configuration:

```console
$ git staircase --workspace-mode=single-git list
feature  2 steps  clean  sequential  (implicit)
```

A Gerrit review plan that depended only on stale `repo` hints fails narrowly rather than guessing:

```console
$ git staircase review plan feature
error [provider-route-incomplete]: Gerrit route inherited from repo is stale
  project: last observed platform/example
  destination: last observed refs/heads/main
  endpoint: last observed review.example.com
  validation unavailable: repo provider could not validate effective manifest

No remote mutation performed.
```

The local staircase remains usable. Only the provider-dependent route is blocked.