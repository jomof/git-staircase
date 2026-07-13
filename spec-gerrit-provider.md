# Git Staircase Gerrit Provider Specification

## 1. Status and scope

This document defines the normative Gerrit provider for **Git Staircase**.

It specializes the provider contracts defined by **Git Staircase Specification**, especially its workspace and provider architecture, verification model, review-provider contract, landing model, persistent-record rules, output rules, and provider-accommodation requirements.

This document is written as a standalone provider specification. It is not an addendum and does not depend on the historical addendum ordering.

The canonical provider name is:

```text
gerrit
```

The provider MAY implement these capability classes:

```text
review
review-identity
verification
review-transport
landing
```

The provider does not implement:

```text
workspace
project-mapping
generic integration-context
```

It MAY consume typed facts and hints from a workspace, project-mapping, integration-context, repository-routing, or transport provider. In particular, it MAY consume project, review-endpoint, and destination-branch hints produced by a `repo` workspace provider, but the providers remain independently bound and replaceable.

The Gerrit provider MUST work in:

* A standalone Git repository.
* A multi-repository workspace.
* An attached worktree.
* A detached worktree.
* A repository with several possible Gerrit endpoints.
* A workspace whose projects use different Gerrit installations or destination branches.
* A repository with no network access during passive bootstrap.

The provider supports Gerrit review publication, review identity, patch-set status, labels, checks, submit requirements, and submission through Gerrit-supported interfaces such as Git receive-pack, REST, or SSH.

Provider support for a particular Gerrit version, plugin, label, check system, or submit strategy is discovered and reported as route capability. It MUST NOT be assumed merely from the provider name.

---

## 2. Normative language and imported terminology

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

This specification uses the consolidated core vocabulary without provider-local synonyms:

```text
implicit staircase
managed staircase
integration context
integration anchor
integration set
symbolic integration target
review destination
landing destination
structure revision
record revision
provider
capability binding
```

The following deprecated terms MUST NOT appear in machine schemas or normative provider interfaces:

```text
integration boundary
staircase revision
tracked staircase
adopted staircase
adapter
```

Where this document says **review**, it means a Gerrit change.

Where it says **exact review revision**, it means one exact Gerrit patch set identified by its commit OID and, when known, its patch-set number.

Where it says **route**, it means the complete typed information required to interpret or mutate a review. A route is not an integration context.

---

## 3. Provider identity and descriptor

A conforming implementation SHOULD expose a descriptor equivalent to:

```json
{
  "protocol_version": 1,
  "name": "gerrit",
  "capabilities": [
    "review",
    "review-identity",
    "verification",
    "review-transport",
    "landing"
  ],
  "probe": {
    "passive": true,
    "network": false,
    "mutates_workspace": false
  }
}
```

The descriptor states implementation capability, not readiness for the current project.

Readiness is evaluated separately for each capability and route.

The provider MUST report at least these readiness levels:

```text
not-applicable
applicable-unbound
bound-incomplete
bound-ready
bound-authentication-required
bound-stale
```

A provider binding MAY be valid while a project-specific route is incomplete.

Example:

```text
provider: gerrit
binding: bound-incomplete
server: review.example.com
project: unresolved
review destination: unresolved
```

---

## 4. Non-goals

The Gerrit provider does not:

* Determine the workspace root.
* Parse or mutate a multi-repository manifest as a workspace authority.
* Treat detached `HEAD` as a review destination or integration anchor merely because it is detached.
* Infer `main`, `master`, `trunk`, or another conventional branch name without evidence.
* Treat a remote named `origin`, `review`, or `gerrit` as authoritative solely because of its name.
* Treat SSH port `29418` as proof of Gerrit.
* Treat a Change-Id trailer as proof of the correct Gerrit server, project, or branch.
* Assume one staircase step equals one commit.
* Assume one staircase step equals one Gerrit change.
* Silently squash, reorder, amend, sign, or otherwise rewrite commits during review planning or upload.
* Automatically install or execute a repository-supplied `commit-msg` hook during passive bootstrap.
* Contact Gerrit during passive provider discovery.
* Treat a Gerrit topic as staircase lineage identity.
* Treat a numeric change number as globally unique.
* Treat a Change-Id as globally unique.
* Treat patch-set number as review identity.
* Treat “mergeable,” “approved,” “verified,” “submit requirements satisfied,” and “submitted” as interchangeable.
* Treat `repo upload` as required merely because the repository is inside a `repo` workspace.
* Use `git staircase push` for Gerrit review publication.
* Close, abandon, restore, move, submit, or delete remote reviews as an incidental effect of local archive, delete, rename, split, join, or reorder.

---

## 5. Conceptual model

### 5.1 Gerrit installation

A **Gerrit installation** is one independently administered Gerrit service.

Before network validation, an installation is represented by one or more provisional endpoint locators.

After validation, it SHOULD be represented by a canonical server identity that can survive ordinary endpoint aliases.

A hostname alone is a locator, not necessarily a canonical identity.

### 5.2 Gerrit project

A **Gerrit project** is the provider-native repository identity within one Gerrit installation.

It is distinct from:

```text
local Git repository
workspace project path
workspace project name
Git remote name
fetch URL
push URL
staircase lineage ID
```

A workspace provider's project identity MAY be accepted as a Gerrit project candidate when its provenance states that it names the Gerrit project.

### 5.3 Review destination

A Gerrit **review destination** is:

```text
canonical or provisional server identity
Gerrit project identity
full destination branch ref
```

Example:

```text
server: review.example.com
project: platform/payments
branch: refs/heads/main
```

It is not automatically the staircase integration context.

### 5.4 Review route

A complete Gerrit review route contains:

```text
server identity
project identity
review destination branch
Git publication endpoint
REST endpoint when available
SSH command endpoint when available
authentication profile
transport policy
server capability observations
```

A route key is at least:

```text
server identity
project identity
destination branch
```

Transport endpoint aliases do not create different routes when they have been proven to address the same installation and project.

### 5.5 Change-Id

A **Change-Id** is the provider-native key carried in a commit-message trailer and used by Gerrit, together with project and destination branch, to match a pushed commit to an existing change.

A Change-Id is not a review identity by itself.

### 5.6 Pending review key

Before Gerrit confirms that a change exists, the provider MAY persist a **pending review key**:

```text
server identity
project identity
destination branch
Change-Id
```

This key records publication intent and the expected matching tuple.

It does not prove that a remote review exists.

### 5.7 Confirmed review identity

After Gerrit confirms a review, the canonical confirmed review identity is:

```text
canonical server identity
server-issued change number
```

Machine form:

```json
{
  "provider": "gerrit",
  "server_id": "review.example.com",
  "change_number": 48101
}
```

The provider MUST additionally retain:

```text
project identity
destination branch
Change-Id
```

as canonical attributes and lookup aliases.

The confirmed identity survives:

* New patch sets.
* Commit rebases and amendments that retain the review's Change-Id.
* Ordinary ref renames in the local repository.
* A Gerrit branch move that preserves the same server-issued change number.

A project transfer is not assumed to preserve identity unless Gerrit explicitly reports continuity for the same change number and server.

### 5.8 Exact review revision

An exact Gerrit review revision contains:

```text
confirmed or pending review identity
patch-set commit OID
patch-set number when known
change ref when known
```

Machine form:

```json
{
  "commit_oid": "<full-commit-oid>",
  "patch_set_number": 3,
  "change_ref": "refs/changes/01/48101/3"
}
```

The commit OID is authoritative for exact source equality.

Patch-set number is server sequence metadata and MUST NOT replace the commit OID.

### 5.9 Change ref

A Gerrit change ref under `refs/changes/*` identifies one server patch set.

It is an exact revision locator, not the continuing review identity.

### 5.10 Review association

A durable Gerrit association binds:

```text
staircase lineage
managed step or aggregate subject
mapping policy
ordered local review-commit position
pending review key or confirmed review identity
exact current remote review revision when known
route
```

Durable associations require a managed staircase and are published through full-record compare-and-swap.

---

## 6. Capability binding and passive discovery

### 6.1 Passive probe inputs

A passive Gerrit probe MAY inspect only local, non-secret evidence such as:

* Existing explicit Staircase Gerrit configuration.
* Typed workspace-provider hints.
* Typed repository-routing hints.
* Git remote fetch and push URLs.
* Git refspecs that explicitly target `refs/for/*`.
* Existing managed Gerrit associations.
* Locally registered trusted Gerrit server aliases.
* Provider-owned local route cache.
* Environment variables explicitly documented as Gerrit provider selectors.

It MUST NOT access the network or test authentication.

### 6.2 Strong local evidence

Strong evidence includes:

1. Explicit provider configuration naming a server and project.
2. A trusted workspace provider explicitly identifying a Gerrit review endpoint and Gerrit project.
3. A trusted repository-routing provider identifying a Gerrit route.
4. An existing managed association whose route remains locally valid.
5. A configured publication refspec explicitly targeting `refs/for/<branch>` together with an unambiguous project endpoint.

A unique strong candidate MAY bind `review`, `review-identity`, `verification`, `review-transport`, and `landing` independently according to provider support.

### 6.3 Weak evidence

The following are insufficient by themselves:

* SSH port `29418`.
* A hostname containing `gerrit`, `review`, `code`, or `source`.
* A remote named `gerrit`, `review`, or `origin`.
* A `commit-msg` hook.
* One or more Change-Id trailers.
* Cached `refs/changes/*` refs without server identity.
* A `.gitreview` or similar file not explicitly trusted by provider policy.
* A Gerrit URL found in arbitrary source content.
* The presence of `repo` tooling.
* A manifest review host that cannot be tied to the current project.

Weak evidence MAY appear in diagnostics but MUST NOT authorize automatic network or mutation behavior.

### 6.4 Offline guarantee

During passive probing, the provider MUST NOT:

* Query Gerrit REST.
* Invoke a Gerrit SSH command.
* Run `git ls-remote`.
* Fetch change refs.
* Push a probe object.
* Test credentials.
* install hooks.
* change Git configuration.
* modify workspace-manager state.

### 6.5 Multiple candidates

One workspace MAY contain several Gerrit installations, projects, or destination branches.

The provider MUST retain distinct candidates when any of these differ:

```text
server identity
project identity
destination branch
```

It MUST NOT select by:

* Provider enumeration order.
* Remote name.
* Shortest URL.
* Lexical hostname order.
* A conventional branch name.

Read-only provider-independent commands continue normally when Gerrit binding is ambiguous. Gerrit-dependent commands fail with `provider-unbound` or `provider-route-incomplete` and candidate diagnostics.

### 6.6 Binding scope

The provider implementation may be workspace-bound, while routes are repository-, project-, staircase-, or invocation-scoped.

A workspace-wide binding MUST NOT force all projects through one server or branch.

### 6.7 Automatic binding

Automatic binding is permitted only when:

* The provider is trusted.
* The passive probe is unambiguous.
* Evidence is local and typed.
* No explicit binding conflicts.
* Persistence adds no network or remote mutation.

An incomplete but unambiguous binding MAY be persisted.

Example:

```text
review provider: gerrit
server: review.example.com
project: platform/payments
review destination: unresolved
```

### 6.8 Revalidation

Automatic Gerrit bindings and routes MUST be revalidated when relevant local evidence changes, including:

* Workspace membership.
* Project identity.
* Remotes or endpoint aliases.
* Destination hints.
* Provider version.
* Existing association route.
* Explicit configuration.

Revalidation of local evidence remains offline. Server validation occurs only during a network-requiring command.

---

## 7. Server and endpoint identity

### 7.1 Provisional identity

Before network validation, server identity is provisional and records:

```text
normalized endpoint locator
transport kind
port
path or API base path when relevant
provenance
```

Two provisional endpoints MUST NOT be merged merely because their hostnames resemble one another.

### 7.2 Canonical server identity

A canonical server identity may be established by:

* Explicit trusted configuration.
* A previously validated provider record.
* Authenticated server information.
* User-approved endpoint alias configuration.
* A provider-defined stable installation identifier.

The provider MUST preserve original endpoint locators separately from canonical identity.

### 7.3 Endpoint aliases

SSH, HTTP, and HTTPS endpoints MAY represent one installation.

They may be coalesced only with evidence.

A DNS alias alone is not sufficient when it could route to a different tenant or installation.

### 7.4 First network validation

The first network operation on a provisional route MUST validate, as applicable:

```text
server kind
canonical installation identity
project existence
review destination existence
transport endpoint compatibility
authentication state
required server capabilities
```

If validation contradicts the persisted route, the command MUST stop before mutation and report `provider-route-incomplete` or a more specific stable core error with Gerrit details.

### 7.5 Authentication

Authentication is route-specific.

The provider MUST NOT persist passwords, access tokens, cookies, private keys, or bearer headers in staircase records or machine output.

Authentication profile references MAY be persisted in user-local provider configuration.

---

## 8. Route resolution

### 8.1 Route precedence

For each route field, precedence is:

1. Explicit command input.
2. Managed staircase provider policy or association.
3. Explicit project-specific Gerrit configuration.
4. Trusted workspace or repository-routing fact.
5. Trusted workspace hint.
6. Unique locally derived Gerrit route.
7. Network-confirmed provider result when the command permits network access.
8. Unresolved.

