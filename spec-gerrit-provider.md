# Addendum F: Gerrit Review and Verification Provider

## 1. Status and Scope

This addendum defines the Staircase Gerrit provider.

It depends on:

* **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**
* **Addendum E: `repo` Workspace Provider**, when used in a `repo` workspace

The Gerrit provider may implement:

```text
review
review-identity
verification
review-transport
```

It does not implement:

```text
workspace
project-mapping
generic integration-context
```

The provider may operate:

* In a standalone Git repository.
* In a `repo` workspace.
* In another workspace whose provider supplies compatible project and review hints.
* With explicit Gerrit configuration.

Gerrit accepts review uploads through Git pushes to the `refs/for/<branch>` namespace, and it also supports review operations through HTTP and SSH interfaces.

---

## 2. Provider Identity

The canonical provider name is:

```text
gerrit
```

Recommended descriptor:

```json
{
  "protocol_version": 1,
  "name": "gerrit",
  "capabilities": [
    "review",
    "review-identity",
    "verification",
    "review-transport"
  ],
  "probe": {
    "passive": true,
    "network": false,
    "mutates_workspace": false
  }
}
```

The passive probe establishes applicability only.

Network access occurs only when a command requests remote Gerrit information or performs upload.

---

## 3. Non-Goals

The Gerrit provider does not:

* Determine the workspace root.
* Parse a multi-repository manifest unless supplied through another provider.
* Treat detached `HEAD` as an integration target.
* Choose a review destination from `main`, `master`, or another conventional name.
* Assume that every commit equals one staircase step.
* Assume that one staircase step equals one Gerrit change.
* Install a `commit-msg` hook automatically during bootstrap.
* Contact a Gerrit server during passive provider discovery.
* Treat a Gerrit topic as staircase identity.
* Treat a numeric change number as globally unique.
* Treat “mergeable,” “verified,” and “submittable” as interchangeable.

---

## 4. Applicability Discovery

### 4.1 Strong local evidence

The provider may automatically bind when one unique high-confidence route is established from local evidence such as:

1. A workspace provider explicitly identifies a Gerrit review endpoint.
2. A workspace provider supplies a project name, destination branch, and Gerrit review host.
3. Explicit Staircase Gerrit configuration names the endpoint and project.
4. A configured Git review route explicitly targets `refs/for/*`.
5. Existing provider configuration is still valid.

A `repo` manifest remote may identify the Gerrit hostname used by `repo upload`, and a project or default `dest-branch` may identify the review destination.

---

### 4.2 Weak evidence

The following evidence is insufficient by itself for automatic binding:

* SSH port `29418`.
* A remote host name containing `gerrit` or `review`.
* Presence of a `commit-msg` hook.
* A Change-Id footer in one commit.
* A remote named `review`.
* A branch name resembling a Gerrit destination.
* Cached `refs/changes/*` objects with no known server.
* A web URL whose page has not been contacted.

Weak evidence may be displayed by diagnostics.

---

### 4.3 No network during probe

The passive probe must not:

* Query `/config/server/info`.
* Query changes.
* Run an SSH Gerrit command.
* Test credentials.
* Push a probe commit.
* Fetch `refs/changes/*`.

Network validation is deferred until a network-requiring command.

---

### 4.4 Multiple Gerrit endpoints

One workspace may contain projects served by different Gerrit instances.

The provider binding is therefore workspace-scoped, but its route is project-specific.

A route key contains:

```text
server identity
project identity
destination branch
transport endpoint
```

The provider must not force the entire workspace onto one server.

---

## 5. Gerrit Route

### 5.1 Route fields

A complete review route contains:

```text
provisional or confirmed server identity
project name
destination branch
Git upload endpoint
REST endpoint, when available
SSH endpoint, when available
authentication profile
```

Not every operation requires every field.

For example:

* Planning may need project and branch only.
* Upload needs a Git transport endpoint.
* Review status needs REST or SSH query access.
* Local Change-Id validation needs no network.