A weaker source MUST NOT overwrite a stronger source.

### 8.2 Project identity

Project identity candidates MAY come from:

* Explicit `--project` input.
* Managed review association.
* Explicit provider configuration.
* A workspace provider's typed Gerrit project fact.
* A uniquely parsed review push URL.
* A network query explicitly permitted by the command.

A filesystem path or Git remote name MUST NOT be stored as Gerrit project identity unless a trusted mapping proves that equivalence.

### 8.3 Destination branch

Destination-branch precedence is:

1. Explicit review destination argument.
2. Existing confirmed review association.
3. Managed review policy.
4. Explicit provider configuration.
5. Trusted workspace destination fact.
6. Unique locally configured review branch.
7. Unresolved.

The internal form is always a full ref:

```text
refs/heads/<branch>
```

The provider MUST NOT invent a destination from `main`, `master`, `trunk`, current branch name, or remote symbolic `HEAD` without an explicit policy authorizing that evidence source.

### 8.4 Integration context remains separate

A review destination MAY be offered to the core as integration-context evidence when a corresponding exact local or remotely resolved destination OID is available.

The core decides the integration context.

The provider MUST preserve separately:

```text
integration anchor
symbolic integration target
review destination
review transport ref
landing destination
```

### 8.5 Review transport ref

For destination:

```text
refs/heads/main
```

the ordinary Gerrit review transport ref is conceptually:

```text
refs/for/main
```

Server-specific accepted forms MAY differ.

Push options are typed fields and MUST NOT be treated as part of destination identity.

### 8.6 Route completeness

Planning may proceed with a route that has enough local information to validate commits and mapping.

Remote status requires a query endpoint.

Upload requires a publication endpoint.

Landing requires a submission endpoint and confirmed review identities.

The provider MUST report which route fields are missing for the requested operation.

### 8.7 Cross-project staircase

A linear staircase MAY contain commits intended for several repositories only when represented by a higher-level multi-project operation outside one Git commit graph.

Within one Git repository, one Gerrit publication plan MUST use one project and destination branch unless the provider exposes an explicit multi-route plan and the core operation journals each route separately.

The provider MUST NOT split one repository's commit chain among projects based on path contents.

---

## 9. Supported review mappings

### 9.1 General rule

The selected mapping policy MUST be explicit in the plan and durable association.

When no managed policy or invocation option exists, Gerrit's provider-native mapping resolves to:

```text
mapping: per-commit
topology: stacked commit ancestry
```

The plan MUST print that resolution.

### 9.2 Per-commit mapping

Under `per-commit`:

```text
one active review commit -> one Gerrit change
```

A staircase step may contain zero, one, or several review commits.

The step association stores an ordered review set following commit ancestry.

This is the native Gerrit mapping.

### 9.3 Per-step mapping

Under `per-step`:

```text
one active staircase step -> one Gerrit change
```

Every active step MUST contain exactly one review commit before publication.

If a step contains several commits, planning MUST fail or present a separate explicit normalization command.

The provider MUST NOT squash as a side effect of `review create` or `review upload`.

### 9.4 Aggregate mapping

Under `aggregate`:

```text
one active staircase -> one Gerrit change
```

The active staircase body MUST be represented by exactly one review commit.

A multi-commit staircase requires an explicit core rewrite before aggregate publication.

### 9.5 Stacked topology

Gerrit naturally represents dependent changes when review commits form an ancestry chain.

The provider MUST validate that each active review commit's first parent is either:

* The previous active review commit.
* A commit in the selected integration set.
* An explicitly supported merge parent arrangement.

A provider MAY report the generic topology as `stacked`, but the durable mapping remains commit- or step-oriented.

### 9.6 Cumulative mapping

A cumulative set in which several independent commits each contain all lower changes is not Gerrit's ordinary dependent-change model.

The provider MUST reject `cumulative` unless an explicit provider extension defines exact matching, diff, and landing semantics.

It MUST NOT label a cumulative set as an incremental stack.

### 9.7 Provider-native mapping

`provider-native` is accepted as a policy value and resolves deterministically to `per-commit` for this provider version.

Machine output MUST contain both:

```json
{
  "requested_mapping": "provider-native",
  "resolved_mapping": "per-commit"
}
```

### 9.8 Merge commits

A publication set containing merge commits requires explicit route and server support.

The plan MUST show every parent relationship and explain how Gerrit will interpret the merge.

Absent confirmed support, the provider returns a validation failure and performs no upload.

### 9.9 Already integrated commits

A commit reachable from the exact review destination or selected integration set is not an active review commit.

The provider MUST exclude it from new publication and reconcile any associated Gerrit change as merged, superseded, or inconsistent according to remote evidence.

### 9.10 Partial landing

After lower changes land, the provider MUST recompute the active review set against the new integration context.

It MUST NOT re-upload landed commits merely because their Change-Ids remain in local history.

---

## 10. Change-Id and commit-message rules

### 10.1 Trailer parsing

The provider parses Change-Id using Git commit-message trailer semantics.

For every review commit it distinguishes:

```text
absent
valid-single
malformed
multiple
```

Text outside the trailer block does not establish a Change-Id.

### 10.2 Validity

A valid Change-Id MUST satisfy the Gerrit route's accepted grammar.

The common form is:

```text
I<40 hexadecimal digits>
```

The provider MUST NOT assume one grammar when a validated server explicitly advertises another supported form.

### 10.3 Missing Change-Id

A missing Change-Id has these consequences:

* A durable pending review key cannot be formed.
* Existing-review continuity cannot be proven locally.
* `review create` cannot prepare a stable association under the default Git-push workflow.
* `review upload` MAY create an unassociated remote review only when an explicit ephemeral policy permits it and the server accepts it.

The default managed workflow MUST fail with a provider-specific detail under a stable core validation code.

It SHOULD suggest:

```console
git staircase normalize <selector> --provider gerrit --ensure-change-ids
```

That command is explicit and may rewrite commits. It MUST produce a rewrite plan, preserve step identity, restack upper commits, update owned refs, invalidate exact verification, and use the general mutation protocol.

### 10.4 Commit-message hooks

The provider MAY invoke a trusted configured Change-Id generator only during an explicit normalization or commit-message operation.

It MUST NOT:

* Install a hook silently.
* Execute an untrusted repository hook automatically.
* Run a hook during passive discovery.
* Modify a commit during upload planning.

### 10.5 Multiple Change-Ids in one commit

A commit with more than one Change-Id trailer is identity-ambiguous.

The provider MUST NOT choose one silently.

### 10.6 Duplicate Change-Ids in one publication set

Two active review commits with the same Change-Id and route are invalid unless the server and an explicit mapping extension define that relationship.

The ordinary provider MUST reject the plan before network mutation.

### 10.7 Existing route mismatch

If a Change-Id matches an existing review in another project or branch, the provider MUST NOT silently attach, move, or update it.

It reports the existing route and requires an explicit `review attach`, provider-supported move, or new Change-Id workflow.

### 10.8 Rebase and amendment

A rewritten commit retaining its Change-Id may remain associated with the same confirmed review identity.

Its exact review revision changes after upload.

A rewritten commit whose Change-Id changes is a different pending review key unless the user explicitly attaches it to an existing confirmed review.

### 10.9 Split

When one review commit is split into several commits:

* At most one resulting commit MAY retain the original Change-Id by default.
* Newly created review commits require distinct Change-Ids.
* The existing confirmed review remains associated only with the surviving conceptual review subject selected by the core operation.
* The provider MUST NOT duplicate the original Change-Id.

For a staircase step split, core step-identity rules determine which step survives. Gerrit review association follows the surviving review commit, not merely the old ordinal branch name.

### 10.10 Join

When review commits are joined:

* One explicitly selected surviving review identity MAY remain.
* Other confirmed reviews become retired associations, not silently reassigned.
* Remote changes are not abandoned automatically.
* The plan MUST show which reviews will no longer have an active local subject.

### 10.11 Reorder and move

Reorder or moving changes between steps may preserve a Gerrit review identity only when the corresponding Change-Id remains attached to the intended review commit.

If patch content moves while Change-Ids stay with old commits, the provider reports the resulting review semantics rather than guessing user intent.

### 10.12 Drop

Dropping a step or commit removes its active local mapping subject.

The local association MAY be retained as retired history or detached explicitly.

The remote change is not abandoned by default.

### 10.13 Commit signatures

Adding or editing a Change-Id rewrites the commit and invalidates any signature over the previous commit object.

An explicit normalization plan MUST account for configured signing policy and report whether rewritten commits will be re-signed.

### 10.14 Encoding and untrusted content

Commit messages are untrusted input.

The provider MUST bound parsing, preserve raw commit identity, and safely escape control characters in diagnostics and output.

---
## 11. Persistent review associations

### 11.1 Management requirement

A durable Gerrit association requires a managed staircase.

The following operations therefore require existing management or automatic adoption:

```console
git staircase review create <selector>
git staircase review attach <selector> --provider gerrit --review <review-selector>
git staircase review upload <selector>
```

`review upload --ephemeral` MAY publish an implicit staircase without retaining associations when the core and route permit it. Human and machine output MUST state that review continuity is not recorded.

### 11.2 Structural versus observed state

Durable review mapping is structural state when it affects interpretation of steps, review identity, or landing order.

The structure descriptor stores, as applicable:

```text
provider name
route identity
mapping policy
step ID or aggregate subject
ordered review entries
pending review keys
confirmed review identities
review-commit Change-Ids
```

Remote observations such as current labels, reviewers, checks, patch-set status, and mergeability are cached provider state. Refreshing only those observations SHOULD NOT change the structure revision or record revision.

### 11.3 Ordered review entries within a step

Under `per-commit`, each managed step stores an ordered review set.

Each entry contains:

```text
Change-Id
current local commit OID
pending review key or confirmed review identity
last associated exact remote revision
association state
```

Commit order follows ancestry, not lexical Change-Id or change number.

### 11.4 Full-record compare-and-swap

Any operation that creates, attaches, detaches, retires, retargets, or otherwise mutates a durable Gerrit association MUST compare the complete expected staircase record revision.

A provider MUST NOT update only a provider sidecar and silently overwrite a concurrent:

* Rebase.
* Restack.
* Split.
* Join.
* Reorder.
* Archive.
* Metadata edit.
* Policy edit.
* Independent review-association edit.

### 11.5 One logical read

A review operation reads one immutable staircase record for planning.

It MUST NOT combine structure from one record revision with associations from another.

### 11.6 Publication order

For a persistent association mutation:

1. Resolve the selected staircase and expected record revision.
2. Validate route, mapping, and exact local commits.
3. Write any new immutable provider-association objects or structure blob.
4. Write the new record tree.
5. Verify applicable record refs still equal the expected record revision.
6. Publish all applicable record refs transactionally.
7. Update provider observation cache only after durable association publication or through a recoverable journal.

### 11.7 Provider cache key

Cached Gerrit observations MUST be keyed by enough information to prevent cross-review or cross-revision reuse, including:

```text
canonical or provisional server identity
confirmed review identity or complete pending review key
exact patch-set commit OID
observation type
provider schema version
```

A cached observation for one patch set MUST NOT satisfy status or verification for another.

### 11.8 Association states

A Gerrit association MAY use these generic synchronization states:

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

Provider-native change status remains a separate field.

### 11.9 Retired associations

A review whose active local subject disappears after join, drop, aggregate rewrite, or partial landing MAY remain in retained provider history.

A retired association:

* Does not participate in upload planning by default.
* Does not satisfy current review policy.
* Preserves the known confirmed review identity and last exact revision.
* Does not trigger remote abandonment.

### 11.10 Archive behavior

Archiving a staircase preserves Gerrit associations.

Archive performs no Gerrit network mutation by default.

While archived:

* `review show` and `review status` MAY inspect cached state.
* Network refresh requires explicit command intent.
* `review create`, `review upload`, and `land` fail with `archived-mutation`.
* Remote reviews remain unchanged.

Unarchive MUST revalidate route and provider readiness before later mutation.

---

## 12. Review planning

### 12.1 Canonical command

The canonical planning command is:

```console
git staircase review plan <selector> --provider gerrit [--mapping <policy>]
```

When `gerrit` is the unambiguous bound review provider, `--provider gerrit` MAY be omitted.

### 12.2 Plan contents

A complete Gerrit review plan MUST contain:

```text
provider identity
route and destination
exact integration anchor
selected record revision or implicit structural key
structure revision when managed
resolved mapping and topology
ordered active review commits
step IDs and ordinals
Change-Ids
pending or confirmed review identities
exact last observed remote revisions
create, update, no-op, retire, or blocked action
expected remote lease state
transport ref
push options
verification consequences
normalization requirements
```

### 12.3 Exact source

Every planned review commit is a full immutable commit OID.

The plan MUST NOT store a moving branch as the authoritative upload source.

Primary branches and other refs MAY appear as protected locators with expected OIDs.

### 12.4 Record and ref leases

A plan records:

```text
expected staircase record revision
expected structure revision or structural key
expected local source ref OIDs
expected remote patch-set numbers and OIDs when known
```

Before upload, all applicable leases MUST be rechecked.

### 12.5 Local-only versus remote-aware planning

A local-only plan validates:

* Route evidence available locally.
* Mapping.
* Commit ancestry.
* Change-Id trailers.
* Existing durable associations.
* Exact local source OIDs.

A remote-aware plan additionally queries Gerrit for:

* Existing matching changes.
* Current patch sets.
* Change status.
* Destination validity.
* Submit and upload capabilities.

The command MUST indicate whether remote state was queried.

### 12.6 No silent normalization

If publication requires commit rewriting, the plan MUST stop before mutation and show the separate normalization action.

Examples include:

* Adding Change-Ids.
* Squashing several commits into one per-step review.
* Splitting one commit into several reviews.
* Removing duplicate Change-Ids.
* Re-signing rewritten commits.
* Linearizing unsupported merge commits.

### 12.7 Action classification

For each planned review commit, action is one of:

```text
create
update
no-op
attach-existing
blocked
retire-local-association
unknown-until-query
```

`no-op` requires exact equality with the current remote patch-set commit OID.

### 12.8 Review destination change

If the requested route differs from an existing confirmed association, the plan MUST classify the item as `retargeted` or blocked.

It MUST NOT assume that pushing the same Change-Id to a new branch updates the old change.

A provider-supported Gerrit move is a distinct remote mutation and requires an explicit plan.

### 12.9 Draft-aware plan

```console
git staircase review plan <selector> --provider gerrit --include-draft
```

MAY show a hypothetical publication set after materializing the exact staged index.

It MUST label the plan hypothetical and list excluded unstaged, untracked, and ignored content.

No Gerrit mutation occurs until the draft has been explicitly materialized and the resulting record revision has been published locally.

### 12.10 Human plan example

```text
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a500000
  structure revision: 8b02a65
  mapping: per-commit
  topology: stacked

  step  commit    Change-Id                                  review  action
  1     b511000   I1111111111111111111111111111111111111111 48101   update
  2     c516000   I4444444444444444444444444444444444444444 none    create
  3     c522000   I2222222222222222222222222222222222222222 48102   update
  4     d532000   I3333333333333333333333333333333333333333 48103   update

No remote mutation performed.
```

---

## 13. Review creation

### 13.1 Canonical command

```console
git staircase review create <selector> --provider gerrit
```

The selector MAY identify a complete managed staircase or one managed step when the mapping permits step-scoped creation.

### 13.2 Default Git-push workflow

In the default Gerrit Git-push workflow, `review create` prepares durable pending review keys from existing Change-Ids.

It does not need to publish commits immediately.

Example result:

```text
Prepared 3 review identities for 'payments'.
  step 1: I1111111111111111111111111111111111111111  pending first upload
  step 2: I2222222222222222222222222222222222222222  pending first upload
  step 3: I3333333333333333333333333333333333333333  pending first upload

record revision: 88ad41c -> 9bf5a72
Remote publication is still required.
```

The word **identity** in this output refers to pending review keys. Structured output MUST distinguish them from confirmed review identities.

### 13.3 Server-side creation

A Gerrit installation or provider extension MAY support creating a change through REST or another native operation.

When used:

* The command MUST identify the exact initial commit or edit subject.
* It MUST report whether a confirmed change number was created.
* It MUST distinguish creation from patch-set publication.
* Partial or uncertain results MUST be journaled and reconciled.

### 13.4 Missing Change-Ids

If required review commits lack Change-Ids, default `review create` fails before record mutation.

It MUST NOT insert trailers itself unless an explicit combined normalization mode is defined and selected.

### 13.5 Existing matching changes

Remote-aware creation MAY discover that a matching Gerrit change already exists.

It MUST not attach it silently when:

* More than one match exists.
* The route differs.
* The existing review is merged or abandoned and policy does not permit reuse.
* The current exact revision is incompatible.

The plan SHOULD suggest an explicit `review attach` or a new Change-Id.

### 13.6 Partial and uncertain creation

If some identities are prepared or created and others fail:

* The operation journal records per-item outcome.
* Confirmed remote identities are never discarded.
* The local record update is completed consistently or remains recoverable.
* Repeating the command blindly is forbidden.
* `git staircase review reconcile <selector>` is the next action when remote outcome is unknown.

---

## 14. Review upload

### 14.1 Canonical command

```console
git staircase review upload <selector> --provider gerrit
```

The top-level command:

```console
git staircase upload
```

is noncanonical and MUST NOT be implemented as an alias.

### 14.2 Preconditions

Before remote mutation, the provider MUST verify:

* The selected record revision or structural key is unchanged.
* Planned source refs still point to planned OIDs.
* The route is complete and validated for publication.
* Authentication is available.
* Mapping preconditions hold.
* Every required Change-Id is valid and unambiguous.
* No duplicate Change-Id exists in the active publication set.
* Existing confirmed identities still belong to the planned route.
* Expected remote patch-set leases still hold or have been explicitly reconciled.
* No active conflicting Staircase or external Git operation controls the source worktree.
* The staircase is active, not archived.

### 14.3 Publication source

The provider uploads exact commit OIDs.

For an ordinary linear per-commit stack, it MAY push the exact aggregate top OID to the review transport ref when that push includes precisely the intended active review commits.

If one push would include unintended commits, the provider MUST construct another exact publication plan or fail.

### 14.4 Publication transport

The provider MAY use:

* Direct `git push` to `refs/for/*`.
* A trusted Gerrit-specific transport implementation.
* A separately bound review-transport provider that preserves Gerrit semantics.

Being in a `repo` workspace does not require delegation to `repo upload`.

If `repo upload` or another helper is used, its effects MUST still satisfy the immutable plan, lease, output, and reconciliation requirements in this specification.

### 14.5 Push options

Gerrit push options MUST be represented as typed plan fields.

Common fields MAY include:

```text
topic
reviewers
CC recipients
hashtags
work-in-progress state
private state
notification policy
base selection when supported
```

The provider MUST validate and safely encode values.

It MUST NOT concatenate untrusted values into shell source or an unchecked refname suffix.

### 14.6 Current versus local-newer

When the exact current remote patch-set OID equals the local commit OID, action is `no-op` and synchronization is `current`.

When the local associated commit differs and the remote lease is unchanged, action is `update` and synchronization before upload is `local-newer`.

### 14.7 Remote-newer

When Gerrit contains a newer patch set than the last observed association:

* The provider reports `remote-newer`.
* It MUST NOT hide the remote patch set by relying only on local Change-Id continuity.
* Default upload stops before mutation.
* The user must reconcile or explicitly accept the remote state according to policy.

Gerrit patch sets are append-only server history, but unnoticed remote changes can still invalidate review, verification, or user intent.

### 14.8 Diverged

State is `diverged` when local and remote exact revisions differ and neither is established as the intended successor under the current association and operation history.

The provider MUST show both OIDs and available patch-set lineage.

It MUST NOT choose one silently.

### 14.9 Post-upload reconciliation

After transport, the provider MUST establish for every planned commit:

```text
accepted, rejected, or unknown
confirmed change identity
patch-set number
patch-set commit OID
change ref when available
provider status
```

A successful process exit is insufficient when individual mappings remain unresolved.

### 14.10 Record publication after upload

Confirmed review identities and exact remote revisions are durable association changes.

They MUST be published through full-record compare-and-swap or a journaled recoverable transaction.

If the local record changed after remote success, the provider MUST preserve the remote result and enter reconciliation rather than discarding it or overwriting the new record.

### 14.11 Partial result

The provider MUST NOT assume one Gerrit push is fully atomic unless the validated transport declares and proves that transaction scope.

Per-item server outcomes are authoritative when available.

### 14.12 Uncertain result

If network or process failure leaves outcome uncertain:

* Preserve the exact plan and transport identifiers.
* Mark affected items `upload-unknown`.
* Do not assume success or failure.
* Do not retry blindly.
* Instruct the user to run `git staircase review reconcile <selector>`.

### 14.13 Ephemeral upload

```console
git staircase review upload <implicit-selector> --provider gerrit --ephemeral
```

MAY publish an implicit staircase when route and mapping are fully explicit.

The command MUST state:

```text
Persistent review associations were not recorded.
Future rewrites may not be matched to these reviews automatically.
```

### 14.14 Example upload result

```text
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

---

## 15. Review reconciliation

### 15.1 Canonical command

```console
git staircase review reconcile <selector> --provider gerrit
```

### 15.2 Purpose

Reconciliation resolves:

* `upload-unknown` outcomes.
* Remote-newer patch sets.
* External patch-set creation.
* Server-confirmed review identities not yet stored locally.
* Route changes.
* Gerrit branch moves.
* Abandoned, merged, deleted, or inaccessible changes.
* Local record publication failure after remote success.
* Multiple remote matches for one pending review key.
* Existing review attachment inconsistencies.

### 15.3 Query keys

The provider queries by strongest available identity:

1. Confirmed server identity plus change number.
2. Confirmed server-native stable identifier.
3. Complete pending review key: server, project, branch, Change-Id.
4. Exact change ref for one known patch set.

It MUST NOT reconcile by title, topic, reviewer, branch spelling alone, or bare numeric change number without server scope.

### 15.4 Deterministic result

Reconciliation produces a plan before local mutation.

Possible actions include:

```text
confirm current association
record remote-newer
record remote merge
record abandonment
attach confirmed identity to pending key
mark identity-ambiguous
retire stale local association
require explicit route move
require explicit local choice
```

### 15.5 No destructive default

Reconciliation does not by default:

* Upload a new patch set.
* Abandon a change.
* Restore an abandoned change.
* Move a change to another branch.
* Rewrite local commits.
* Reset local branches.
* Change staircase step order.

### 15.6 Concurrent record change

If the staircase record changes during reconciliation, publication fails with `concurrent-record-update` and preserves the fetched Gerrit observation for a later retry.

### 15.7 Idempotency

When local association and remote state are already reconciled, the command succeeds as an explicit no-op.

Structured output MUST distinguish the no-op from a record transition.

---

## 16. Existing review attachment and detachment

### 16.1 Attach command

```console
git staircase review attach <step-or-staircase-selector> \
    --provider gerrit --review <gerrit-review-selector>
```

### 16.2 Accepted Gerrit review selectors

A provider MAY accept:

```text
full Gerrit change URL
server-qualified numeric change number
server/project/branch/Change-Id tuple
server-native change identifier
exact change ref
```

A bare numeric change number is accepted only when one server is already unambiguous.

A bare Change-Id is accepted only when server, project, and destination branch are already unambiguous.

### 16.3 Attachment validation

Attachment MUST:

* Resolve exactly one confirmed review identity.
* Resolve or record the exact current patch-set OID.
* Confirm project and destination.
* Validate mapping compatibility.
* Compare the remote patch against the selected local subject.
* Report `current`, `local-newer`, `remote-newer`, or `diverged`.
* Publish through full-record compare-and-swap.

Title similarity or topic membership is insufficient.

### 16.4 Provisional attachment

When network validation is unavailable, a route-complete pending review key MAY be attached provisionally.

A provisional association:

* Does not prove a remote change exists.
* Does not satisfy review or verification policy.
* Must be visibly labeled provisional.
* Requires reconciliation before upload or landing under strict policy.

### 16.5 Incompatible multiple attachment

One confirmed Gerrit review MUST NOT be attached to incompatible local subjects unless the selected mapping explicitly permits that relationship.

The provider reports every existing attachment and performs no mutation.

### 16.6 Detach command

```console
git staircase review detach <step-or-staircase-selector> \
    --provider gerrit [--review <gerrit-review-selector>]
```

Detach removes only the local durable association by default.

It does not abandon, delete, move, or otherwise mutate the Gerrit change.

### 16.7 Historical retention

Detach MAY retain a non-active historical event recording the former association, subject to core retention policy.

The detached review no longer participates in current upload, verification, or landing policy.

---

## 17. Review status, show, and open

### 17.1 Commands

```console
git staircase review status <selector> --provider gerrit
git staircase review show <selector> --provider gerrit
git staircase review open <step-selector> --provider gerrit
```

### 17.2 Network behavior

`review status` and `review show` MAY contact Gerrit because the command explicitly requests provider state.

A `--cached` mode MAY prohibit network access and return only typed cached observations with age and freshness information.

`git staircase list` MUST NOT contact Gerrit implicitly.

### 17.3 Status fields

For every mapped review, status MUST preserve at least:

```text
local exact commit OID
confirmed or pending review identity
remote exact patch-set OID
patch-set number
provider change status
synchronization state
work-in-progress or private state
review labels and approvals
checks or plugin results
mergeability
submit requirements
submittability
queue or submit state when applicable
observation time and provenance
```

### 17.4 Exact equality

A local review commit is `current` only when its full OID equals the exact current Gerrit patch-set commit OID.

Patch-ID similarity, equal tree, matching Change-Id, or equal diff is not exact equality.

Such similarities MAY appear as diagnostics but MUST NOT produce `current`.

### 17.5 Related changes

Gerrit's related-changes information MAY corroborate dependency topology.

It MUST NOT redefine the local staircase, step order, or integration context by itself.

Related changes outside the selected mapping are reported separately.

### 17.6 Review open

`review open` opens or prints the canonical URL of one confirmed review.

It MUST fail for:

* Pending-only identity.
* Ambiguous step mapping.
* Several per-commit reviews without an explicit review selector.
* Unknown server web route.

It performs no review mutation.

### 17.7 Example status

```text
payments
  provider: gerrit
  destination: platform/payments refs/heads/main

  step  local     review  remote revision  sync         review state  verification
  1     b110000   48101   b110000          current      needs review  pending
  2     c120000   48102   c120000          current      needs review  pending
  3     d130000   48103   d130000          current      needs review  pending