---

### 5.2 Project identity

Preferred project identity sources are:

1. Explicit configuration.
2. A workspace provider’s project identity.
3. A uniquely derivable remote repository path.
4. Remote query after network access is explicitly permitted.

When `repo` and Gerrit are used together, the manifest project name is the preferred Gerrit project identity. The `repo` manifest documentation notes that project names must match the name known to Gerrit when Gerrit is used for review.

---

### 5.3 Destination branch

Destination-branch precedence is:

1. Explicit command argument.
2. Managed staircase review policy.
3. Explicit project-specific Gerrit configuration.
4. Workspace-provider destination hint.
5. Unanimous branch review configuration.
6. Unresolved.

The provider must not invent:

```text
main
master
trunk
```

The normalized internal representation is:

```text
refs/heads/<branch>
```

---

### 5.4 Upload ref

For a normalized destination:

```text
refs/heads/main
```

the Gerrit review upload ref is conceptually:

```text
refs/for/main
```

or its full-target equivalent accepted by the server.

Gerrit uses the `refs/for/` namespace to distinguish review uploads from direct branch updates.

Push options are modeled separately from the branch name.

---

### 5.5 Endpoint aliases

SSH and HTTP endpoints may represent the same Gerrit installation.

They must not be merged solely because their hostnames resemble one another.

A provisional endpoint key is based on normalized local configuration.

A confirmed server identity may be established through:

* Explicit configuration.
* Authenticated server information.
* A provider-specific stable server identifier.
* User-approved alias configuration.

---

## 6. Review Identity

### 6.1 Change-Id

A Gerrit Change-Id is independent of the Git commit OID and is intended to associate revised commits with the same review across amendments and rebases.

A Change-Id alone is not globally unique.

Gerrit matches an existing review using at least:

```text
Change-Id
repository
branch
```

---

### 6.2 Canonical Staircase Gerrit identity

The canonical review identity is:

[
G =
(\text{server},\text{project},\text{branch},\text{Change-Id})
]

Machine form:

```json
{
  "provider": "gerrit",
  "server_id": "review.example.com",
  "project": "tools/example",
  "branch": "refs/heads/main",
  "change_id": "Iabc123..."
}
```

When known, the record may additionally contain:

```text
numeric change number
current patch-set number
current patch-set commit OID
change ref
web URL
status
```

These are attributes of the canonical identity, not replacements for it.

---

### 6.3 Numeric change numbers

A numeric Gerrit change number is scoped to one Gerrit server.

It must always be stored with server identity.

It is not sufficient as a portable review identity.

---

### 6.4 Change refs

A patch set may have a ref under:

```text
refs/changes/*
```

Gerrit documents these refs as server-side refs used to retrieve particular patch sets.

A change ref identifies one patch set, not the continuing review.

---

## 7. Mapping Staircases to Gerrit Changes

### 7.1 Default mapping is commit-oriented

Gerrit’s native review unit is a commit uploaded for review.

Therefore the default Staircase mapping is:

```text
one Gerrit change per review commit
```

A staircase step may contain:

* One review commit.
* Several review commits.
* No Gerrit review yet.

The provider must not silently claim that a multi-commit step corresponds to one Gerrit change.

---

### 7.2 Step review set

A managed step may store an ordered review set:

```text
step ID
    → Gerrit change identity 1
    → Gerrit change identity 2
    → ...
```

The ordering follows commit ancestry within the step.

---

### 7.3 One-change-per-step policy

A staircase may explicitly require:

```text
review mapping = one Gerrit change per step
```

Before upload, every active step must then be represented by exactly one review commit.

If a step contains several commits, the provider must:

* Refuse upload, or
* Invoke an explicit staircase normalization operation, or
* Present a dry-run rewrite plan.

It must not squash commits silently as a side effect of upload.

---

### 7.4 Aggregate-review policy

An aggregate single-change upload requires the entire active staircase to be materialized as one commit.

This is a separate explicit policy.