```

---
## 18. Verification and submit evidence

### 18.1 Verification subjects

The Gerrit provider MAY produce evidence for these core subject kinds:

```text
provider-review-revision
provider-test-merge
structure-prefix
structure-aggregate
landed-revision
```

It MAY add namespaced subject kinds when required by a Gerrit plugin.

### 18.2 Exact subject requirement

Provider review evidence applies to one exact patch-set commit OID.

Evidence MUST NOT be promoted to a rewritten local commit merely because:

* The Change-Id is unchanged.
* The patch-set number is newer or older.
* The tree is equal.
* Patch IDs are similar.
* Gerrit displays the changes as related.

### 18.3 Evidence record

A Gerrit verification observation MUST identify at least:

```text
canonical server identity
project
destination branch
confirmed review identity
exact patch-set commit OID
patch-set number when known
exact destination or test base OID when known
subject type
profile and policy identity
labels and votes
checks and statuses
submit requirements
mergeability
submittability
result
observation time
provider schema version
```

If the exact destination or test base is unavailable, the provider MUST state the resulting freshness limitation.

### 18.4 Distinct provider concepts

The provider MUST keep these concepts separate:

```text
change status
work-in-progress state
private state
label vote
review approval
review rejection
check result
presubmit result
mergeability
submit requirement
submittability
submitted state
```

No one field automatically implies another.

### 18.5 Labels

Gerrit labels are provider-native typed evidence.

The provider MUST preserve:

```text
label name
vote value
permitted range when known
voter identity or opaque provider principal
patch set to which the vote applies
whether Gerrit copied the vote forward
```

A policy MAY interpret labels such as `Code-Review` or `Verified`, but the provider MUST NOT hard-code those names as universal Gerrit semantics.

### 18.6 Copied votes

When Gerrit copies a vote from an earlier patch set, the provider MUST report that provenance.

A copied vote satisfies Staircase policy only when the configured policy accepts Gerrit's copy rule for the exact current patch set.

### 18.7 Checks and plugins

Checks, CI results, and plugin-provided statuses MAY use provider-specific schemas.

The provider MUST preserve the producing system, run identifier, exact revision, status, conclusion, and URL when available.

Unknown check types remain namespaced provider evidence and MUST NOT be discarded.

### 18.8 Submit requirements

Submit requirements are evaluated by Gerrit for a particular change and patch set under server configuration.

The provider MUST preserve:

```text
requirement name
satisfied, unsatisfied, not-applicable, or error state
provider explanation
applicability expression or opaque identifier when available
blocking status
```

A generic `passed` result requires all Staircase-configured provider requirements to be satisfied for the exact current revisions.

### 18.9 Mergeability

Mergeability means Gerrit currently believes the change can be merged against its destination under its merge computation.

It does not imply:

* Required review approval.
* Passing checks.
* Satisfied submit requirements.
* Permission to submit.
* Successful future submission.

### 18.10 Submittability

Submittability means the server currently reports the change eligible for submission under the applicable route and user context.

It is distinct from mergeability and may change without a new patch set because of votes, permissions, destination movement, or server policy.

### 18.11 Staircase aggregate verification

For a per-commit Gerrit mapping, aggregate provider verification MUST evaluate every active mapped review and the dependency chain.

It MUST fail or remain incomplete when:

* Any active review is not on the exact local commit.
* Any required review is missing.
* An upper change depends on an unrecognized or stale lower patch set.
* Required evidence is pending, failed, blocked, or unknown.
* The destination or submit requirement context is stale.

### 18.12 Prefix verification

For `--each-prefix`, the provider evaluates each cumulative staircase prefix.

In a one-change-per-step mapping, each prefix may correspond to the exact current patch set at that cut.

In per-commit mappings with several commits per step, the provider MUST identify the exact final review commit for each prefix and include all lower review evidence.

### 18.13 Local verification remains separate

Local build and test evidence is not Gerrit provider evidence unless explicitly imported through a typed verified channel.

Gerrit evidence is not local execution evidence.

Both MAY be required by one Staircase verification profile.

### 18.14 Verification command

```console
git staircase verify <selector> --provider gerrit
```

The command queries or consumes Gerrit evidence according to the selected profile.

It MUST print the exact structure revision and patch-set OIDs evaluated.

### 18.15 Result mapping

Recommended aggregate result mapping is:

```text
passed      all required exact evidence satisfied
pending     provider work in progress
queued      provider work queued but not complete
failed      required check, vote, or requirement failed
blocked     policy cannot proceed because of review state or dependency
stale       evidence belongs to another exact revision or base
incomplete  required evidence absent
unknown     provider result cannot be determined
```

### 18.16 Example verification output

```text
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

---

## 19. Gerrit landing

### 19.1 Capability

When bound for `landing`, the Gerrit provider integrates reviewed changes by requesting Gerrit submission and reconciling the actual destination result.

The canonical command remains the core command:

```console
git staircase land <selector> --provider gerrit <landing-options>
```

The provider MUST NOT introduce `submit` as a top-level synonym for `land`.

### 19.2 Landing destination

The landing destination is normally the confirmed Gerrit project and destination branch of the active review associations.

It MUST be separately typed from:

* Integration anchor.
* Symbolic integration target.
* Review transport ref.
* Local branch upstream.

### 19.3 Preconditions

Before landing, the provider MUST verify:

* Every selected subject has a confirmed Gerrit review identity.
* Every selected local review commit equals the exact current Gerrit patch-set OID.
* Review destination and landing destination agree with policy.
* Required lower dependencies are current or already integrated.
* Required labels, checks, and submit requirements are satisfied.
* The change is not abandoned, merged unexpectedly, or inaccessible.
* The staircase record and source refs still satisfy local leases.
* The destination observation is current enough for policy.
* The staircase is active.

### 19.4 Stepwise landing

The provider supports:

```console
git staircase land <selector> --provider gerrit --stepwise
git staircase land <selector> --provider gerrit --through <step-selector>
```

Changes are submitted bottom to top.

After each confirmed submission, the core and provider MUST:

1. Resolve the exact resulting destination OID.
2. Determine the actual landing method and graph.
3. Confirm which reviewed commits became reachable.
4. Mark the submitted review merged.
5. Advance the integration context.
6. Recompute the active staircase body.
7. Determine whether upper review commits remain valid descendants.
8. Restack and upload upper revisions when required by the approved plan.
9. Re-evaluate approvals, checks, and submit requirements.
10. Continue only when the next review is current and permitted.

### 19.5 Aggregate landing

```console
git staircase land <selector> --provider gerrit --aggregate
```

is supported only when the plan proves one of:

* The staircase maps to one Gerrit change.
* Gerrit can atomically submit exactly the selected dependent change set.
* A provider extension defines a verified atomic submit group containing no unrelated changes.

The provider MUST reject aggregate landing when Gerrit would submit an unknown or broader set.

### 19.6 Submit whole topic

A Gerrit configuration may support submission of all changes in a topic.

A topic is not staircase identity.

Before using whole-topic submission, the provider MUST query the exact topic membership and prove that the submitted set is exactly the selected landing set or an explicitly approved superset.

Unrelated topic members block default aggregate landing.

### 19.7 Dependent change submission

Gerrit may submit a change together with dependent ancestors according to server behavior.

The plan MUST enumerate the complete expected submission closure.

A top change MUST NOT be submitted as though it were isolated when Gerrit will also submit lower changes.

### 19.8 Method constraint

A user-supplied:

```console
--method <method>
```

is a required landing constraint, not an assumption.

The provider MUST validate that Gerrit project configuration and the selected changes can use that method.

After submission, actual destination graph is authoritative.

### 19.9 Actual graph reconciliation

Gerrit may land by merge commit, rebase, cherry-pick, fast-forward, or another configured strategy.

The provider MUST inspect or query the resulting destination and report:

```text
old destination OID
new destination OID
actual submit strategy when known
new commits created by Gerrit
which reviewed commits are ancestors
patch-integrated equivalence when exact commits are not ancestors
```

### 19.10 Review evidence after restack

If a lower submission causes upper commits to be rewritten:

* Existing confirmed review identities MAY survive through unchanged Change-Ids.
* Exact remote revisions become `local-newer` until uploaded.
* Previous exact patch-set verification becomes stale.
* Copied approvals are accepted only according to policy and observed Gerrit behavior.

### 19.11 Partial landing

After a selected lower prefix lands:

* Landed associations become merged history.
* Remaining active steps keep their stable step IDs.
* Review status is recomputed relative to the new integration context.
* Local sequential branches are renumbered only after the landing operation confirms and publishes the new local structure.

### 19.12 Already merged review

If Gerrit reports a selected review already merged:

* The provider queries the exact destination result.
* It determines whether the expected patch or commit is integrated.
* It reconciles local structure before attempting further submission.
* It does not submit the review again.

### 19.13 Abandoned review

An abandoned review blocks landing.

The provider MAY offer a separately planned restore action, but `land` MUST NOT restore it silently.

### 19.14 Work-in-progress and private changes

Work-in-progress or private state remains provider state.

It may block landing according to server or Staircase policy.

`land` MUST report the blocking state rather than changing it implicitly.

### 19.15 Uncertain landing

When submission outcome is uncertain:

* Preserve the exact landing plan and request identifiers.
* Mark the operation unresolved.
* Query Gerrit change status and the exact destination before retry.
* Avoid resubmission until reconciliation determines actual outcome.
* Preserve local branches and record refs needed for recovery.

### 19.16 External submission

A review may be submitted outside Staircase.

`review status`, `review reconcile`, or `land` MUST detect that state and reconcile the actual destination graph.

External submission MUST NOT silently redefine local staircase structure without a planned local record update.

---

## 20. Route and destination changes

### 20.1 Branch move

Gerrit may support moving an existing change to another destination branch.

A branch move preserves confirmed review identity only when Gerrit reports the same server-issued change number.

The provider MUST update the route attributes through a planned full-record mutation.

### 20.2 Push to a different branch

Pushing a commit with the same Change-Id to a different branch does not prove continuity with the existing change.

The provider MUST query and reconcile the resulting review identity.

### 20.3 Project change

Publishing to another Gerrit project creates a different route and normally a different review.

The provider MUST NOT infer continuity solely from Change-Id equality.

### 20.4 Server change

The same project, branch, Change-Id, or numeric change number on another Gerrit installation is a different identity scope.

### 20.5 Destination deletion or rename

When a destination branch is deleted, renamed, or inaccessible:

* The route becomes incomplete or stale.
* Upload and landing stop before mutation.
* Existing confirmed reviews remain historical identities.
* The provider reports explicit route repair choices.

### 20.6 Integration-context consequences

A review destination change does not automatically change the staircase integration context.

The core resolves any integration-context update separately and may require a rebase before publication or landing.

---

## 21. Multiple worktrees and active operations

### 21.1 Worktree independence

Gerrit associations belong to managed staircase structure, not one worktree.

Drafts, indexes, and active Git operations remain worktree-scoped.

### 21.2 Dirty worktree

Review planning may inspect committed staircase state while another worktree is dirty.

Review upload MUST refuse or use exact committed OIDs without disturbing draft state according to the core dirty-worktree rewrite rules.

It MUST NOT autostash unrelated work as a provider side effect.

### 21.3 Active external Git operation

If the source worktree is controlled by an external merge, rebase, cherry-pick, revert, or sequencer operation, the provider MUST NOT reinterpret its conflicted index as review content.

Remote mutation is blocked when exact source publication cannot be proven independent of that operation.

### 21.4 Active Staircase operation

Review upload and landing are blocked while a structural operation is unresolved unless the operation journal explicitly defines a publication phase and exact recoverable plan.

### 21.5 Concurrent provider commands

Commands mutating one staircase lineage or one confirmed Gerrit review identity MUST use provider and lineage operation locks sufficient to prevent conflicting local publication plans.

Remote leases and post-operation reconciliation remain required because another client may mutate Gerrit concurrently.

---
## 22. Configuration and provider diagnostics

### 22.1 Configuration scope

Automatically discovered Gerrit bindings and routes are user-local and non-versioned.

They MUST NOT be written into:

* Source-controlled files.
* Workspace manifests.
* `.repo` internal state.
* Provider-owned server configuration.
* Git remotes or refspecs without explicit user action.

### 22.2 Explicit configuration

Explicit configuration MAY establish:

```text
canonical or provisional server
project mapping
review destination
authentication profile
publication endpoint
REST endpoint
SSH endpoint
mapping default
verification profile mapping
landing policy
trusted endpoint aliases
```

Explicit configuration outranks automatic evidence and is not silently replaced.

### 22.3 Standalone repository

In a standalone Git repository, the provider MAY bind when local route evidence is unique and strong.

The lack of a workspace provider does not prevent Gerrit use.

The provider MUST still resolve project and destination explicitly or through strong route evidence.

### 22.4 Composition with a `repo` workspace

When a `repo` workspace provider supplies typed hints such as:

```text
Gerrit review endpoint
manifest project identity
destination branch
fetch or push route
```

the Gerrit provider independently validates and accepts or rejects them.

The resulting bindings remain conceptually:

```text
workspace             = repo
project-mapping       = repo
integration-context   = repo
review                = gerrit
review-identity       = gerrit
verification          = gerrit
review-transport      = gerrit or another bound provider
landing               = gerrit
```

The `repo` provider does not become a Gerrit provider, and Gerrit does not become the workspace provider.

### 22.5 Provider doctor

The canonical diagnostic command is:

```console
git staircase provider gerrit doctor
```

It reports:

```text
provider installation and protocol version
capability support and binding state
passive applicability evidence
rejected candidates and reasons
route completeness by current project
server identity state
project and destination provenance
authentication requirement without exposing credentials
transport readiness
query readiness
landing readiness
stale observations
supported mapping policies
server features known from prior validation
```

### 22.6 Doctor does not mutate

`provider gerrit doctor` is read-only unless an explicit refresh or authentication test option is selected.

Its default mode MUST NOT:

* Push.
* Create a change.
* Add a patch set.
* Submit or abandon a change.
* Install hooks.
* Change provider binding.
* Modify Git configuration.

### 22.7 Example doctor output

```text
Gerrit provider
  installation: installed
  protocol version: 1
  binding: bound-ready

Current project route
  server: review.example.com (confirmed)
  project: platform/payments
  review destination: refs/heads/main
  publication: ready
  query: ready
  landing: ready
  authentication: available through profile work

Capabilities
  review: ready
  review-identity: ready
  verification: ready
  review-transport: ready
  landing: ready

Mapping
  provider-native: per-commit
  per-step: supported with one-commit-per-step precondition
  aggregate: supported with one-commit precondition
```

### 22.8 Partial diagnostics

When route is incomplete, doctor MUST identify each missing field and its candidate sources.

Example:

```text
Gerrit provider
  binding: bound-incomplete
  server: review.example.com
  project: platform/payments
  review destination: unresolved

Candidates:
  refs/heads/main     from workspace destination hint
  refs/heads/release  from explicit branch review configuration

No destination selected.
```

---

## 23. Output and machine schemas

### 23.1 General output rules

The provider follows core output modes:

```text
human
porcelain
json
```

Machine-readable standard output MUST contain only the selected schema.

Bootstrap announcements, progress, and credential prompts MUST NOT contaminate JSON or porcelain output.

### 23.2 Full identifiers

JSON and porcelain output MUST use:

* Full Git OIDs.
* Full destination refs.
* Full Change-Ids.
* Canonical server identity when confirmed.
* Server-qualified change numbers.
* Full staircase lineage and step IDs.
* Full record and structure revision OIDs.

Human output MAY abbreviate OIDs only when unique in displayed scope.

### 23.3 Route schema

A Gerrit route object SHOULD be equivalent to:

```json
{
  "provider": "gerrit",
  "server": {
    "id": "review.example.com",
    "provisional": false,
    "endpoints": {
      "git": "ssh://review.example.com:29418/platform/payments",
      "rest": "https://review.example.com/"
    }
  },
  "project": "platform/payments",
  "review_destination": "refs/heads/main",
  "review_transport_ref": "refs/for/main",
  "authentication_profile": "work"
}
```

Sensitive endpoint credentials MUST be removed or redacted.

### 23.4 Pending review schema

```json
{
  "state": "not-uploaded",
  "pending_review_key": {
    "server_id": "review.example.com",
    "project": "platform/payments",
    "branch": "refs/heads/main",
    "change_id": "I1111111111111111111111111111111111111111"
  },
  "confirmed_review_identity": null,
  "exact_remote_revision": null
}
```

### 23.5 Confirmed review schema

```json
{
  "state": "current",
  "pending_review_key": {
    "server_id": "review.example.com",
    "project": "platform/payments",
    "branch": "refs/heads/main",
    "change_id": "I1111111111111111111111111111111111111111"
  },
  "confirmed_review_identity": {
    "provider": "gerrit",
    "server_id": "review.example.com",
    "change_number": 48101
  },
  "exact_remote_revision": {
    "commit_oid": "<full-commit-oid>",
    "patch_set_number": 3,
    "change_ref": "refs/changes/01/48101/3"
  }
}
```

### 23.6 Review plan schema

A JSON review plan MUST include:

```json
{
  "provider": "gerrit",
  "remote_queried": true,
  "record_revision_oid": "<full-record-oid>",
  "structure_revision_oid": "<full-structure-oid>",
  "integration_anchor_oid": "<full-commit-oid>",
  "route": {},
  "requested_mapping": "provider-native",
  "resolved_mapping": "per-commit",
  "topology": "stacked",
  "items": [
    {
      "step_id": "<uuid>",
      "step_ordinal": 1,
      "local_commit_oid": "<full-commit-oid>",
      "change_id": "I1111111111111111111111111111111111111111",
      "review_identity": null,
      "expected_remote_revision": null,
      "action": "create",
      "blocked_reasons": []
    }
  ],
  "remote_mutation_performed": false
}
```

### 23.7 Verification schema

Provider verification output MUST preserve individual evidence rather than only an aggregate Boolean.

A result item SHOULD include:

```json
{
  "review_identity": {
    "server_id": "review.example.com",
    "change_number": 48101
  },
  "exact_revision_oid": "<full-commit-oid>",
  "patch_set_number": 3,
  "labels": [],
  "checks": [],
  "submit_requirements": [],
  "mergeable": true,
  "submittable": true,
  "result": "passed",
  "observed_at": "<offset-aware-timestamp>"
}
```

### 23.8 Landing schema

Landing output MUST include:

```text
selected review identities and exact revisions
expected submission closure
old destination OID
new destination OID
actual merged changes
actual landing method when known
new provider-created commits
exact local structure transition
remaining active reviews
uncertain fields, if any
```

### 23.9 Human URLs

Human output MAY render canonical review URLs.

Machine identity MUST never depend on URL display form.

### 23.10 Control characters

All server-provided strings, commit subjects, reviewer names, label descriptions, and error messages are untrusted.

Terminal control characters MUST be escaped or stripped according to a documented safe rendering rule.

---

## 24. Diagnostics, error codes, and exit behavior

### 24.1 Stable core codes

The provider maps failures to stable core error codes including:

```text
provider-unbound
provider-route-incomplete
provider-authentication-unavailable
selection-changed
concurrent-record-update
remote-newer
remote-diverged
remote-outcome-unknown
verification-stale
landing-blocked
archived-mutation
operation-in-progress
active-git-operation
```

### 24.2 Gerrit detail codes

Provider-specific details SHOULD use namespaced codes including:

```text
gerrit.server-unconfirmed
gerrit.project-unresolved
gerrit.destination-unresolved
gerrit.destination-not-found
gerrit.change-id-missing
gerrit.change-id-malformed
gerrit.change-id-multiple
gerrit.change-id-duplicate
gerrit.review-not-found
gerrit.review-identity-ambiguous
gerrit.review-route-mismatch
gerrit.patch-set-mismatch
gerrit.change-abandoned
gerrit.change-merged
gerrit.change-work-in-progress
gerrit.submit-requirement-unsatisfied
gerrit.submit-closure-mismatch
gerrit.topic-contains-unrelated-changes
gerrit.transport-rejected
gerrit.response-unparseable
```

The stable core code remains authoritative for automation category.

### 24.3 Missing Change-Id example

```text
error: Gerrit review identity cannot be prepared for 1 commit

  step: 2
  commit: c516000
  detail: no valid Change-Id trailer
  code: gerrit.change-id-missing

No commits, refs, records, or remote reviews were changed.

Run an explicit normalization plan:
  git staircase normalize payments:2 --provider gerrit --ensure-change-ids
```

### 24.4 Remote-newer example

```text
error: Gerrit review 48102 has a newer remote patch set

  local associated commit: c522000
  last observed patch set: 4 c522000
  current remote patch set: 5 c599000
  synchronization: remote-newer

No upload performed.
Reconcile before publishing:
  git staircase review reconcile payments:3 --provider gerrit
```

### 24.5 Ambiguous server example

```text
error: Gerrit server is ambiguous

candidates:
  review-a.example.com/platform/payments refs/heads/main
  review-b.example.com/platform/payments refs/heads/main

Use the same command with one explicit server selector.
```

### 24.6 Uncertain outcome example

```text
error: Gerrit upload outcome is unknown

  planned commits: 3
  confirmed: 1
  unknown: 2
  operation journal: review-upload 7a6b13d2
  code: remote-outcome-unknown

Do not repeat the upload blindly.
Run:
  git staircase review reconcile payments --provider gerrit
```

### 24.7 Exit statuses

The provider follows core exit-status classes:

```text
0  success or defined idempotent no-op
1  usage or validation failure
2  selection or ambiguity failure
3  concurrent state or lease failure
4  operation conflict requiring resolution
5  provider, network, or authentication failure
6  verification or landing policy failure
7  internal integrity failure
```

Machine-readable error codes are more specific than exit status.

### 24.8 No mutation on validation failure

Route ambiguity, Change-Id error, mapping failure, stale record, remote-newer state, or unsupported topology MUST fail before remote mutation.

### 24.9 Server rejection

A Gerrit rejection is reported with:

```text
stable core code
namespaced provider detail code
route
exact source OID
server response safely rendered
known per-item outcome
reconciliation requirement
```

---

## 25. Security and trust

### 25.1 Credentials

Credentials MUST be obtained through trusted user or system mechanisms.

They MUST NOT be:

* Stored in staircase records.
* Printed in diagnostics.
* Included in operation journals without encryption and explicit policy.
* Copied from unrelated command output.
* Read during passive discovery.

### 25.2 Endpoint parsing

Git, SSH, HTTP, and HTTPS endpoints MUST be parsed structurally.

The provider MUST NOT use fragile string slicing to derive project or host identity.

User information, ports, percent encoding, IPv6 literals, paths, and schemes require correct parsing.

### 25.3 Shell safety

Repository, server, project, branch, topic, reviewer, hashtag, and push-option values MUST NOT be interpolated into shell source.

Commands use argument arrays or equivalent safe APIs.

### 25.4 Ref safety

Destination and transport refs MUST be validated as appropriate Git refs.

The provider MUST prevent newline, NUL, option-injection, and refspec-injection attacks.

### 25.5 REST responses

REST responses are untrusted and bounded.

The provider MUST validate schema, types, sizes, identity fields, and pagination behavior before using results.

Server-provided HTML or Markdown MUST NOT be rendered as trusted terminal control content.

### 25.6 SSH responses

SSH command output is untrusted structured or text input.

The provider MUST use a documented parser and reject malformed identity or status fields rather than guessing.

### 25.7 Push output

Receive-pack output and helper output are untrusted.

A success exit code does not replace identity reconciliation.

### 25.8 Hooks

A configured Change-Id hook is executable code.

Only trusted hooks selected by explicit policy may be run by provider operations.

Passive bootstrap never executes hooks.

### 25.9 URLs

Review URLs are display and navigation locators.

The provider validates scheme and server scope before opening them.

A URL is not accepted as identity until parsed and resolved to one configured or explicitly approved Gerrit installation.

### 25.10 Secrets in machine output

Machine output MUST redact:

```text
passwords
access tokens
cookies
private keys
Authorization headers
credential-bearing URLs
```

### 25.11 Network boundaries

Every command must make its network behavior predictable:

* Passive probe: no network.
* Local-only plan: no network.
* Remote-aware plan: network allowed and reported.
* Review status and verification: network allowed unless cached mode selected.
* Upload and landing: network mutation explicitly requested.

---

## 26. Canonical Gerrit command surface

### 26.1 Core commands specialized by Gerrit

```console
git staircase review plan <selector> --provider gerrit
git staircase review create <selector> --provider gerrit
git staircase review upload <selector> --provider gerrit
git staircase review status <selector> --provider gerrit
git staircase review show <selector> --provider gerrit
git staircase review open <step-selector> --provider gerrit
git staircase review reconcile <selector> --provider gerrit
git staircase review attach <selector> --provider gerrit --review <review-selector>
git staircase review detach <selector> --provider gerrit [--review <review-selector>]
git staircase verify <selector> --provider gerrit
git staircase land <selector> --provider gerrit --stepwise
git staircase land <selector> --provider gerrit --through <step-selector>
git staircase land <selector> --provider gerrit --aggregate
git staircase normalize <selector> --provider gerrit --ensure-change-ids
git staircase provider gerrit doctor
```

### 26.2 Route options

A conforming implementation SHOULD support explicit route inputs equivalent to:

```text
--server <server-selector>
--project <gerrit-project>
--destination <branch-or-full-ref>
```