It changes decomposition and review identity and may require a history rewrite.

---

### 7.5 Merge commits

A review series containing merge commits requires explicit server and policy support.

The provider must not assume that a normal linear push plan applies.

---

## 8. Change-Id Validation

### 8.1 Footer parsing

The provider parses Change-Id lines using Git commit-message trailer semantics.

It must distinguish:

* No Change-Id.
* Exactly one valid Change-Id.
* Several Change-Id trailers.
* Malformed Change-Id.
* Change-Id outside the trailer block.

---

### 8.2 Missing Change-Id

A missing Change-Id is not always invalid because Gerrit installations may differ in policy. Gerrit commonly requires Change-Id footers, but repositories may be configured otherwise.

Behavior:

* Local inspection reports the missing identity.
* Upload planning marks identity continuity as unavailable.
* The provider does not invent a Change-Id during passive discovery.
* An explicit command may invoke an installed hook or rewrite plan.
* Server rejection remains possible.

---

### 8.3 Multiple Change-Ids

A commit with multiple Change-Id trailers is ambiguous.

The provider must not choose one silently.

Upload requires:

* Explicit correction, or
* A provider-supported disambiguation policy backed by known existing review records.

---

### 8.4 Rebase and amendment

When a commit is rewritten but its Change-Id remains unchanged, the Gerrit review identity may remain stable while the patch-set commit OID changes.

This aligns naturally with Staircase’s distinction between:

```text
stable step or review identity
exact commit OID
```

---

### 8.5 Split and join

A split or join may change the number of review commits.

The provider must not automatically duplicate one Change-Id across several resulting commits.

Recommended behavior:

* A surviving conceptual review commit retains its Change-Id.
* A newly created review commit receives no review identity until explicitly assigned.
* A removed review commit’s association is retired or marked superseded.
* Existing uploaded reviews are never silently reassigned to unrelated content.

---

## 9. Upload Planning

### 9.1 Local plan first

Before network mutation, the provider constructs a complete upload plan containing:

```text
server
project
destination branch
integration anchor
ordered commits
Change-Ids
existing known review identities
push ref
push options
expected mapping policy
```

---

### 9.2 Preconditions

The provider must detect:

* Missing destination.
* Missing transport endpoint.
* Ambiguous project identity.
* Duplicate Change-Ids in the upload set.
* Multiple Change-Ids in one commit.
* Commits already reachable from the destination.
* Commits outside the selected staircase body.
* Merge commits requiring special handling.
* Current staircase revision changing during planning.
* Review records targeting another project or branch.

---

### 9.3 Upload command

The provider may upload using:

* Direct `git push`.
* A configured Gerrit transport helper.
* A workspace-provided transport adapter.

The chosen transport must preserve Gerrit review semantics.

Using a `repo` workspace does not require delegating upload to `repo upload`, though a separately defined transport adapter may do so.

---

### 9.4 Push source

The pushed source must be an exact commit OID or protected ref resolved from the planned staircase revision.

The provider must not re-resolve a moving branch after the plan is approved without verifying that it still points to the expected OID.

---

### 9.5 Post-upload reconciliation

After upload, the provider reconciles the server response with the planned commits.

For each commit it records:

```text
accepted or rejected
Gerrit change identity
patch-set number
patch-set commit OID
change ref
server response
```

A successful transport exit alone is insufficient if individual mapping cannot be established.

---

### 9.6 Partial or uncertain outcome

If the network result is uncertain:

* Do not assume that no changes were created.
* Query Gerrit by the canonical identity tuple when possible.
* Mark unresolved commits as `upload-unknown`.
* Preserve the upload plan for deterministic reconciliation.
* Do not repeat the upload blindly.

---

## 10. Remote Review Discovery

### 10.1 Network permission

Review queries require explicit command intent or configured background policy.

Examples:

```console
git staircase review status auth
git staircase verify auth --provider gerrit
git staircase upload auth
```

These commands may contact the configured server.

`git staircase list` should not do so by default.

---