Accepted short branch input is normalized immediately to a full destination ref.

### 26.3 Mapping option

```console
--mapping per-commit
--mapping per-step
--mapping aggregate
--mapping provider-native
```

Unsupported mapping values fail during planning.

### 26.4 Typed publication options

Implementations MAY expose typed options for:

```text
topic
reviewer
CC recipient
hashtag
work-in-progress or ready state
private state
notification policy
```

These options affect the publication plan and MUST be shown before mutation when consequential.

### 26.5 Raw push options

A provider MAY expose an advanced raw push-option escape hatch only when:

* It is explicitly named as unsafe or advanced.
* Values are passed without shell interpolation.
* Identity-affecting options are parsed and reflected in the plan.
* Unknown effects prevent claims of complete reconciliation.

It is not required for conformance.

### 26.6 Noncanonical aliases

The provider MUST NOT introduce these aliases:

```text
git staircase upload              for review upload
git staircase submit              for land
git staircase gerrit upload       for review upload
git staircase gerrit submit       for land
git staircase gerrit status       for review status
git staircase review push         for review upload
```

Provider selection is expressed by capability binding or `--provider gerrit`, not by a parallel top-level command tree.

### 26.7 Ordinary Git remains available

Users MAY continue to use ordinary `git commit`, `git rebase`, and `git push`.

When external Git or Gerrit mutation changes associated state, Staircase detects and reconciles it rather than claiming exclusive ownership of Git or Gerrit.

---

## 27. Normative invariants

A conforming Gerrit provider preserves all of the following.

### 27.1 Provider boundaries

1. Gerrit is not a workspace provider.
2. Workspace hints do not bind Gerrit without an independent provider decision.
3. Integration context, review destination, transport ref, and landing destination remain distinct.
4. Passive discovery is offline and nonmutating.

### 27.2 Identity

1. Change-Id alone is not review identity.
2. Bare change number is not globally unique.
3. Pending review key includes server, project, branch, and Change-Id.
4. Confirmed review identity includes canonical server identity and server-issued change number.
5. Exact review revision includes full patch-set commit OID.
6. Patch-set number does not replace exact commit OID.
7. Change refs identify patch sets, not continuing reviews.

### 27.3 Mapping

1. Review mapping is explicit.
2. Gerrit provider-native mapping resolves to per-commit.
3. A step may map to zero, one, or several Gerrit changes.
4. Per-step requires exactly one review commit per active step.
5. Aggregate requires exactly one active review commit unless an explicit extension says otherwise.
6. Unsupported merge or cumulative topology is never mislabeled as an ordinary stack.

### 27.4 Commit metadata

1. Change-Ids are parsed as trailers.
2. Multiple or duplicate Change-Ids are never chosen silently.
3. Upload planning never rewrites commits.
4. Change-Id insertion is an explicit normalization operation.
5. Split never duplicates one Change-Id across new review commits.
6. Join never silently reassigns retired remote reviews.

### 27.5 Persistence and concurrency

1. Durable associations require management.
2. Persistent provider mutations compare the complete record revision.
3. One logical operation reads one immutable record.
4. Cached observations do not silently redefine local structure.
5. Remote success followed by local CAS failure is reconciled, not discarded.
6. Archived staircases retain associations without remote mutation.

### 27.6 Publication

1. Upload plans use exact immutable commit OIDs.
2. Local refs and records are protected by leases.
3. Remote patch sets are checked as leases when known.
4. Remote-newer state blocks default upload.
5. Successful transport exit does not replace per-item identity reconciliation.
6. Unknown outcomes are not retried blindly.
7. `review upload`, not `push`, publishes Gerrit reviews.

### 27.7 Verification

1. Gerrit evidence applies to exact patch-set subjects.
2. Labels, checks, mergeability, submit requirements, and submittability remain distinct.
3. Copied votes retain provenance.
4. Older patch-set evidence does not verify a rewritten local commit.
5. Provider evidence and local execution evidence remain distinct.

### 27.8 Landing

1. Lower dependencies land before upper dependencies.
2. Aggregate landing proves the exact submission closure.
3. Whole-topic submission never includes unrelated changes silently.
4. Actual destination graph, not planned method, determines reconciliation.
5. Uncertain submission is reconciled before retry.
6. External submission is detected and reconciled.

### 27.9 Security and output

1. Passive probe does not authenticate.
2. Credentials never enter staircase records or ordinary output.
3. Endpoints, refs, and push options are structurally validated.
4. Server and commit text are rendered safely.
5. Machine output uses full identities and stable error codes.

---

## 28. Conformance scenarios

The following scenarios are normative examples. OIDs are abbreviated only for readability.

### 28.1 Passive composition in a `repo` workspace

Given typed local workspace facts:

```text
workspace provider: repo
project: platform/payments
integration anchor: a100000
review endpoint hint: review.example.com
review destination hint: refs/heads/main
```

and a unique Gerrit route, the first command may report:

```console
$ git staircase list
[stderr] Configured Staircase workspace:
[stderr]   workspace:            repo
[stderr]   project:              platform/payments
[stderr]   integration-context:  repo
[stderr]   review:               gerrit
[stderr]   verification:         gerrit
[stderr]   landing:              gerrit
[stderr]
[stdout] payments  3 steps  clean  sequential  (implicit)
```

No network request, authentication test, hook execution, ref update, or manifest mutation occurs.

### 28.2 Prepare and upload three reviews

Given a managed three-commit staircase with three unique Change-Ids:

```console
$ git staircase review create payments --provider gerrit
Prepared 3 review identities for 'payments'.
  step 1: I1111111111111111111111111111111111111111  pending first upload
  step 2: I2222222222222222222222222222222222222222  pending first upload
  step 3: I3333333333333333333333333333333333333333  pending first upload

record revision: 88ad41c -> 9bf5a72
Remote publication is still required.
```

Then:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.
  destination: review.example.com/platform/payments refs/heads/main

  step  commit    change  patch set  result
  1     b110000   48101   1          created
  2     c120000   48102   1          created
  3     d130000   48103   1          created

Associated 3 reviews.
```

Every confirmed change number is stored with the server identity, route attributes, Change-Id, and exact patch-set commit OID.

### 28.3 Rebase preserves review identity but stales exact evidence

After the staircase is rebased, local review commits change from:

```text
b110000 c120000 d130000
```

to:

```text
b310000 c320000 d330000
```

while Change-Ids remain unchanged.

Expected status before upload:

```text
step  review  local     remote    sync         verification
1     48101   b310000   b110000   local-newer  stale
2     48102   c320000   c120000   local-newer  stale
3     48103   d330000   d130000   local-newer  stale
```

Confirmed review identities remain unchanged. Exact review revisions and verification evidence do not.

### 28.4 Remote-newer patch set blocks upload

If another client uploads patch set 5 for change 48102 after Staircase last observed patch set 4, default upload produces:

```text
error: Gerrit review 48102 has a newer remote patch set
  synchronization: remote-newer
  last observed: patch set 4 c522000
  current remote: patch set 5 c599000

No upload performed.
Run:
  git staircase review reconcile payments:3 --provider gerrit
```

No local record or remote review is changed.

### 28.5 Unknown upload outcome

If transport fails after the server may have accepted some commits:

```text
error: Gerrit upload outcome is unknown
  confirmed: 1
  unknown: 2
  state: upload-unknown

Do not repeat the upload blindly.
Run:
  git staircase review reconcile payments --provider gerrit
```

Reconciliation queries by confirmed identity or complete pending review key and records each exact result.

### 28.6 Split one reviewed step

When a reviewed commit is split:

```text
old step: Change-Id I2222... confirmed review 48102
new lower step: new Change-Id I4444... no confirmed review
surviving upper step: Change-Id I2222... confirmed review 48102
```

The original Change-Id is not duplicated.

The new review is created explicitly, and the old review remains with the surviving subject.

### 28.7 Provider verification

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'
  provider: gerrit
  structure revision: 92f55fd

  step  review  exact revision  review       checks  submit requirements
  1     48101   b511000         approved     passed  satisfied
  2     48144   c516000         approved     passed  satisfied
  3     48102   c522000         approved     passed  satisfied
  4     48103   d532000         approved     passed  satisfied

Result: passed for the exact current review revisions.
```

A later patch set makes the corresponding prior item stale until refreshed.

### 28.8 Stepwise landing

For changes `48101 -> 48144 -> 48102 -> 48103`, stepwise landing submits `48101` first.

After confirmation, the provider and core resolve the new destination OID and determine whether upper commits remain valid.

If Gerrit rebases or cherry-picks the lower change, upper local commits are restacked before their existing reviews receive new patch sets.

The next change is not submitted until its exact current patch set again satisfies policy.

### 28.9 Whole-topic safety

If topic `payments-redesign` contains the four staircase changes plus unrelated change `49000`, aggregate whole-topic landing fails:

```text
error: Gerrit topic contains changes outside the selected landing set

selected:
  48101 48144 48102 48103
additional topic members:
  49000

No changes submitted.
code: landing-blocked
detail: gerrit.topic-contains-unrelated-changes
```

### 28.10 Archive is local

```console
$ git staircase archive payments
Archived 'payments'.
```

The command moves owned local refs according to the core archive contract and preserves Gerrit associations.

It does not abandon changes `48101`, `48144`, `48102`, or `48103` and performs no network request by default.

---

## 29. Summary definition

The Gerrit provider maps exact committed staircase subjects to Gerrit changes while preserving the distinctions among:

```text
staircase lineage
stable step identity
pending Change-Id route key
confirmed Gerrit review identity
exact Gerrit patch-set revision
review destination
integration context
provider verification evidence
landing destination and actual resulting graph
```

Its governing rules are:

1. Discover passively and bind only from trusted, unambiguous local evidence.
2. Keep workspace, integration, review, transport, verification, and landing capabilities separate.
3. Treat Gerrit's native mapping as one review per commit unless an explicit policy says otherwise.
4. Never rewrite commits during planning or upload.
5. Plan publication from immutable OIDs and protect local and remote state with leases.
6. Preserve server-confirmed review identity across patch sets while applying evidence only to exact patch-set commits.
7. Reconcile every uncertain or externally changed remote outcome.
8. Submit dependent reviews only in a proven order and inspect the actual destination graph afterward.
9. Persist durable associations through the consolidated staircase record and full-record compare-and-swap.
10. Never let provider state silently redefine local staircase structure.

---

# Appendix A: User journeys

## A.1 Status and transcript conventions

This appendix is normative for:

* Canonical command names.
* Provider and core operation boundaries.
* The distinctions among pending review keys, confirmed review identities, and exact patch-set revisions.
* The material facts that human output MUST communicate.
* Failure behavior, recovery instructions, and the absence or presence of remote mutation.

Concrete OIDs, UUIDs, change numbers, patch-set numbers, paths, hosts, durations, and timestamps are illustrative. Human output MAY use different spacing or unique OID abbreviations, but it MUST preserve every material fact shown. Machine output remains governed by Section 23 and MUST use full identifiers.

Lines beginning with `$` are commands. Commands not beginning with `git staircase` are ordinary Git or workspace-manager commands and are labeled by context.

Unless stated otherwise, the examples use:

```text
Gerrit server:       review.example.com
Gerrit project:      platform/payments
review destination:  refs/heads/main
transport ref:       refs/for/main
mapping:             per-commit
```

The same staircase name, `payments`, is reused across journeys for readability. Each journey is independent unless it explicitly says otherwise.

---

## A.2 Journey 1: Prepare and publish a new Gerrit review stack

### A.2.1 Starting state

A standalone Git repository has an integration anchor at `a100000` and a three-step sequential staircase:

```text
payments-1 -> b110000  Add ledger model
payments-2 -> c120000  Route writes through ledger
payments   -> d130000  Add migration and tests
```

The first and third commits have valid Change-Id trailers. The second does not. The repository has an explicit Gerrit route, but the staircase is still implicit.

```console
$ git staircase list
payments  3 steps  clean  sequential  (implicit)
```

The developer adopts it because durable review associations are required:

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

### A.2.2 Planning detects a missing Change-Id without mutation

```console
$ git staircase review plan payments --provider gerrit --mapping provider-native
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a100000
  structure revision: 241ea87
  requested mapping: provider-native
  resolved mapping: per-commit
  topology: stacked
  remote queried: yes

  step  commit    Change-Id                                  action
  1     b110000   I1111111111111111111111111111111111111111 create
  2     c120000   missing                                    blocked
  3     d130000   I3333333333333333333333333333333333333333 create

Blocked: 1 commit requires normalization.
No commits, refs, records, or remote reviews were changed.

Run:
  git staircase normalize payments:2 --provider gerrit --ensure-change-ids
```

Attempting creation fails for the same reason and does not partially record the two valid identities:

```console
$ git staircase review create payments --provider gerrit
error: Gerrit review identity cannot be prepared for 1 commit

  step: 2
  commit: c120000
  detail: no valid Change-Id trailer
  code: gerrit.change-id-missing

No commits, refs, records, or remote reviews were changed.

Run an explicit normalization plan:
  git staircase normalize payments:2 --provider gerrit --ensure-change-ids
```

### A.2.3 Explicit normalization rewrites only the required commit and its descendants

The configured trusted Change-Id generator produces a new trailer for step 2. Because step 3 depends on step 2, it is restacked. The operation preserves stable step IDs and primary-branch ownership.

```console
$ git staircase normalize payments:2 --provider gerrit --ensure-change-ids
Normalized Gerrit review identity for 'payments' step 2.

  step  old commit  new commit  Change-Id
  2     c120000     c121000     I2222222222222222222222222222222222222222

Restacked dependent commits:
  step  old commit  new commit
  3     d130000     d131000

  primary branches updated: 2
  review associations changed: none
  verification: stale
  structure revision: 241ea87 -> 2f6d9c1
  record revision: 88ad41c -> 91c01bb
```

A new plan is now fully actionable:

```console
$ git staircase review plan payments --provider gerrit --mapping provider-native
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  integration anchor: a100000
  structure revision: 2f6d9c1
  requested mapping: provider-native
  resolved mapping: per-commit
  topology: stacked
  remote queried: yes

  step  commit    Change-Id                                  action
  1     b110000   I1111111111111111111111111111111111111111 create
  2     c121000   I2222222222222222222222222222222222222222 create
  3     d131000   I3333333333333333333333333333333333333333 create

Expected remote mutations: 3 review creations.
No remote mutation performed.
```

### A.2.4 Creation records pending keys; upload confirms server identities

```console
$ git staircase review create payments --provider gerrit
Prepared 3 review identities for 'payments'.
  step 1: I1111111111111111111111111111111111111111  pending first upload
  step 2: I2222222222222222222222222222222222222222  pending first upload
  step 3: I3333333333333333333333333333333333333333  pending first upload

record revision: 91c01bb -> 9bf5a72
Remote publication is still required.
```

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.
  destination: review.example.com/platform/payments refs/heads/main
  structure revision: 2f6d9c1

  step  commit    change  patch set  result
  1     b110000   48101   1          created
  2     c121000   48102   1          created
  3     d131000   48103   1          created

Associated 3 reviews.
record revision: 9bf5a72 -> 31f477a
```

The status command distinguishes stable review identity from exact patch-set revision:

```console
$ git staircase review status payments --provider gerrit
payments
  provider: gerrit
  destination: platform/payments refs/heads/main

  step  local     review  patch set  remote revision  synchronization  review state
  1     b110000   48101   1          b110000          current          needs review
  2     c121000   48102   1          c121000          current          needs review
  3     d131000   48103   1          d131000          current          needs review
```

---

## A.3 Journey 2: A difficult `repo` and Gerrit review cycle

### A.3.1 Passive composition in a detached `repo` workspace

A developer is in project `platform/payments` inside a `repo` workspace. The project was checked out detached at manifest commit `a100000`. Three local branches form a sequential staircase, and each step contains one valid Gerrit review commit.

The first Staircase command passively binds the workspace, integration-context, review, and verification capabilities, then continues the requested operation:

```console
$ git staircase list
Configured Staircase workspace:
  workspace:            repo
  project:              platform/payments
  integration-context:  repo
  review:               gerrit
  verification:         gerrit

payments  3 steps  clean  sequential  (implicit)
```

The developer adopts, creates pending keys, and uploads the initial stack:

```console
$ git staircase adopt payments
Adopted 'payments'.
  lineage: 7c1f287e-0b7b-48ec-8901-758c73030f5c
  steps: 3
  structure revision: 241ea87
  record revision: 88ad41c
```

```console
$ git staircase review create payments --provider gerrit
Prepared 3 review identities for 'payments'.
  step 1: I1111111111111111111111111111111111111111  pending first upload
  step 2: I2222222222222222222222222222222222222222  pending first upload
  step 3: I3333333333333333333333333333333333333333  pending first upload

record revision: 88ad41c -> 9bf5a72
Remote publication is still required.
```

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b110000   48101   1          created
  2     c120000   48102   1          created
  3     d130000   48103   1          created

record revision: 9bf5a72 -> 31f477a
```

### A.3.2 Review round one: amend the bottom review and resolve conflicts in two upper reviews

Reviewers request a schema change in step 1. In a worktree checked out at `payments-1`, the developer attaches rewrite intent and stages only the intended files:

```console
$ git staircase draft attach payments:1 --mode=rewrite-step
Attached current worktree draft.
  staircase: payments
  step: 1
  intent: rewrite-step
  expected basis: b110000
```

```console
$ git add ledger/schema.proto ledger/model.cc
$ git staircase draft materialize payments:1 --amend
Materialized staged draft into step 1.
  old cut: b110000
  new cut: b211000
  Change-Id preserved: I1111111111111111111111111111111111111111
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

The conflict belongs to the active Staircase operation. Another structural mutation cannot begin:

```console
$ git staircase reorder payments --steps 1,3,2
error: Staircase operation 53e2f58d is active
  kind: restack
  state: conflict
  code: operation-in-progress
  next: git staircase continue
  abort: git staircase abort
```

The first resolution advances step 2, then step 3 conflicts independently:

```console
$ git add ledger/writer.cc
$ git staircase continue
Resolved step 2.
  old cut: c120000
  new cut: c221000
  Change-Id preserved: I2222222222222222222222222222222222222222

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

```console
$ git add ledger/migration_test.cc
$ git staircase continue
Restack complete for 'payments'.

  step  old cut   new cut
  1     b110000   b211000
  2     c120000   c221000
  3     d130000   d231000

  primary branches updated: 3
  review synchronization: local-newer for 3 reviews
  verification: stale
  structure revision: 241ea87 -> 17ef0c1
  record revision: 31f477a -> d0834ba
```

All three stable Gerrit identities survive. Upload creates patch set 2 for each review:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b211000   48101   2          updated
  2     c221000   48102   2          updated
  3     d231000   48103   2          updated

record revision: d0834ba -> f5be6f2
```

### A.3.3 Review round two: rebase onto a newer manifest anchor with conflicts at nonadjacent steps

The workspace integration candidate advances to `a300000`:

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

The whole staircase rebase conflicts in step 1:

```console
$ git staircase rebase payments
Rebasing 'payments'.
  old integration anchor: a100000
  new integration anchor: a300000

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

After step 1 is resolved, step 2 applies cleanly and step 3 conflicts:

```console
$ git add ledger/schema.proto
$ git staircase continue
Resolved step 1.
  new cut: b311000
  Change-Id preserved: I1111111111111111111111111111111111111111

  step 2 of 3: applied
  new cut: c321000
  Change-Id preserved: I2222222222222222222222222222222222222222

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

```console
$ git add ledger/migration.cc ledger/migration_test.cc
$ git staircase continue
Rebase complete for 'payments'.
  integration anchor: a100000 -> a300000

  step  old cut   new cut
  1     b211000   b311000
  2     c221000   c321000
  3     d231000   d331000

  review synchronization: local-newer for 3 reviews
  verification: stale
  structure revision: 17ef0c1 -> 8eb860d
  record revision: f5be6f2 -> bfa8427
```

Local prefix verification and Gerrit upload remain separate operations:

```console
$ git staircase verify payments --each-prefix --profile presubmit
Verification profile: presubmit
  integration anchor: a300000
  structure revision: 8eb860d

  prefix  subject   result
  1       b311000   passed
  2       c321000   passed
  3       d331000   passed

Result: passed
Evidence recorded for 3 exact local subjects.
```

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b311000   48101   3          updated
  2     c321000   48102   3          updated
  3     d331000   48103   3          updated
```

### A.3.4 A `repo sync` conflict remains owned by the external Git rebase

The developer later runs `repo sync` in the baseline worktree. `repo` starts an ordinary Git rebase for a baseline-only local commit and stops on a conflict:

```console
$ repo sync platform/payments
error: could not apply e400000... Preserve local test fixture
CONFLICT (content): Merge conflict in testdata/accounts.json
```

Staircase reports the external operation but does not reinterpret it as a staircase rebase:

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

The developer resolves it with ordinary Git:

```console
$ git add testdata/accounts.json
$ git rebase --continue
Successfully rebased and updated detached HEAD.
$ repo sync platform/payments
Fetching: 100% (1/1), done in 0.842s
```

The refreshed workspace anchor is now `a500000`. `repo sync` did not rewrite staircase branches:

```console
$ git staircase workspace refresh
Refreshed workspace context.
  project: platform/payments
  previous integration anchor: a300000
  current integration anchor: a500000

Affected active staircases:
  payments  behind integration anchor  verification stale
```

The later Staircase rebase conflicts only in the middle step:

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

```console
$ git add ledger/writer.cc
$ git staircase continue
Resolved step 2.
  new cut: c521000

  step 3 of 3: applied
  new cut: d531000

Rebase complete for 'payments'.
  integration anchor: a300000 -> a500000
  review synchronization: local-newer for 3 reviews
  verification: stale
  structure revision: 8eb860d -> f8c365a
```

The fourth Gerrit patch sets preserve all three confirmed review identities:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b511000   48101   4          updated
  2     c521000   48102   4          updated
  3     d531000   48103   4          updated
```

---

## A.4 Journey 3: Split one reviewed subject without duplicating its review identity

A reviewer asks that the API transition and call-site migration in step 2 become separate reviews. Step 2 currently contains two commits and is associated with Gerrit change `48102` through Change-Id `I2222...`.

The developer inserts a cut at `c515000`:

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

  existing review 48102 remains with the surviving upper subject
  new lower subject has no review identity
  structure revision: f8c365a -> 8b02a65
```

The original Change-Id appears on only the surviving upper review commit. The new lower commit initially lacks a Change-Id:

```console
$ git staircase review plan payments --provider gerrit --mapping per-commit
Review publication plan for 'payments'
  provider: gerrit
  destination: review.example.com/platform/payments refs/heads/main
  structure revision: 8b02a65

  step  commit    review  Change-Id                                  action
  1     b511000   48101   I1111111111111111111111111111111111111111 no-op
  2     c515000   none    missing                                    blocked
  3     c521000   48102   I2222222222222222222222222222222222222222 update
  4     d531000   48103   I3333333333333333333333333333333333333333 update

Blocked: step 2 requires a distinct Change-Id.
No remote mutation performed.
```

Explicit normalization gives only the new lower review commit a new Change-Id and restacks its descendants:

```console
$ git staircase normalize payments:2 --provider gerrit --ensure-change-ids
Normalized Gerrit review identity for 'payments' step 2.

  step  old commit  new commit  Change-Id
  2     c515000     c516000     I4444444444444444444444444444444444444444

Restacked dependent commits:
  step  old commit  new commit  retained review
  3     c521000     c522000     48102
  4     d531000     d532000     48103

  existing Change-Id I2222222222222222222222222222222222222222 was not duplicated
  structure revision: 8b02a65 -> 92f55fd
```

Creation prepares exactly one new pending key:

```console
$ git staircase review create payments:2 --provider gerrit
Prepared review identity for 'payments' step 2.
  Change-Id: I4444444444444444444444444444444444444444
  state: pending first upload

record revision: 6f8314a -> 7714a91
Remote publication is still required.
```

Upload creates one review and updates only rewritten existing subjects:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b511000   48101   4          no-op
  2     c516000   48144   1          created
  3     c522000   48102   5          updated
  4     d532000   48103   5          updated

Associated 1 new review.
```

---

## A.5 Journey 4: Reconcile a patch set uploaded outside Staircase

A collaborator uploads patch set 6 to Gerrit change `48102` from another machine. The local association last observed patch set 5 at `c522000`; Gerrit now reports `c599000`.

Status exposes the mismatch:

```console
$ git staircase review status payments --provider gerrit
payments
  provider: gerrit

  step  local     review  last observed  current remote  synchronization
  1     b511000   48101   4 b511000      4 b511000       current
  2     c516000   48144   1 c516000      1 c516000       current
  3     c522000   48102   5 c522000      6 c599000       remote-newer
  4     d532000   48103   5 d532000      5 d532000       current

Result: attention required.
```

A normal upload is blocked before transport:

```console
$ git staircase review upload payments --provider gerrit
error: Gerrit review 48102 has a newer remote patch set

  local associated commit: c522000
  last observed patch set: 5 c522000
  current remote patch set: 6 c599000
  synchronization: remote-newer
  code: remote-newer

No upload performed.
Reconcile before publishing:
  git staircase review reconcile payments:3 --provider gerrit
```

Reconciliation records the remote observation without changing local commits:

```console
$ git staircase review reconcile payments:3 --provider gerrit
Reconciled Gerrit review state for 'payments' step 3.
  review: 48102
  previous observation: patch set 5 c522000
  current observation: patch set 6 c599000
  synchronization: remote-newer

Local staircase structure was not changed.
Remote review was not changed.
record revision: 7714a91 -> 83e6d42
```

The developer decides the collaborator's patch set should become the local step subject. They fetch the exact patch-set ref with ordinary Git and move the owned primary branch explicitly:

```console
$ git fetch review refs/changes/02/48102/6
From ssh://review.example.com:29418/platform/payments
 * branch            refs/changes/02/48102/6 -> FETCH_HEAD