### 10.2 Change lookup

Lookup by Change-Id must include:

```text
server
project
destination branch
Change-Id
```

The provider must reject multiple matching results unless one is selected explicitly.

---

### 10.3 Patch-set equality

A local review commit is current on Gerrit only when its full commit OID equals the Gerrit change’s current revision OID.

The Gerrit REST API can return the current revision and patch-set commit ID.

Possible states include:

```text
not uploaded
current
local newer
server newer
diverged
merged
abandoned
lookup ambiguous
```

---

### 10.4 Related changes

Gerrit can report changes related through dependency relationships.

The provider may compare Gerrit’s related-change graph with the local staircase commit graph.

A mismatch is diagnostic evidence.

The local staircase structure remains governed by Git ancestry and managed Staircase metadata.

---

## 11. Verification and Presubmit

### 11.1 Typed evidence

The Gerrit verification capability returns typed server evidence such as:

```text
change status
current patch-set commit
labels
submit requirements
submittable state
mergeability
work-in-progress state
external checks, when available
```

Gerrit’s REST API exposes labels, evaluated submit requirements, current revision information, and submittable state as distinct fields.

The provider must preserve those distinctions.

---

### 11.2 Verification is revision-specific

A Gerrit result applies to a local commit only when:

```text
server identity matches
project matches
destination branch matches
Change-Id matches
current patch-set OID matches local commit OID
```

If the local commit is amended or rebased without upload, previous Gerrit verification is stale for the local revision.

---

### 11.3 Staircase verification aggregation

For a staircase mapped to several Gerrit changes, aggregate status is computed from the ordered review set.

Recommended states include:

```text
passed
pending
failed
blocked
stale
incomplete
unknown
```

The provider must not reduce the result to `passed` when:

* Any active review commit is not the current patch set.
* Any required submit requirement fails.
* A required review is missing.
* A review targets another branch.
* Server state could not be queried.
* The configured policy requires checks not represented in Gerrit data.

---

### 11.4 Mergeable versus submittable

`mergeable` means the revision can be merged mechanically under Gerrit’s current computation.

`submittable` reflects whether review and submit conditions are satisfied.

Neither alone means that all desired presubmit checks passed.

The provider must report each field separately.

---

### 11.5 Local verification remains separate

A local command such as:

```console
git staircase verify auth --profile local-presubmit
```

produces local verification evidence.

Gerrit verification evidence does not overwrite it.

A policy may require both.

---

## 12. Automatic Configuration

### 12.1 High-confidence `repo` composition

When the `repo` provider supplies all of:

```text
Gerrit review endpoint
project identity
destination branch
```

and the Gerrit provider is trusted and installed, the Gerrit provider may auto-bind:

```text
review = gerrit
verification = gerrit
```

without contacting the server.

The route remains locally inferred until network validation occurs.

---

### 12.2 Standalone Git repository

In a standalone repository, automatic binding is allowed when one unique high-confidence local Gerrit route exists.

Otherwise:

* The repository remains fully usable without review integration.
* `git staircase review status` explains what configuration is missing.
* The user may configure the endpoint, project, and destination explicitly.

---

### 12.3 Partial route

The provider may bind with partial readiness.

Example:

```text
review provider: Gerrit
project: known
destination: known
server host: known
upload transport: unresolved
REST authentication: unresolved
```

Local Change-Id inspection remains available.

Network commands fail narrowly with an actionable route diagnostic.

---

### 12.4 First network validation

On the first network-requiring operation, the provider may establish:

```text
confirmed server identity
supported API features
authenticated account
REST endpoint
SSH endpoint
Git upload endpoint
server version
```

This validation updates provider state but does not rewrite workspace or staircase semantics.

---

## 13. Multiple Servers and Branches

### 13.1 Same Change-Id on different branches

The same Change-Id used on two destination branches identifies two distinct review identities.

The provider must not merge them.

---

### 13.2 Same project on different servers

The same project path on two Gerrit servers represents two distinct review domains.