$ git switch payments-3
Switched to branch 'payments-3'
$ git reset --hard FETCH_HEAD
HEAD is now at c599000 Route writes through ledger
```

Staircase detects that the managed lower subject changed externally and that step 4 is now stale:

```console
$ git staircase status payments
payments
  state: stale
  changed outside Staircase: step 3
  step 3 cut: c522000 -> c599000
  Gerrit review 48102 patch set 6 matches the new cut
  stale from: step 4
  next: git staircase restack payments --from payments:3
```

Restacking preserves change `48103` but rewrites its exact local revision:

```console
$ git staircase restack payments --from payments:3
Restack complete for 'payments'.
  step 3: adopted externally selected cut c599000
  step 4: d532000 -> d633000
  review 48102: current at patch set 6
  review 48103: local-newer
  verification: stale
  state: clean
```

Only step 4 requires publication:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b511000   48101   4          no-op
  2     c516000   48144   1          no-op
  3     c599000   48102   6          no-op
  4     d633000   48103   6          updated
```

---

## A.6 Journey 5: Recover from an upload whose network outcome is unknown

A four-review upload loses the network connection after Gerrit may have accepted some commits:

```console
$ git staircase review upload payments --provider gerrit
error: Gerrit upload outcome is unknown

  planned commits: 4
  confirmed: 1
  unknown: 3
  operation journal: review-upload 7a6b13d2
  code: remote-outcome-unknown

Do not repeat the upload blindly.
Run:
  git staircase review reconcile payments --provider gerrit
```

The staircase record retains the immutable plan and marks unresolved items `upload-unknown`. A second direct upload remains blocked:

```console
$ git staircase review upload payments --provider gerrit
error: unresolved Gerrit upload operation exists
  operation journal: review-upload 7a6b13d2
  state: upload-unknown
  code: remote-outcome-unknown

Run:
  git staircase review reconcile payments --provider gerrit
```

Reconciliation queries each confirmed identity or complete pending review key and produces deterministic per-item results:

```console
$ git staircase review reconcile payments --provider gerrit
Reconciled Gerrit upload operation 7a6b13d2.

  step  commit    change  patch set  reconciled result
  1     b711000   48101   7          accepted
  2     c716000   48144   2          accepted
  3     c722000   48102   7          accepted
  4     d732000   48103   6          not accepted

  remote review mutations performed: none
  local commits changed: none
  unresolved items: 0
  step 4 synchronization: local-newer
  record revision: 91ae720 -> a003cc2
```

Retrying is now safe because the first three items are exact no-ops and only step 4 remains local-newer:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b711000   48101   7          no-op
  2     c716000   48144   2          no-op
  3     c722000   48102   7          no-op
  4     d732000   48103   7          updated
```

---

## A.7 Journey 6: Provider verification applies only to exact patch-set revisions

All four reviews are approved and their current patch sets pass required checks and submit requirements:

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'
  provider: gerrit
  structure revision: 92f55fd

  step  review  patch set  exact revision  code review  presubmit  submit requirements
  1     48101   7          b711000         approved     passed     satisfied
  2     48144   2          c716000         approved     passed     satisfied
  3     48102   7          c722000         approved     passed     satisfied
  4     48103   7          d732000         approved     passed     satisfied

Result: passed for the exact current review revisions.
```

A reviewer then asks for a message and test adjustment in step 4. The developer amends the local commit while retaining its Change-Id:

```console
$ git switch payments
Switched to branch 'payments'
$ git commit --amend
[payments d733000] Add migration and tests
```

Status distinguishes stable review identity from exact revision drift:

```console
$ git staircase review status payments --provider gerrit
payments
  provider: gerrit

  step  local     review  remote revision  synchronization  verification
  1     b711000   48101   b711000          current          passed
  2     c716000   48144   c716000          current          passed
  3     c722000   48102   c722000          current          passed
  4     d733000   48103   d732000          local-newer      stale
```

Provider verification cannot reuse patch-set 7 evidence for `d733000`:

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'
  provider: gerrit

  step  review  local revision  remote revision  result
  1     48101   b711000         b711000          passed
  2     48144   c716000         c716000          passed
  3     48102   c722000         c722000          passed
  4     48103   d733000         d732000          stale

Result: stale
code: verification-stale

Upload the exact local revision and wait for its required Gerrit evidence.
```

After upload, patch set 8 exists but checks are still running:

```console
$ git staircase review upload payments --provider gerrit
Uploaded 'payments' to Gerrit.

  step  commit    change  patch set  result
  1     b711000   48101   7          no-op
  2     c716000   48144   2          no-op
  3     c722000   48102   7          no-op
  4     d733000   48103   8          updated
```

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'

  step  review  patch set  exact revision  code review  presubmit  submit requirements
  1     48101   7          b711000         approved     passed     satisfied
  2     48144   2          c716000         approved     passed     satisfied
  3     48102   7          c722000         approved     passed     satisfied
  4     48103   8          d733000         needs vote   running    unsatisfied

Result: pending
```

A later invocation may return `passed` only after the exact patch set 8 satisfies every configured requirement.

---

## A.8 Journey 7: Stepwise landing with a Gerrit-created commit and an upper restack

All four exact review revisions are approved and submittable. Gerrit is configured to cherry-pick submitted commits onto the destination branch. The developer requests bottom-to-top landing:

```console
$ git staircase land payments --provider gerrit --stepwise
Landing plan for 'payments'
  provider: gerrit
  destination: platform/payments refs/heads/main
  old destination: a700000
  mode: stepwise

  order  review  patch set  exact revision  action
  1      48101   7          b711000         submit
  2      48144   2          c716000         submit after lower integration
  3      48102   7          c722000         submit after lower integration
  4      48103   8          d733000         submit after lower integration

Submitting review 48101...
Submitted review 48101.
  Gerrit strategy: cherry-pick
  reviewed revision: b711000
  destination commit: a711100
  destination: a700000 -> a711100

Restacking remaining active reviews onto a711100...
  step 2 of 4: conflict
  source cut: c716000
  conflicted paths:
    ledger/model.cc

Operation: land 6fe713c4
State: conflict while restacking remaining reviews
Resolve conflicts, stage the resolutions, then run:
  git staircase continue
To stop before any further submissions and preserve the already merged result, run:
  git staircase abort
```

`abort` cannot undo the already merged Gerrit change. Its contract is to stop further submission and restore the best valid local representation of confirmed remote state.

The developer resolves the conflict and continues:

```console
$ git add ledger/model.cc
$ git staircase continue
Resolved remaining step 2.

Restack complete for remaining active staircase.
  former step 2: c716000 -> c812000
  former step 3: c722000 -> c822000
  former step 4: d733000 -> d833000

Existing review identities preserved:
  48144  local-newer
  48102  local-newer
  48103  local-newer

Renumbered remaining primary branches:
  payments-2 -> payments-1
  payments-3 -> payments-2
  payments   -> payments

Uploaded rewritten upper reviews:
  review 48144: patch set 3 c812000
  review 48102: patch set 8 c822000
  review 48103: patch set 9 d833000

Landing paused before the next submission.
Reason: required Gerrit verification is pending for new exact revisions.
  next review: 48144
  next: git staircase verify payments --provider gerrit
  resume: git staircase land payments --provider gerrit --stepwise
```

After the new patch sets satisfy policy, the developer resumes. The provider starts from the actual current destination and current remaining staircase, not the original plan's stale assumptions:

```console
$ git staircase verify payments --provider gerrit
Provider verification for 'payments'
  structure revision: b19c77a

  step  review  exact revision  result
  1     48144   c812000         passed
  2     48102   c822000         passed
  3     48103   d833000         passed

Result: passed for the exact current review revisions.
```

```console
$ git staircase land payments --provider gerrit --stepwise
Resuming stepwise landing for 'payments'.
  destination: a711100
  remaining reviews: 48144, 48102, 48103

Submitted review 48144.
  destination: a711100 -> a812100
  strategy: cherry-pick

Submitted review 48102.
  destination: a812100 -> a822100
  strategy: cherry-pick

Submitted review 48103.
  destination: a822100 -> a833100
  strategy: cherry-pick

Landing complete for 'payments'.
  old destination: a700000
  new destination: a833100
  merged reviews: 48101, 48144, 48102, 48103
  active steps remaining: 0
  actual reviewed commits reachable: no
  patch-integrated equivalents confirmed: 4
  lifecycle: active, fully landed
```

The output does not falsely claim that the original reviewed commit OIDs are ancestors when Gerrit created cherry-picked destination commits.

---

## A.9 Journey 8: Aggregate topic submission is blocked by an unrelated change

A managed staircase uses Gerrit topic `payments-redesign`. The selected landing set contains reviews:

```text
48101 48144 48102 48103
```

Gerrit reports that the same topic also contains unrelated review `49000`.

The aggregate landing plan must enumerate the actual topic membership:

```console
$ git staircase land payments --provider gerrit --aggregate
error: Gerrit topic contains changes outside the selected landing set

selected landing set:
  48101 48144 48102 48103

additional topic members:
  49000  Refactor unrelated metrics exporter

No changes submitted.
code: landing-blocked
detail: gerrit.topic-contains-unrelated-changes
```

Changing the local staircase title, labels, or topic metadata does not make review `49000` part of the staircase. The developer may choose stepwise landing or separately remove the unrelated topic member through an explicit Gerrit operation outside this command.

A stepwise prefix landing remains valid because it does not rely on whole-topic submission:

```console
$ git staircase land payments --provider gerrit --through payments:1
Landing plan for 'payments'
  provider: gerrit
  mode: stepwise through step 1
  selected review: 48101
  unrelated topic members submitted: none

Submitted review 48101.
  reviewed revision: b711000
  destination: a700000 -> a711100
  actual strategy: cherry-pick

Landing complete through step 1.
  merged reviews: 48101
  remaining active reviews: 48144, 48102, 48103
```

---

## A.10 Journey 9: Attach reviews created by an existing Gerrit workflow

A team has already uploaded two dependent commits with ordinary Git:

```console
$ git push review HEAD:refs/for/main
remote: New Changes:
remote:   https://review.example.com/c/platform/payments/+/51001
remote:   https://review.example.com/c/platform/payments/+/51002
```

The local staircase is managed, but it has no provider associations. The developer attaches each confirmed review explicitly:

```console
$ git staircase review attach payments:1 \
    --provider gerrit \
    --review https://review.example.com/c/platform/payments/+/51001
Attached Gerrit review to 'payments' step 1.
  review: review.example.com/platform/payments~51001
  Change-Id: Iaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  patch set: 1
  remote revision: e110000
  local revision: e110000
  synchronization: current

record revision: 510ac11 -> 630bb82
```

```console
$ git staircase review attach payments:2 \
    --provider gerrit \
    --review https://review.example.com/c/platform/payments/+/51002
Attached Gerrit review to 'payments' step 2.
  review: review.example.com/platform/payments~51002
  Change-Id: Ibbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
  patch set: 1
  remote revision: e120000
  local revision: e120000
  synchronization: current

record revision: 630bb82 -> 74c19d0
```

A title or topic match would not have been sufficient. The provider validated server, project, branch, mapping, exact patch-set OID, and commit relationship.

Detaching one review removes only the local association:

```console
$ git staircase review detach payments:2 \
    --provider gerrit \
    --review https://review.example.com/c/platform/payments/+/51002
Detached Gerrit review from 'payments' step 2.
  review: review.example.com/platform/payments~51002
  retained as historical association: yes
  remote review changed: no

record revision: 74c19d0 -> 7db44e2
```

The Gerrit change remains open and unchanged.

---

## A.11 Journey 10: Archive and unarchive preserve Gerrit associations without remote mutation

The developer archives an inactive but still open staircase:

```console
$ git staircase archive payments
Archived 'payments'.
  lineage: 7c1f287e-0b7b-48ec-8901-758c73030f5c
  owned local branches archived: 4
  Gerrit review associations preserved: 4
  remote reviews changed: 0
  network requests: 0
```

Provider-mutating operations are blocked while archived:

```console
$ git staircase review upload --id 7c1f287e-0b7b-48ec-8901-758c73030f5c --provider gerrit
error: archived staircase cannot be uploaded
  lineage: 7c1f287e-0b7b-48ec-8901-758c73030f5c
  code: archived-mutation

Unarchive it before review mutation:
  git staircase unarchive --id 7c1f287e-0b7b-48ec-8901-758c73030f5c
```

Unarchive restores local active state and then marks remote observations stale until refreshed. It does not assume that Gerrit remained unchanged while the staircase was archived:

```console
$ git staircase unarchive --id 7c1f287e-0b7b-48ec-8901-758c73030f5c
Unarchived 'payments'.
  owned local branches restored: 4
  Gerrit review associations restored: 4
  remote observations: stale
  remote reviews changed: 0

Run:
  git staircase review status payments --provider gerrit
```

```console
$ git staircase review status payments --provider gerrit
payments
  provider: gerrit

  step  review  remote state  synchronization
  1     48101   merged        integrated
  2     48144   open          current
  3     48102   open          remote-newer
  4     48103   abandoned     blocked

Result: reconciliation required before publication or landing.
```

The provider reports each externally changed state. It does not reopen, upload, reset, or abandon anything implicitly.