---

### 13.3 Destination changes

Changing a staircase’s destination branch changes the canonical Gerrit identity tuple for every associated Change-Id.

Existing review associations must be:

* Preserved as historical associations.
* Migrated only through an explicit operation.
* Never silently retargeted.

---

### 13.4 Cross-project staircase

A single repository-local staircase cannot contain commits from several Git projects.

A future workspace-level aggregate may contain one staircase per project, each with its own Gerrit route.

The Gerrit provider must not represent such an aggregate as one Gerrit change chain.

---

## 14. Review-State Persistence

A managed staircase descriptor may store:

```text
provider = gerrit
server identity
project
destination branch
mapping policy
step-to-review associations
last observed patch-set OIDs
last query timestamp
```

Remote observations are cached evidence.

They are not authoritative after their validity conditions change.

---

## 15. Commands

Recommended commands include:

```console
git staircase review show auth
git staircase review status auth
git staircase review plan auth
git staircase review upload auth
git staircase review reconcile auth
git staircase review open auth:2
git staircase verify auth --provider gerrit
git staircase provider gerrit doctor
```

Examples:

```console
git staircase review plan auth --mapping=per-commit
git staircase review upload auth --destination main
git staircase review reconcile auth
```

---

## 16. Diagnostics

A route diagnostic should distinguish:

```text
provider not applicable
provider applicable but unbound
project unresolved
destination unresolved
upload endpoint unresolved
authentication unavailable
server unreachable
review not found
review identity ambiguous
current patch set differs
verification stale
```

Example:

```text
Gerrit review provider is configured, but upload is not ready.

Known:
  server: review.example.com
  project: tools/example
  destination: refs/heads/main

Missing:
  authenticated Git upload endpoint
```

---

## 17. Security

### 17.1 Credentials

The provider must use existing credential mechanisms where possible.

It must not copy credentials into Staircase descriptors or workspace records.

---

### 17.2 Server responses

Server-provided strings, URLs, commit messages, labels, and reviewer names are untrusted display data.

They must not be evaluated as shell commands or refnames without validation.

---

### 17.3 Push options

Push-option names and values must be encoded according to Gerrit and Git transport rules.

They must not be concatenated from unvalidated shell fragments.

---

### 17.4 No bootstrap authentication

Automatic provider discovery does not prompt for credentials or initiate login.

---

## 18. Normative Invariants

### 18.1 Gerrit is not a workspace provider

It consumes project and workspace context supplied elsewhere.

### 18.2 Review identity is a tuple

Change-Id alone is insufficient.

### 18.3 Commit OID identifies the patch set

Change-Id identifies review continuity.

### 18.4 A step may map to several changes

No implicit one-step-to-one-change assumption is permitted.

### 18.5 Destination is explicit or evidenced

No default branch folklore is used.

### 18.6 Passive discovery is offline

The server is contacted only for an operation that requests remote behavior.

### 18.7 Verification is exact-revision-specific

A server result for an older patch set does not verify a rewritten local commit.

### 18.8 Mergeability and submittability remain distinct

Neither is silently translated into a generic “presubmit passed.”

### 18.9 Upload is planned against immutable OIDs

A moving branch cannot change underneath an approved plan unnoticed.

### 18.10 Uncertain uploads are reconciled

They are not blindly repeated.

### 18.11 Project and branch changes alter review identity

Existing associations are not silently reused across those boundaries.

### 18.12 `repo` hints do not fuse the providers

The Gerrit provider independently interprets and validates review semantics.

---

## 19. Summary

The composed model is:

```text
repo provider
    workspace root
    project identity
    exact workspace revision
    review endpoint hint
    destination hint
             │
             ▼
Gerrit provider
    review route
    Change-Id identity
    upload planning
    patch-set mapping
    submit requirements
    presubmit evidence
```

The governing rule is:

> Workspace metadata may tell Staircase where Gerrit probably is; only the Gerrit provider decides how reviews are identified, uploaded, and verified.
