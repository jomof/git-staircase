# Addendum G: GitHub Repository Routing, Pull Request, Verification, and Landing Provider

## 1. Status and Scope

This addendum defines the Staircase GitHub provider. It specializes the provider contracts in the consolidated **Git Staircase Specification**, especially its workspace and provider architecture, implicit and managed behavior, persistent records, mutation protocol, verification, review-provider contract, landing, lifecycle, output, security, and provider-accommodation requirements.

The consolidated specification is authoritative for core vocabulary and command semantics. This addendum defines only GitHub-specific routes, identities, mapping topologies, publication mechanics, verification evidence, and landing behavior. Where this addendum and the core appear to conflict, the core requirement governs unless the core explicitly delegates the behavior to the provider.

The provider may compose with any workspace or integration-context provider. In particular, a `repo` workspace provider may supply workspace, project, integration, or repository hints, but GitHub independently resolves and validates its hosted-repository and pull-request routes.

The provider may implement the following capabilities:

```text
repository-routing
review
review-identity
verification
review-transport
landing
```

It is not a general `workspace`, `project-mapping`, or `integration-context` provider. After a review destination has been selected or an existing pull request has been associated, it MAY return the exact pull-request base OID and a compatible local ref as typed integration-context evidence. The core still selects and validates the integration context under its precedence rules.

The provider supports:

* GitHub.com.
* GitHub Enterprise Server.
* Same-repository pull requests.
* Pull requests from forks.
* Aggregate pull requests.
* Branch-based chains of stacked pull requests.
* Explicit cumulative pull-request sets.
* Check runs and commit statuses.
* Branch protection and ruleset requirements.
* Auto-merge.
* Merge queues and merge-group verification.
* Merge, squash, and rebase landing methods.

The provider MUST remain usable in:

* A standalone Git repository using the `core.git` workspace fallback.
* A provider-managed multi-repository workspace.
* An attached or detached worktree.
* A repository with several GitHub remotes.
* A repository with separate fetch, push, base, and fork remotes.
* An active or archived managed staircase for operations permitted by the core lifecycle model.

GitHub pull requests propose merging one head branch into one base branch. The head branch may belong to the base repository or to another repository in the fork network. Staircase lineage, step identity, local cut layout, and durable operation recovery remain core concepts rather than GitHub-native objects.

---

## 2. Provider Identity

The canonical provider name is:

```text
github
```

Recommended provider descriptor:

```json
{
  "protocol_version": 1,
  "name": "github",
  "capabilities": [
    "repository-routing",
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

The passive probe establishes local applicability and possible repository routes.

It does not:

* Authenticate.
* Query GitHub.
* Push branches.
* Create pull requests.
* Install hooks.
* Modify remotes.
* Modify staircase metadata merely because a GitHub-looking remote exists.

---

## 3. Non-Goals

The GitHub provider does not:

* Determine the workspace root.
* Assume that a repository hosted on GitHub is a one-repository workspace.
* Treat a remote named `origin` as authoritative.
* Infer the integration branch from names such as `main` or `master`.
* Treat the repository’s default branch as the review destination without evidence.
* Treat every local branch as a pull-request head branch.
* Treat every staircase step as one commit.
* Treat every staircase step as one pull request unless an explicit mapping policy says so.
* Treat a pull-request number as globally unique.
* Treat a branch name as stable review identity.
* Treat successful checks as sufficient proof that a pull request is mergeable or submittable.
* Treat GitHub Actions as the only possible check producer.
* Fetch pull-request refs during passive bootstrap.
* Contact GitHub merely to make `git staircase list` more decorative.
* Automatically create or delete remote branches during provider discovery.
* Transport Staircase records through pull-request publication; `git staircase push` and `fetch` remain separate core transport operations.
* Treat GitHub labels, milestones, projects, branch names, or pull-request titles as Staircase lineage or stable step identity.
* Mutate pull requests or remote branches merely because a staircase is archived or unarchived locally.
* Store credentials, transient check output, or unvalidated server text in authoritative Staircase structure.

---

## 4. Conceptual Model

### 4.1 GitHub installation

A **GitHub installation** is one independently administered GitHub service.

Examples include:

```text
github.com
github.enterprise.example
```

A hostname is initially a routing locator.

After network validation, the provider may record a stronger server identity supplied or confirmed by the service.

---

### 4.2 Hosted repository

A **hosted repository** is a repository identity within one GitHub installation.

Before network validation, it may be represented provisionally as:

```text
installation locator
owner name
repository name
```

After validation, the provider should additionally retain stable server-issued identifiers where available.

The provider must distinguish:

```text
local Git repository
GitHub base repository
GitHub head repository
GitHub fork parent or source repository
```

These may be different objects.

---

### 4.3 Pull-request base

A pull request’s **base** consists of:

```text
base repository
base branch
base branch OID
```

The base repository owns the pull request and its pull-request number.

---

### 4.4 Pull-request head

A pull request’s **head** consists of:

```text
head repository
head branch
head commit OID
```

The head repository may be the base repository or a fork.

A pull request from a fork compares a branch in the fork with a branch in the upstream repository.

---

### 4.5 GitHub review revision

The exact GitHub review revision of an open pull request is its current full head commit OID.

The pull-request identity remains stable when the head branch is updated, while:

```text
head OID
diff
checks
approvals
mergeability
```

may change.

This parallels the Staircase distinction between:

```text
stable review identity
exact current revision
```

---

### 4.6 Review route

A **GitHub review route** contains:

```text
GitHub installation
base repository
base branch
head repository
head branch policy
authentication profile
API endpoint
Git transport endpoint
```

A provider may be applicable even when the complete route has not yet been resolved.

---

## 5. Passive Applicability Probe

### 5.1 Inputs

The passive probe may inspect:

* Local Git remotes.
* Fetch and push URLs.
* Full remote refspecs.
* Remote symbolic `HEAD` refs already present locally.
* Existing Staircase provider configuration.
* Workspace-provider hints.
* Locally registered GitHub Enterprise hosts.
* Existing managed GitHub review associations.
* Environment variables explicitly intended to select GitHub context.

It must not access the network.

---

### 5.2 GitHub.com remotes

A locally configured remote whose normalized host is:

```text
github.com
```

and whose path has a valid repository shape:

```text
<owner>/<repository>
```

is strong evidence that the repository is hosted on GitHub.com.

Supported URL forms should include at least:

```text
https://github.com/OWNER/REPOSITORY.git
ssh://git@github.com/OWNER/REPOSITORY.git
git@github.com:OWNER/REPOSITORY.git
git://github.com/OWNER/REPOSITORY.git
```

The provider must parse these as URLs or Git transport locators, not by fragile string slicing.

---

### 5.3 GitHub Enterprise hosts

A remote on a custom hostname is strong GitHub evidence only when the host is identified through one of:

1. Explicit GitHub provider configuration.
2. A trusted workspace-provider hint.
3. A locally registered GitHub Enterprise host.
4. A previously network-validated GitHub provider route.
5. An explicitly selected GitHub host for the current command.

A hostname containing words such as:

```text
github
git
review
source
code
```

is not sufficient evidence.

GitHub CLI supports authentication against explicitly selected GitHub hosts, including enterprise hosts, but authentication inspection is separate from passive provider probing.

---

### 5.4 GitHub CLI state

The presence of the `gh` executable does not establish that the current repository is hosted on GitHub.

The provider may use locally registered GitHub CLI host information as corroborating evidence only through a trusted, non-secret interface.

It must not invoke:

```text
gh auth status
```

during a passive probe because that command tests authentication state for known hosts and may perform behavior beyond purely static repository inspection.

The provider must never read or copy authentication tokens during discovery.

---

### 5.5 Weak evidence

The following are insufficient by themselves:

* A `.github` directory.
* GitHub Actions workflow files.
* `CODEOWNERS`.
* A branch named `pull-request`.
* Locally fetched `refs/pull/*`.
* A `gh` executable.
* A remote named `github`.
* A remote named `origin`.
* A commit message mentioning a pull request.
* A hostname that resembles GitHub.
* A GitHub URL found in arbitrary repository content.

Weak evidence may be included in diagnostics but must not activate the provider automatically.

---

### 5.6 Multiple remotes

The provider must inspect every relevant remote.

It must not assume:

```text
origin = head repository
upstream = base repository
```

Those names are common conventions, not identities.

Several remotes may represent:

* Different protocols for the same hosted repository.
* Separate fetch and push routes.
* A personal fork and its upstream repository.
* Several upstream repositories.
* A mirror.
* A migrated repository.
* Two unrelated GitHub repositories sharing local history.

Candidates that normalize to the same provisional hosted-repository identity may be coalesced while preserving all transport URLs.

---

### 5.7 Provider binding

The GitHub provider may be automatically bound when at least one unambiguous high-confidence GitHub repository candidate exists.

The binding may remain partially configured:

```text
provider: GitHub
hosting repository: known
base repository: unresolved
head repository: unresolved
review destination: unresolved
authentication: unresolved
```

This is preferable to refusing all GitHub-aware behavior or guessing a complete review route.

---

## 6. Repository Routes and Identity

### 6.1 Provisional identity

Before network validation, a hosted repository is identified provisionally by:

```text
normalized installation locator
owner component
repository component
```

The `.git` suffix is removed for repository interpretation.

The original transport URL is retained separately.

---

### 6.2 Canonical server identity

For GitHub.com, the canonical installation identity may be a built-in constant.

For GitHub Enterprise Server, the provider should establish a canonical server identity during the first network operation or through explicit trusted configuration.

The canonical identity must account for:

* Scheme where relevant.
* Host.
* Port.
* API base path.
* Explicit aliases.
* Enterprise migrations or front-door aliases.

Two endpoints must not be treated as one server solely because their DNS names resemble one another.

---

### 6.3 Canonical repository identity

After network validation, canonical repository identity should use:

```text
canonical GitHub installation identity
stable repository identifier
```

Human names remain locators and display values.

This permits the provider to distinguish repository continuity from:

* Repository rename.
* Owner rename.
* Organization transfer.
* Remote URL change.

Until a stable server identity is available, the route remains provisional.

---

### 6.4 Remote roles

A Git remote may have one or more roles:

```text
fetch source
push destination
head repository
base repository
read-only mirror
symbolic integration-target source
```

Roles are selected through evidence and explicit policy, not remote names.

---

### 6.5 Base-repository precedence

The base repository for review is chosen using:

1. Explicit command input.
2. Managed staircase review policy.
3. Existing associated pull requests.
4. Explicit repository-level provider configuration.
5. Trusted workspace-provider hint.
6. A uniquely proven upstream relationship after network validation.
7. Unresolved.

The provider must not infer the base repository solely from which remote is fetched most often.

---

### 6.6 Head-repository precedence

The head repository is chosen using:

1. Explicit command input.
2. Existing pull-request association.
3. Managed staircase head-publication policy.
4. A uniquely writable configured push route.
5. A verified fork belonging to the active account or organization.
6. Unresolved.

A successful local fetch does not prove push permission.

---

## 7. Integration-Context Evidence and Review Destination

### 7.1 Review destination as integration-context evidence

Once a base repository and base branch have been selected, the provider may return the corresponding local remote-tracking branch or resolved OID as an integration-context candidate.

For example:

```text
review base:
  repository: organization/project
  branch: refs/heads/main

local candidate:
  refs/remotes/upstream/main
```

The Staircase core still validates the OID and graph relationship.

---

### 7.2 No default-branch guess

The provider must not choose a review base merely because GitHub identifies a repository default branch.

A default branch is useful evidence, but it is not necessarily:

* The staircase’s intended destination.
* The release branch.
* The branch used by an existing pull request.
* The correct symbolic integration target for every local branch.

The default branch may be used automatically only when a configured policy explicitly states that new reviews use the repository default branch as their review destination.

---

### 7.3 Existing pull request

For a staircase already associated with a pull request, the pull request’s exact base repository and base branch are authoritative review-route evidence.

The currently resolved base branch OID is used as an exact integration-context candidate for review-relative operations.

---

## 8. Pull-Request Identity

### 8.1 Canonical identity

The canonical GitHub pull-request identity is:

[
P =
(\text{installation},\text{base repository},\text{pull-request number})
]

Machine representation:

```json
{
  "provider": "github",
  "installation_id": "<stable-installation-id>",
  "installation_locator": "github.com",
  "base_repository_id": "<stable-repository-id>",
  "number": 412
}
```

Before repository identity is network-confirmed, the provider may use a provisional identity:

```json
{
  "provider": "github",
  "installation": "github.com",
  "base_repository": "organization/project",
  "number": 412,
  "provisional": true
}
```

---

### 8.2 Pull-request number scope

A pull-request number is scoped to its base repository.

The following are distinct:

```text
organization/project-a#42
organization/project-b#42
another-host/organization/project-a#42
```

A bare `#42` is accepted only when the base repository is already unambiguous.

---

### 8.3 Accepted selectors

The provider may accept:

```text
#412
organization/project#412
github.com/organization/project#412
full pull-request URL
provider-native node identifier
```

Selectors must resolve to one canonical pull-request identity.

A URL is parsed as structured input and must match a configured or explicitly allowed GitHub installation.

---

### 8.4 Exact review revision

An associated pull request record must include:

```text
pull-request identity
head repository identity
head ref
head OID
base repository identity
base ref
base OID
last observed state
```

The head OID is required before verification evidence may be attached to a local structure revision.

---

### 8.5 Deleted head branch or repository

A pull request may remain a valid historical review identity after:

* Its head branch is deleted.
* Its fork is deleted or inaccessible.
* The local remote is removed.
* The local branch is renamed.

In such a state, the provider records:

```text
review identity: valid
head route: unavailable
update capability: unavailable
```

It must not discard the review association merely because the head ref can no longer be resolved.

---

## 9. Review Mapping Policies

The GitHub provider supports at least two primary mapping policies.

### 9.1 Aggregate pull request

Under the **aggregate** policy:

```text
one staircase
    → one pull request
```

The pull-request head represents the aggregate top of the active staircase.

The pull-request base is the selected integration branch.

The pull request contains every active staircase step.

This policy is natural for:

* Fork-based contributions.
* Teams that review the staircase as one unit.
* Repositories that do not support dependent pull-request chains.
* Staircases whose intermediate steps are not independently landable.

---

### 9.2 Stacked pull-request chain

Under the **stacked** policy:

```text
one staircase step
    → one pull request
```

For a staircase:

```text
S1, S2, ..., SN
```

the desired branch topology is:

```text
PR1:
  base = integration branch
  head = branch for S1

PR2:
  base = branch for S1
  head = branch for S2

...

PRN:
  base = branch for S(N-1)
  head = branch for SN
```

This causes each pull request to present the incremental difference introduced by one step.

GitHub pull requests compare a selected head branch with a selected base branch, and the base branch may be changed after creation.

---

### 9.3 Pull request is step-oriented, not commit-oriented

A GitHub pull request may contain several commits from its head branch.

Therefore, unlike the default Gerrit mapping, a natural GitHub mapping is:

```text
one staircase step
    → one pull request
    → one or more commits
```

A provider must not require one commit per step unless repository policy explicitly does so.

---

### 9.4 No native staircase identity

GitHub does not provide a general first-class staircase entity.

The provider derives a stacked chain from:

```text
base repository
base branch
head repository
head branch
pull-request associations
```

Staircase lineage and step identity remain Staircase concepts.

A GitHub label, milestone, project item, or shared title prefix must not replace the Staircase lineage ID.

---

### 9.5 Cumulative pull-request set

Under the **cumulative** policy:

```text
one active staircase prefix
    → one pull request

all pull requests
    base = selected integration branch
```

The pull request for step `i` contains the cumulative prefix through that step. This is not an incremental stacked chain. The provider MUST report that upper reviews repeat lower-step changes, may not be independently landable, and require separate landing rules.

---

### 9.6 Mapping persistence

`aggregate`, `stacked`, and `cumulative` are provider realizations of the core review-mapping classes. A mapping selected only for `review plan` is invocation-local. Persisting a default mapping requires a managed staircase and a structure-record mutation through the core policy mechanism.

A mapping transition for an already-associated staircase MUST produce a complete plan that identifies pull requests to retain, retarget, detach, close, supersede, or create. No existing pull-request identity is reassigned to a different conceptual step merely because the requested topology changed.

---

## 10. Stacked Pull-Request Topology Constraints

### 10.1 Base branches belong to the base repository

A pull request’s selected base branch belongs to the selected base repository.

A fork-based pull request normally uses a branch in the upstream repository as its base and a branch in the fork as its head.

Therefore a chain such as:

```text
fork:feature-1
fork:feature-2
fork:feature
```

cannot automatically become three incremental pull requests in the upstream repository unless the required intermediate base branches also exist in a repository that can own those pull-request bases.

---

### 10.2 Same-repository chain

The simplest stacked topology is:

```text
base repository = head repository
```

with all staircase branches published to that repository.

This requires sufficient permission to create and update those branches.

---

### 10.3 Fork-based aggregate review

For a normal fork workflow, the aggregate policy is the default safe mapping:

```text
base: upstream/integration-branch
head: fork/staircase-tip
```

This matches GitHub’s ordinary fork pull-request model.

---

### 10.4 Fork-based stacked review

A fork-based stacked review is allowed only when the provider can construct a valid explicit topology.

Possible topologies include:

1. Intermediate base branches are published to the upstream repository with appropriate permission.
2. The dependent pull requests are hosted in the fork repository, with a final aggregate or promotion pull request targeting the upstream repository.
3. A repository-specific policy supplies another supported arrangement.

The provider must show which repository owns every pull request.

It must not present several cumulative upstream pull requests as isolated steps when each actually includes all lower changes.

---

### 10.5 Cumulative pull-request set

A set of pull requests that all use the integration branch as their base:

```text
integration <- step-1
integration <- step-2-cumulative
integration <- tip-cumulative
```

is a **cumulative pull-request set**, not a stacked incremental chain.

This topology may be supported explicitly, but it has different semantics:

* Upper reviews include lower-step changes.
* Review diff size grows cumulatively.
* Independent landing may be unsafe.
* Duplicate change presentation is expected.

The provider must label the topology accurately.

---

## 11. Stable Remote Head Branches

### 11.1 Local layout names are positional

Under the core `sequential-v1` primary-branch layout, local names such as:

```text
feature-1
feature-2
feature
```

follow current staircase positions.

They may change after split, join, reorder, or partial landing.

---

### 11.2 Pull-request head names are review locators

An existing GitHub pull request remains associated with its configured remote head branch.

The provider must not assume that an existing pull request can follow arbitrary local branch renames.

Consequently:

> Local sequential branch names and remote review branch names are separate naming layers.

---

### 11.3 Default remote-head policy

After a pull request is created, its remote head branch SHOULD remain stable for the lifetime of that review. Stability is the invariant; the initial spelling is selected by policy. For newly created stacked mappings, the provider-generated `stable` naming policy SHOULD be the default unless repository policy requires another scheme.

A managed step stores:

```text
step ID
local primary branch
GitHub head repository
GitHub head branch
pull-request identity
```

When the local step is renumbered, the provider updates the existing remote head branch from the step’s new local branch.

Example:

```text
before:
  step ID X
  local branch: feature-2
  remote review branch: feature-2

after local insertion:
  step ID X
  local branch: feature-3
  remote review branch: feature-2
```

The next push is conceptually:

```text
local feature-3
    → remote feature-2
```

This preserves pull-request identity.

---

### 11.4 Provider-generated stable head names

Under the `stable` policy, the provider uses remote branch names based on stable identity:

```text
staircase/<lineage-id>/<step-id>
```

or a shorter collision-resistant equivalent.

These names:

* Follow stable step identity.
* Do not require remote renumbering.
* Need not match local primary branch names.
* Must be validated as Git refs.
* Must respect repository rulesets and branch restrictions.

---

### 11.5 Mirror-primary policy

A policy may require remote review branches to mirror local primary branch names.

This policy is safe only when:

* No existing pull-request identity depends on the old branch name, or
* GitHub and repository policy provide a verified continuity-preserving rename operation, or
* The user explicitly accepts closing and recreating affected reviews.

It is not the default.

---

## 12. Branch Publication

### 12.1 Publication plan

Before creating or updating pull requests, the provider constructs a publication plan:

```text
structure revision OID or implicit structural key
record revision OID, when managed
local source OID
head repository
remote branch
expected old remote OID
required update kind
associated pull request
```

---

### 12.2 Fast-forward updates

A fast-forward branch update may use an ordinary protected push with an expected remote state.

---

### 12.3 Rewritten staircase steps

A rebase, restack, reorder, split, join, or amendment may require a non-fast-forward update of an existing pull-request head branch.

Such an update must use force-with-lease semantics against the provider’s last confirmed remote OID.

An unconditional force push is nonconforming.

---

### 12.4 Stale lease

If the remote branch moved after planning:

* The push must fail.
* The provider must fetch or query current state explicitly.
* The provider must determine whether the change came from:

  * the same user,
  * a maintainer,
  * automation,
  * a conflicting workspace,
  * an unknown actor.
* The provider must not overwrite the remote update automatically.

---

### 12.5 Protected head branch

Rulesets or branch protection may prevent:

* Force pushes.
* Branch creation.
* Branch deletion.
* Updates without signed commits.
* Updates without required checks.

GitHub rulesets and protected-branch policies can govern branch updates and pull-request merge requirements.

A blocked push is reported as a route or policy conflict, not as evidence that the local staircase is invalid.

---

### 12.6 Head repository permission

The ability to read a remote does not establish permission to push.

The provider confirms write capability during a network operation or through explicit configuration.

For fork pull requests, maintainers may have conditional permission to update a contributor’s branch, but this must not be assumed universally.

---

## 13. Creating and Associating Pull Requests

### 13.1 Review plan

Before network mutation, the provider produces a complete review plan containing:

```text
structure revision OID or implicit structural key
record revision OID, when managed
mapping policy
base repository and branch
head repository
per-step or aggregate head branches
expected head OIDs
existing review associations
pull requests to create
pull requests to update
pull requests to retarget
branches to publish
branches whose deletion is deferred
```

---

### 13.2 Preconditions

The provider checks:

* Complete repository routes.
* Destination branch resolution.
* Push permission or known push route.
* Step materialization.
* Remote branch collisions.
* Existing pull requests with conflicting topology.
* Draft policy.
* Visibility and fork compatibility.
* Whether the selected structure revision, record revision, or implicit structural key changed during planning.
* Whether a head branch moved remotely.
* Whether a pull request already exists for the intended branch pair.

---

### 13.3 Staircase drafts and GitHub draft pull requests

A GitHub **draft pull request** is remote review state. A Staircase **worktree draft** is staged, unstaged, untracked, or ignored local content relative to an exact basis. The provider MUST NOT conflate them.

The provider supports the core hypothetical planning form:

```console
git staircase review plan <selector> --include-draft
```

The result MUST:

* Describe the hypothetical immutable commit and review topology that would result from materializing the exact current index under the selected intent.
* Label the plan hypothetical.
* Identify unstaged, untracked, and ignored content that is excluded.
* Avoid creating commits, adopting, pushing, or creating pull requests.
* Distinguish a policy to create the eventual pull request in GitHub draft state from the presence of a local worktree draft.

`review create` and `review upload` operate on immutable commit OIDs. A combined materialize-and-publish workflow MUST complete and publish the local Staircase mutation before beginning GitHub branch or pull-request mutation.

---

### 13.4 Pull-request creation

A pull request must be created against exact planned:

```text
base repository
base branch
head repository
head branch
```

The local branch spelling is not substituted after planning without revalidation.

---

### 13.5 Existing pull-request discovery

Network-assisted discovery may search by:

* Head repository and head branch.
* Base repository and base branch.
* Known head OID.
* Pull requests associated with a commit.
* Existing managed review identity.
* Explicit pull-request selector.

Branch or commit matching alone may produce several candidates.

The provider must not choose silently.

---

### 13.6 Local pull-request refs

GitHub exposes temporary read-only refs such as:

```text
refs/pull/<number>/head
refs/pull/<number>/merge
```

The head ref follows the pull-request head commit. The merge ref represents a simulated merge when the pull request has no merge conflict.

If these refs are already fetched locally, they may provide exact revision evidence.

They do not by themselves establish:

* Server identity.
* Base repository identity.
* Base branch.
* Review state.
* Canonical pull-request identity.

---

### 13.7 Association persistence

A managed staircase may store:

```text
GitHub installation identity
base repository identity
pull-request number
step ID or aggregate staircase association
head repository identity
head branch
last observed head OID
base branch
last observed base OID
mapping policy
last confirmed remote branch OID used as a lease
```

Remote observations are cached evidence and MUST be revalidated before mutation.

Durable review associations that affect mapping, publication, or landing belong in the managed structure descriptor under a namespaced GitHub extension. Volatile observations such as current checks, mergeability, queue position, or the last query time belong in provider cache unless a core recovery or audit rule requires them in an operation journal.

---

### 13.8 Attaching and detaching existing pull requests

The provider supports the core commands:

```console
git staircase review attach <step-or-staircase-selector> \
    --provider github --review <github-review-selector>
git staircase review detach <step-or-staircase-selector> \
    --provider github [--review <github-review-selector>]
```

`review attach` MUST:

1. Resolve the selector to one canonical pull-request identity.
2. Establish the installation, base repository, base branch, head repository, head branch, base OID, and head OID when network access is available.
3. Validate compatibility with the selected aggregate, step, or cumulative subject.
4. Refuse a branch-name, title, or commit-only guess when several pull requests match.
5. Store a provisional association when validation is incomplete and prevent that association from satisfying review or verification policy.
6. Publish the association through full-record compare-and-swap.

`review detach` removes only the local association by default. It does not close the pull request, delete its branch, retarget it, disable auto-merge, or leave a merge queue unless an explicit GitHub mutation is separately planned.

---

### 13.9 Record publication and remote uncertainty

A durable create, upload, attach, detach, retarget, queue, auto-merge, or landing operation spans local and remote surfaces that cannot be one physical transaction.

The provider MUST therefore use the core mutation protocol:

* Select one exact record revision or implicit discovery snapshot.
* Determine whether automatic adoption is required before remote mutation.
* Persist a durable operation journal before an uncertain remote mutation can occur.
* Protect every local record update with full-record compare-and-swap.
* Protect every non-fast-forward branch update with an expected-old remote OID.
* Reconcile the actual pull-request and branch state after mutation.
* Leave the operation resumable or reconciliation-required when the remote result is uncertain or the local record changed concurrently.

A successful remote mutation followed by `concurrent-record-update` MUST NOT be repeated blindly. The next action is `git staircase review reconcile <selector>` against the canonical pull-request identity and exact planned OIDs.

---

## 14. Review State

The provider should preserve distinct pull-request state fields rather than flattening them prematurely.

Relevant states include:

```text
open
closed
merged
draft
ready for review
head unavailable
base unavailable
mergeability unknown
mergeable
conflicting
review required
changes requested
approved
checks pending
checks failed
checks passed
merge queue required
queued
auto-merge enabled
```

Draft pull requests cannot be merged until made ready for review.

---

## 15. Exact Revision Reconciliation

### 15.1 Current

A local review mapping is **current** when:

```text
local planned cut or top OID
    =
GitHub pull-request head OID
    =
remote head branch OID
```

---

### 15.2 Local newer

The local staircase has changed and has not yet been published.

---

### 15.3 Remote newer

The remote head branch or pull request has advanced beyond the provider’s last known OID.

The provider must not force-update until reconciled.

---

### 15.4 Diverged

Neither local nor remote OID is an ancestor of the other, or the associated histories otherwise conflict.

---

### 15.5 Review retargeted externally

The pull request’s base branch differs from the stored mapping.

The provider records the externally changed route and requires explicit reconciliation before a topology-changing operation.

---

### 15.6 Review closed or merged

A closed or merged pull request remains historical evidence.

It must not be reopened or replaced automatically merely because a local branch still exists.

---

## 16. Verification Model

### 16.1 Separate verification subjects

GitHub-related verification may apply to different typed core subjects:

1. `provider-review-revision`: the pull-request head commit.
2. `provider-test-merge`: GitHub’s simulated test merge commit.
3. `provider-merge-group`: a merge-queue merge-group commit.
4. `landed-revision`: the eventual landed commit.
5. `structure-prefix` or `structure-aggregate`: a local Staircase subject.

These subjects MUST NOT be conflated. Every evidence item MUST identify its exact subject OID, exact base OID where applicable, policy and profile identity, observation provenance, and freshness conditions.

---

### 16.2 Head verification

Head verification is attached to:

```text
head repository
head OID
```

Required checks generally apply to the latest relevant pull-request commit, not an earlier head revision.

---

### 16.3 Test merge commit

GitHub may create a temporary test merge commit to compute whether a pull request can be merged into its current base.

The pre-merge `merge_commit_sha` may identify this test merge, and mergeability computation may initially be pending.

The provider represents:

```text
test merge OID
base OID used
head OID used
mergeability result
observation time
```

A test merge result becomes stale when either base or head moves.

---

### 16.4 Pull-request refs

When available, the GitHub pull-request merge ref represents a simulated current merge and updates when the head changes.

It is verification evidence, not permanent identity.

---

### 16.5 Check runs and commit statuses

GitHub supports both check runs and commit statuses.

They are separate evidence channels.

The provider must query and report both when relevant.

GitHub’s Checks tab is populated by checks rather than legacy commit statuses, while commit statuses remain a separate API and merge-policy mechanism.

---

### 16.6 Repository scope of checks

Check-run association is repository-specific.

GitHub notes that Checks API behavior only associates pushes in the repository where the check suite or run was created, which matters for fork-based pull requests.

The provider may need to inspect:

```text
base repository checks
head repository checks
pull-request check rollup
test merge commit checks
```

It must not infer missing checks merely from one repository-scoped query.

---

### 16.7 Required checks

A check result is considered current only when it applies to the exact required subject OID.

Checks for a previous head OID do not verify a rewritten structure revision.

---

### 16.8 Review requirements

Approvals, requested changes, code-owner requirements, required conversations, and other review conditions remain distinct from build checks.

Branch-protection rules and rulesets may require approving reviews and passing checks before merge.

---

### 16.9 Verification aggregation

Recommended provider result states are:

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

`passed` requires satisfaction of the configured Staircase policy, not merely one green check.

The provider must not report `passed` when:

* The pull request head differs from the local step.
* Required checks apply to a newer OID.
* The base changed after checks completed.
* Reviews were dismissed or invalidated.
* The pull request is draft under a ready-for-review requirement.
* The pull request has conflicts.
* The merge queue requires separate merge-group checks.
* Required policy could not be determined.
* The provider lacks sufficient access.

---

## 17. Merge Queues

### 17.1 Separate merge-group revision

A GitHub merge queue creates temporary merge-group branches and commits that differ from the ordinary pull-request head OID.

Required CI must run for the merge-group event or the queue’s temporary branch.

The provider records:

```text
pull-request head OID
queue group OID
queue base OID
queue position
queue check state
```

Queue checks do not retroactively become head checks.

---

### 17.2 Aggregate pull request

An aggregate staircase pull request may be queued normally when repository policy allows it.

---

### 17.3 Stacked chain

An upper pull request whose base is another staircase branch must not normally be queued for the final integration branch until:

* Its lower dependency has landed.
* Its base has been retargeted.
* Its branch has been restacked when required.
* Its current checks and review requirements have been recomputed.

The provider must not enqueue the entire chain as though the pull requests were independent.

---

### 17.4 Queue invalidation

Any update to:

```text
head branch
base branch
dependency chain
queue group
```

may invalidate queue state.

The provider treats queue membership as remote ephemeral state.

---

## 18. Auto-Merge

Auto-merge means GitHub will merge a pull request after configured merge requirements are satisfied.

The provider records:

```text
auto-merge enabled
selected merge method
enabling actor
last observed state
```

Auto-merge is not staircase identity.

For a stacked chain:

* Auto-merge may be enabled only according to the staircase landing policy.
* Upper pull requests must not be allowed to land before dependencies.
* A base-branch change or new push may disable or invalidate auto-merge state.
* The provider rechecks state after every lower-step landing.

---

## 19. Landing Methods

GitHub supports pull-request landing by merge commit, squash, or rebase where repository policy permits.

These methods have materially different staircase consequences.

---

### 19.1 Merge-commit landing

With merge-commit landing:

* The reviewed head commits remain reachable from the destination.
* The merge commit joins the base and head histories.
* Upper staircase branches that descend from the lower head may often remain structurally connected to the landed commits.
* Dependent pull requests still require base retargeting and status reconciliation.

The provider must verify the actual landed graph rather than assume the expected merge method was used.

---

### 19.2 Squash landing

With squash landing:

* The destination receives a new aggregate commit.
* The original lower-step commit IDs generally do not become ancestors of the destination.
* Upper staircase branches still contain the original lower-step commits.
* Merely changing an upper pull request’s base may cause already-landed changes to remain visible or produce a misleading diff.

The provider must normally restack upper steps onto the resulting destination commit before continuing stepwise landing.

---

### 19.3 Rebase landing

With rebase landing:

* GitHub applies commits onto the destination as new commit objects.
* Original local commit IDs may not become ancestors of the destination.
* Upper staircase branches may require restacking onto the new landed commits.

The provider must reconcile by patch or outcome relationship and then produce a new exact structure revision.

---

### 19.4 Landing result

After each merge, the provider records:

```text
pull-request identity
merge method
destination before OID
destination after OID
GitHub-reported merge or squash commit
local commits made reachable
steps requiring restack
next pull requests requiring retarget
```

---

## 20. Stepwise Landing State Machine

For a stacked pull-request chain, lower steps land before upper steps.

After pull request (P_i) lands, the provider must:

1. Query or fetch the exact new destination OID.
2. Determine the actual merge method and resulting graph.
3. Mark step (S_i) as integrated or patch-integrated.
4. Determine whether upper commits remain valid descendants.
5. Restack upper steps when required.
6. Update their stable remote head branches using lease-protected pushes.
7. Retarget (P_{i+1}) to the resulting integration branch.
8. Re-query the pull request’s diff, head, base, checks, and review state.
9. Retire the lower remote head branch only when no dependent pull request uses it as a base.
10. Continue only when the next review is current and policy permits landing.

This sequence is one managed Staircase operation whenever lineage, review continuity, or recovery state is retained.

It may span several remote transactions and therefore requires:

* The exact expected record revision at operation start.
* A durable operation journal and operation refs for local recovery objects.
* Exact destination-before, destination-after, head, base, and remote-branch OIDs.
* Full-record compare-and-swap for every durable local transition.
* Resume and abort where feasible.
* Reconciliation after uncertain network outcomes.
* Re-evaluation of checks, reviews, mergeability, auto-merge, and queue state after every base or head transition.

---

## 21. Base Retargeting

Changing the base branch of a pull request may change:

* Its diff.
* Its merge base.
* Its required checks.
* Its applicable rules.
* Its review relevance.
* Auto-merge state.
* Merge-queue eligibility.

The provider treats base retargeting as a review-revision transition even though the canonical pull-request identity remains unchanged.

It must not retain a prior “passed” status without re-evaluation.

---

## 22. Branch Deletion

### 22.1 Lower branch

A lower review branch must not be deleted while any active upper pull request uses it as its base.

---

### 22.2 Head branch after merge

Automatic deletion of a merged pull request’s branch is subject to repository policy and dependency topology.

The provider must override or delay deletion when that branch remains a base for another staircase review.

---

### 22.3 Deleted externally

If a required base branch is deleted externally:

* The stacked topology becomes broken.
* The affected pull request remains a known review identity.
* The provider must reconstruct or retarget the topology explicitly.
* It must not create an unrelated branch at the same name without lease and identity checks.

---

## 23. Pull-Request Diff Semantics

The provider must not assume that every locally computed diff exactly matches the GitHub pull-request Files view.

GitHub documents that compare pages and pull-request pages may use different merge-base timing, and a pull-request page may retain the common ancestor selected when the pull request was created.

Therefore:

* Local structural validation uses the Git commit graph.
* Review display validation may query GitHub’s current pull-request files or comparison.
* A mismatch is reported, not silently normalized away.
* Base retargeting may be required to restore an isolated step diff.

---

## 24. Detached `HEAD`

The GitHub provider does not require the current worktree to be attached to a branch.

In a detached worktree:

* The core or workspace provider resolves integration context.
* Staircase branches may exist as other refs.
* The GitHub provider may publish those refs without checking one out.
* The provider must not assume detached `HEAD` is a pull-request head.
* The provider must not create a review from detached `HEAD` unless it is selected explicitly as the source or materialized through a managed remote-head policy.

This permits the provider to compose naturally with the core detached-worktree and typed integration-context model.

---

## 25. Network Validation and Authentication

### 25.1 First network operation

The first network-requiring command may validate:

```text
GitHub installation
repository identities
fork relationships
default branch
permissions
authenticated account
API capabilities
merge methods
ruleset support
merge queue support
existing pull requests
```

This updates provider route state but does not retroactively alter staircase structure without an explicit operation.

---

### 25.2 Authentication sources

The provider should use established credential systems, including:

* Git credential helpers.
* GitHub CLI authentication.
* GitHub App credentials.
* Explicitly configured tokens.
* Enterprise authentication mechanisms supported by the installation.

Credentials must not be copied into:

* Staircase descriptors.
* Workspace discovery records.
* Git refs.
* Commit messages.
* Review descriptions.

---

### 25.3 Multiple accounts

Several accounts may be configured for one GitHub host.

The review route must include an explicit or deterministically selected authentication profile before mutation.

The provider must not choose an account solely because it was most recently used by another tool.

---

### 25.4 Insufficient access

GitHub may return limited or concealed results when the authenticated user lacks repository access.

The provider distinguishes:

```text
not found
not authorized
route incorrect
resource deleted
server unreachable
```

when the available API permits it.

It must not conclude that a private pull request does not exist merely from an unauthenticated failure.

---

## 26. Review and Verification Caching

Cached GitHub evidence includes:

```text
server identity
repository identity
pull-request identity
head and base OIDs
state
review decision
checks
statuses
mergeability
queue state
ruleset or protection summary
observation time
authentication profile class
```

Cached evidence becomes stale when:

* The local structure revision changes.
* The head branch moves.
* The base branch moves.
* The pull request is retargeted.
* The authenticated account changes.
* Repository policy changes.
* The provider route changes.
* The cache exceeds configured age.

`git staircase list` may display cached state with an age marker but must not silently perform a network refresh.

---

## 27. Persistent State, Policy, Lifecycle, and Transport Boundaries

### 27.1 Managed GitHub extension state

A durable GitHub mapping requires a managed staircase. The structure descriptor MAY contain a versioned namespaced extension with:

```text
provider route identity
base and head repository identities
base branch
mapping policy
aggregate or step-to-pull-request associations
stable remote head branch mappings
expected remote branch OIDs
landing and branch-retention policy that affects interpretation
parent structure revision
```

The provider extension MUST NOT store credentials. Volatile checks, reviews, mergeability, queue position, rate-limit state, and authentication diagnostics are cached observations, not authoritative local structure.

A durable mapping change, association change, remote-head mapping change, or landing-policy change modifies the structure and record revisions. A cache refresh alone normally changes neither.

---

### 27.2 Policy

Persistent policy is set through the core command:

```console
git staircase policy set <selector> <key>=<value>...
```

Core review-mapping and landing policies use core-defined keys. GitHub-only policy keys MUST be namespaced. A conforming provider SHOULD define schemas for at least:

```text
github.base-repository
github.base-branch
github.head-repository
github.remote-head-naming = stable | mirror-primary
github.create-as-draft = true | false
github.delete-head-after-merge = true | false
github.auto-merge = disabled | allowed | required
github.merge-queue = disabled | allowed | required
```

Policy validation MUST consider the complete resulting route and topology. For example, `mirror-primary` is invalid when it would break existing pull-request head identity, and `delete-head-after-merge=true` cannot override an active upper pull request that still uses the branch as its base.

---

### 27.3 Implicit staircases and automatic adoption

Read-only route discovery, `review plan`, pull-request discovery, and exact-revision status comparison MAY operate on an implicit staircase without adoption.

The provider MUST require management before preserving:

```text
pull-request associations
stable remote review branch mappings
stacked or cumulative topology
persistent GitHub or landing policy
stepwise landing continuity
remote operation recovery state after command control is relinquished
```

A command that requires this state SHOULD adopt automatically under the core rules. With `--no-adopt`, it MUST fail before local or remote mutation. Human and machine output MUST report the adoption reason and resulting lineage.

The provider MAY support:

```console
git staircase review upload <implicit-selector> --ephemeral --mapping aggregate
```

only as a one-time aggregate publication that retains no durable review association or landing continuity. It MUST NOT be used for stacked or cumulative mappings, and its output MUST say that later update continuity was not recorded.

---

### 27.4 Archive and unarchive

Core archive is local and offline by default.

Archiving a managed staircase:

* Preserves GitHub review identities, routes, remote-head mappings, and last-known exact OIDs in the archived record.
* Removes only Staircase-owned active local refs according to the core lifecycle rules.
* Does not close pull requests, delete GitHub branches, leave queues, disable auto-merge, or change labels.
* Does not treat continuing remote visibility as an archive failure.

Unarchiving restores local active state according to the core archive manifest. Before any later review upload, verification refresh, queue action, auto-merge action, or landing, the provider MUST revalidate the installation, repository route, pull-request state, head and base OIDs, and branch availability.

Any provider-side archival action is a separate explicit plan. Closing a pull request, adding an archival label, deleting a remote branch, or disabling auto-merge are distinct mutations and MUST be named separately in that plan.

---

### 27.5 Staircase-state transport versus review publication

The command families are distinct:

```text
git staircase push / fetch
    transport Staircase records, metadata, lifecycle, and required Git objects

git staircase review create / upload
    create pull-request identities and publish review head branches
```

A review upload MUST NOT silently publish `refs/staircases/*`, `refs/staircase-state/*`, or `refs/staircase-archive/*`. Staircase transport MUST NOT silently create or update pull requests.

---

### 27.6 Concurrent records and uncertain remote state

After any required automatic adoption, every durable GitHub mutation starts from one exact record revision. If another process changes structure, metadata, lifecycle, policy, or review association before publication, the local record update fails with core code:

```text
concurrent-record-update
```

If the remote mutation may already have succeeded, the operation remains reconciliation-required. The provider MUST query by canonical installation, base repository, pull-request identity, and exact head/base OIDs. It MUST NOT infer success from titles or branch spelling and MUST NOT issue a second create, push, retarget, merge, queue, or auto-merge request until the first outcome is resolved.

---

## 28. Commands

The GitHub provider specializes the consolidated core command surface. Canonical examples are:

```console
git staircase provider github doctor

git staircase review plan auth --mapping stacked
git staircase review create auth --mapping stacked
git staircase review upload auth
git staircase review status auth
git staircase review show auth
git staircase review open auth:2
git staircase review reconcile auth

git staircase review attach auth:2 \
    --provider github --review github.com/organization/project#412
git staircase review detach auth:2 \
    --provider github --review github.com/organization/project#412

git staircase verify auth --provider github

git staircase land auth --aggregate --method squash
git staircase land auth --stepwise --method merge
git staircase land auth --through auth:2 --method squash
git staircase land auth --aggregate --queue
```

Mapping-specific planning and creation include:

```console
git staircase review plan auth --mapping aggregate
git staircase review plan auth --mapping stacked
git staircase review plan auth --mapping cumulative

git staircase review create auth --mapping aggregate
git staircase review create auth --mapping stacked
git staircase review create auth --mapping cumulative
```

Route overrides include:

```console
git staircase review create auth \
    --mapping aggregate \
    --github-base github.com/organization/project \
    --base-branch release \
    --github-head github.com/user/project
```

A one-time aggregate publication without durable association MAY be exposed as:

```console
git staircase review upload feature --ephemeral --mapping aggregate \
    --github-base github.com/organization/project \
    --base-branch main \
    --github-head github.com/user/project
```

Provider-specific durable preferences use the core policy command:

```console
git staircase policy set auth \
    github.remote-head-naming=stable \
    github.merge-queue=required
```

`review create` establishes native review identities and may perform the minimum initial branch publication required by GitHub, but its output MUST distinguish pull requests created, branches published, and exact head revisions observed. `review upload` publishes exact current revisions to existing associations and creates only those reviews that the explicit operation plan says remain missing.

All human, porcelain, and JSON output follows the core output contract. Machine output MUST use full typed OIDs and canonical pull-request identities, and MUST keep provider details beneath stable core result or error codes.

## 29. Diagnostics

Provider diagnostics should distinguish:

```text
GitHub provider not applicable
provider applicable but route incomplete
several GitHub remotes found
base repository unresolved
head repository unresolved
base branch unresolved
push permission unknown
authentication unavailable
pull request not found
pull request identity ambiguous
head branch moved remotely
head branch deleted
base branch deleted
pull request retargeted
checks stale
test merge stale
mergeability pending
merge conflict
review requirement incomplete
merge queue required
queue checks pending
branch update blocked by policy
fork topology cannot represent incremental stacked reviews
```

These conditions map to stable core error codes such as:

```text
provider-unbound
provider-route-incomplete
provider-authentication-unavailable
remote-newer
remote-diverged
remote-outcome-unknown
verification-stale
landing-blocked
concurrent-record-update
```

GitHub-specific detail codes SHOULD be namespaced, for example:

```text
github.pull-request-identity-ambiguous
github.head-branch-deleted
github.base-branch-deleted
github.stacked-fork-topology-unrepresentable
github.merge-group-stale
github.branch-update-blocked-by-ruleset
```

Example:

```text
GitHub provider is configured, but a stacked review cannot be created.

Base repository:
  organization/project

Head repository:
  user/project

Reason:
  step 2 requires step 1's branch as its pull-request base, but that
  branch exists only in the head fork and is not available as a base
  branch in organization/project.

Available mappings:
  aggregate
  cumulative
  explicit fork-hosted chain
```

---

## 30. Security

### 30.1 Passive discovery

Passive discovery must not:

* Contact a remote server.
* Invoke authentication.
* Display tokens.
* Execute repository hooks.
* Parse arbitrary repository files as executable provider configuration.
* Trust a repository-controlled executable.
* Push a test ref.

---

### 30.2 Remote URLs

Remote URLs are untrusted input.

They must be:

* Parsed structurally.
* Bounded in length.
* Validated before display.
* Passed as process arguments rather than shell fragments.
* Prevented from injecting command-line options.

---

### 30.3 Pull-request content

Titles, descriptions, comments, reviewer names, labels, check output, and URLs returned by GitHub are untrusted display data.

They must not be evaluated as:

* Shell commands.
* Refnames.
* Local file paths.
* Provider configuration.
* Staircase identity.

---

### 30.4 Force updates

Every non-fast-forward remote update requires:

```text
expected old remote OID
new local OID
explicit associated review route
```

No generic `--force` option may bypass the lease.

---

### 30.5 Uncertain mutations

After a timeout or interrupted network response, the provider must query remote state before retrying:

* Branch creation.
* Branch update.
* Pull-request creation.
* Base retargeting.
* Merge.
* Queue enrollment.
* Auto-merge activation.

Blind retry is prohibited when the first operation may have succeeded.

---

## 31. Interaction with Implicit Staircases

### 31.1 Read-only use

An implicit staircase MAY use the GitHub provider for:

* Passive route discovery.
* `review plan`.
* Existing pull-request discovery.
* Read-only exact-revision comparison.
* Provider verification when the pull-request identity is supplied or resolved unambiguously.
* Aggregate review comparison.

None of these operations creates lineage or stable step identity.

---

### 31.2 Adoption triggers

Creation or retention of a durable pull-request association, stable remote-head mapping, stacked or cumulative topology, persistent policy, or stepwise landing state requires management as specified in Section 27.3 and the core automatic-adoption rules.

When automatic adoption occurs, the triggering review operation and adoption are one logical operation. Failure before durable mutation SHOULD remove provisional adoption state; failure after uncertain remote mutation MUST retain enough managed recovery state to reconcile safely.

---

### 31.3 Ephemeral aggregate publication

An ephemeral aggregate upload is allowed only under the restrictions in Section 27.3. It does not make a pull request discoverable as the same Staircase review in a later invocation unless the user later attaches it explicitly.

## 32. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 32.1 GitHub is not a workspace provider

A GitHub remote does not redefine the containing workspace.

### 32.2 Provider applicability and review route are distinct

The provider may be bound while base, head, or destination remains unresolved.

### 32.3 Pull-request identity is base-repository scoped

A number alone is insufficient outside an already resolved base repository.

### 32.4 Head OID is the exact review revision

Checks and verification are attached to exact OIDs.

### 32.5 Local branch names and remote review branches are separate

Sequential local renaming does not silently rename an established pull-request head branch.

### 32.6 Pull-request mapping is explicit

Aggregate, stacked, and cumulative mappings are not interchangeable.

### 32.7 Fork topology is modeled honestly

The provider does not pretend that branches in a fork are upstream base branches.

### 32.8 Non-fast-forward pushes are lease-protected

Unconditional force push is prohibited.

### 32.9 Checks, statuses, reviews, and mergeability remain distinct

No single green indicator is silently promoted to complete verification.

### 32.10 Verification subject is typed

Head, test merge, merge group, and landed commit evidence are not conflated.

### 32.11 Merge method affects staircase repair

Squash and rebase landing may require upper-step restacking.

### 32.12 Upper pull requests do not land before dependencies

The provider enforces staircase order unless an explicit policy changes the topology.

### 32.13 Remote branch deletion respects dependencies

A branch used as an active pull-request base is retained.

### 32.14 Network discovery is explicit

Ordinary local listing does not authenticate or query GitHub by default.

### 32.15 Uncertain remote results are reconciled

Potentially successful mutations are not blindly repeated.

### 32.16 Durable associations are record-CAS protected

A mapping or association change cannot overwrite a concurrent structure, metadata, lifecycle, policy, or review-association update.

### 32.17 Archive is local by default

Local archive or unarchive does not implicitly mutate pull requests, merge queues, auto-merge state, labels, or remote branches.

### 32.18 Review publication and Staircase transport are separate

Pull-request publication does not transport Staircase records, and Staircase transport does not create pull requests.

### 32.19 Provider observations do not redefine local structure

Checks, approvals, branch state, and mergeability are evidence against exact subjects and routes; they do not silently change cuts, steps, lineage, or lifecycle.

### 32.20 Core command and output contracts remain authoritative

Provider options specialize `review`, `verify`, `land`, `policy`, and diagnostics without introducing incompatible aliases or machine-output shapes.

---

## 33. Example: Standalone Repository with One GitHub Remote

Local configuration:

```text
remote:
  git@github.com:organization/project.git
```

First command:

```console
git staircase list
```

Automatic workspace configuration:

```text
workspace:
  core.git

providers:
  GitHub
    repository candidate: github.com/organization/project
    review route: incomplete
```

No network request is made.

If no staircases exist:

```text
No staircases.
```

---

## 34. Example: Fork and Upstream Remotes

Local remotes:

```text
git@github.com:jomo/project.git
git@github.com:organization/project.git
```

Passive probe:

```text
GitHub provider:
  repository candidates:
    github.com/jomo/project
    github.com/organization/project

  roles:
    unresolved pending network validation or explicit configuration
```

After validation:

```text
head repository:
  jomo/project

base repository:
  organization/project

relationship:
  fork
```

The provider recommends aggregate review unless a valid stacked topology is explicitly available.

---

## 35. Example: Same-Repository Stacked Pull Requests

Staircase:

```text
feature-1 -> C1
feature-2 -> C2
feature   -> C3
```

Review topology:

```text
PR 101:
  base: main
  head: staircase/<lineage>/<step-1>

PR 102:
  base: staircase/<lineage>/<step-1>
  head: staircase/<lineage>/<step-2>

PR 103:
  base: staircase/<lineage>/<step-2>
  head: staircase/<lineage>/<step-3>
```

A local split renumbers `feature-2` to `feature-3`, but the existing remote review branch bound to the step ID remains unchanged.

---

## 36. Example: Squash Landing of the Bottom Step

Before landing:

```text
main:
  B

step 1:
  B---A1

step 2:
  B---A1---A2
```

GitHub squash-merges step 1:

```text
main:
  B---Q1
```

The original `A1` is not necessarily an ancestor of `Q1`.

The provider:

1. Records the squash result.
2. Restacks step 2 onto `Q1`.
3. Produces rewritten `A2'`.
4. Lease-updates the stable remote head branch.
5. Retargets the second pull request to `main`.
6. Marks previous checks and reviews stale where applicable.

---

## 37. Summary

The GitHub provider translates between:

```text
Staircase concepts:
  lineage
  ordered steps
  structure and record revisions
  cut OIDs
  stable step IDs
  local primary branches
  verification contracts
  landing order

GitHub concepts:
  hosted repositories
  base and head branches
  pull requests
  check runs
  commit statuses
  branch policies
  merge queues
  merge methods
```

Its default architectural role is:

```text
core.git or workspace provider
    determines workspace and local integration context
             │
             ▼
GitHub provider
    identifies hosted repositories
    resolves base and head repository routes
    publishes stable remote review branches with leases
    creates, attaches, or reconciles pull requests
    maps checks and reviews to exact OIDs
    lands reviews while preserving staircase order
```

The governing rule is:

> GitHub pull requests name review relationships between branches. Staircase supplies the durable identity and dependency structure that those relationships do not provide on their own.

---

# Appendix A: User journeys

## A.1 Conventions used in the transcripts

This appendix is normative for command names, operation boundaries, selector behavior, adoption behavior, and the material facts that GitHub-provider output must communicate.

Concrete OIDs, UUIDs, repository IDs, pull-request numbers, queue positions, and URLs are illustrative. Human output MAY vary in spacing and abbreviation, but it MUST preserve every material fact shown. JSON and porcelain modes remain governed by the consolidated core schemas and use full typed identifiers.

Lines beginning with `$` are commands. Unless explicitly identified as ordinary Git, they are `git staircase` commands. Network operations are explicit. `git staircase list` and passive provider bootstrap remain offline.

The transcripts use SHA-1 abbreviations only for readability. Implementations MUST support the repository's configured object format.

---

## A.2 Journey 1: Publish a same-repository stacked pull-request chain

### A.2.1 Starting state

A standalone repository has `refs/remotes/github/main` at `a100000`. Three local branches form one implicit sequential staircase:

```text
checkout-1 -> b110000  Extract payment request model
checkout-2 -> c120000  Add payment authorization service
checkout   -> d130000  Wire the checkout UI
```

One GitHub remote is locally configured for `github.com/acme/store`. The first listing passively binds repository routing but performs no network request:

```console
$ git staircase list
[stderr] Configured Staircase workspace:
[stderr]   workspace:           core.git
[stderr]   repository-routing:  github
[stderr]   review:              github
[stderr]
[stdout] checkout  3 steps  clean  sequential  (implicit)
```

The developer previews a stacked topology. Planning may query GitHub because review planning is an explicit provider operation, but it does not adopt or mutate:

```console
$ git staircase review plan checkout --mapping stacked
GitHub review plan for 'checkout' (implicit)
  structural key: implicit@8a26e35f6c91
  installation: github.com
  base repository: acme/store
  base branch: refs/heads/main
  base OID: a100000
  head repository: acme/store
  mapping: stacked
  remote-head naming: stable after adoption

  step  local cut  planned remote head                              base
  1     b110000    staircase/<new-lineage>/<new-step-1>             main
  2     c120000    staircase/<new-lineage>/<new-step-2>             staircase/<new-lineage>/<new-step-1>
  3     d130000    staircase/<new-lineage>/<new-step-3>             staircase/<new-lineage>/<new-step-2>

Actions:
  publish 3 remote branches
  create 3 pull requests
  persist 3 review associations
  adoption required: durable stacked topology and review identity

No local or remote mutation performed.
```

Creating the reviews automatically adopts the staircase because GitHub review identity and stable remote-head mappings must survive later local renumbering:

```console
$ git staircase review create checkout --mapping stacked
Adopted implicit staircase 'checkout'.
  reason: durable GitHub stacked review associations
  lineage: 7a31c92d-7f12-4b84-b818-c0c2767d4df1
  steps: 3

Published GitHub review topology.
  installation: github.com
  base repository: acme/store
  head repository: acme/store

  step  pull request  head OID  base branch                         result
  1     #701          b110000   main                                created
  2     #702          c120000   staircase/7a31c9/5a91e2            created
  3     #703          d130000   staircase/7a31c9/43d8b7            created

  structure revision: 4e61a7c
  record revision: 9bf2d10
```

The provider compares exact local and remote revisions rather than branch names alone:

```console
$ git staircase review status checkout
checkout
  provider: github
  mapping: stacked
  destination: github.com/acme/store refs/heads/main

  step  pull request  local     remote head  base                              sync       review   checks
  1     #701          b110000   b110000      main                              current    pending  pending
  2     #702          c120000   c120000      staircase/7a31c9/5a91e2          current    pending  pending
  3     #703          d130000   d130000      staircase/7a31c9/43d8b7          current    pending  pending
```

A later local split inserts a new step below the old second step. Sequential local branches are renumbered, but the established remote head for old step 2 remains bound to its stable step ID:

```console
$ git staircase split checkout:2 --at c115000
Split step 2 of 'checkout'.
  steps: 3 -> 4

  position  local branch  cut       GitHub association
  1         checkout-1    b110000   #701
  2         checkout-2    c115000   none
  3         checkout-3    c120000   #702; remote head unchanged
  4         checkout      d130000   #703; remote head unchanged

  existing pull-request identities preserved: 3
  new review identity required: 1
```

The new step review is created as a step-scoped mutation anchored to the enclosing record revision. The provider inserts it into the remote dependency chain without recreating the existing reviews:

```console
$ git staircase review create checkout:2
Created GitHub review for 'checkout' step 2.
  pull request: #704
  base: staircase/7a31c9/5a91e2
  head: staircase/7a31c9/9d04aa at c115000

Updated dependent topology:
  #702 base: staircase/7a31c9/5a91e2 -> staircase/7a31c9/9d04aa
  #702 head branch: unchanged
  #703 base and head branches: unchanged
  pull requests recreated: 0
  record revision: 2f710a8 -> 7bc122e
```

This journey demonstrates the separation of local positional branch names, stable step IDs, remote review-branch locators, pull-request identity, and exact head OIDs.

---

## A.3 Journey 2: Reject an impossible fork stack and publish an aggregate pull request

A contributor has two remotes:

```text
personal  -> github.com/alex/parser
upstream  -> github.com/acme/parser
```

The local implicit staircase `unicode` has three steps. Network validation proves that `alex/parser` is a fork of `acme/parser`, that the contributor can push only to the fork, and that the intermediate fork branches cannot be selected as base branches of pull requests owned by `acme/parser`.

The requested incremental upstream stack is rejected without adoption or remote mutation:

```console
$ git staircase review plan unicode --mapping stacked \
    --github-base github.com/acme/parser \
    --base-branch main \
    --github-head github.com/alex/parser
error: GitHub cannot represent the requested incremental stacked topology
  code: provider-route-incomplete
  detail: github.stacked-fork-topology-unrepresentable

  base repository: github.com/acme/parser
  head repository: github.com/alex/parser
  step 2 requires step 1's head branch as its pull-request base, but that
  branch exists only in the fork and cannot be a base branch of a pull
  request owned by acme/parser.

Available mappings:
  aggregate
  cumulative
  explicit fork-hosted chain with a separately planned upstream promotion review

No adoption or remote mutation performed.
```

The aggregate mapping is honest and representable:

```console
$ git staircase review plan unicode --mapping aggregate \
    --github-base github.com/acme/parser \
    --base-branch main \
    --github-head github.com/alex/parser
GitHub review plan for 'unicode' (implicit)
  mapping: aggregate
  base: acme/parser refs/heads/main at a200000
  head: alex/parser staircase/<new-lineage>/aggregate at d230000
  included steps: 3
  action: publish one branch and create one pull request
  adoption required: durable pull-request association

No local or remote mutation performed.
```

The contributor creates the durable aggregate review:

```console
$ git staircase review create unicode --mapping aggregate \
    --github-base github.com/acme/parser \
    --base-branch main \
    --github-head github.com/alex/parser
Adopted implicit staircase 'unicode'.
  reason: durable GitHub aggregate review association
  lineage: d51ec28c-4e7d-4d90-95fc-60b6c20c4ed4

Created pull request acme/parser#1842.
  base: acme/parser refs/heads/main at a200000
  head: alex/parser staircase/d51ec28c/aggregate at d230000
  exact review revision: d230000
  mapping: aggregate
  record revision: 70f1d42
```

The provider does not disguise this pull request as three isolated step reviews. One pull request reviews the aggregate top.

---

## A.4 Journey 3: Reconcile an upload whose network result is unknown

A managed aggregate staircase `search-cache` is associated with `acme/search#944`. Its stable remote head branch currently points to `e310000`. A local rebase produces `e410000`, making the review `local-newer` and all previous exact-revision verification stale.

```console
$ git staircase review upload search-cache
Uploading 'search-cache' to GitHub.
  pull request: acme/search#944
  remote branch: staircase/24b90a/aggregate
  expected old remote OID: e310000
  new head OID: e410000

error: the network response ended before the branch and pull-request state could be confirmed
  code: remote-outcome-unknown
  detail: github.branch-update-result-unknown
  operation: github-upload 8e45cf19

The update may have succeeded. It will not be repeated blindly.
Next: git staircase review reconcile search-cache
```

The durable journal retains the exact old and new OIDs. Reconciliation queries the canonical pull-request identity and branch route, discovers that the push did succeed, and records the exact observed state without creating another pull request or pushing again:

```console
$ git staircase review reconcile search-cache
Reconciled GitHub operation 8e45cf19.
  pull request: acme/search#944
  remote branch OID: e410000
  pull-request head OID: e410000
  local planned OID: e410000
  result: previous upload succeeded

  synchronization: current
  verification: pending for exact head e410000
  record revision: 1e3a773 -> 5c604be
  additional remote mutations: none
```

If the branch had instead remained at `e310000`, reconciliation would mark the first upload as not applied and permit a newly planned retry. If it had moved to an unrelated OID, it would report `remote-newer` or `remote-diverged` and would not overwrite it.

---

## A.5 Journey 4: Squash-land the bottom pull request and repair the remaining chain

A managed three-step same-repository staircase `billing` has current pull requests:

```text
#810: main                         <- staircase/6b9221/step-a at b510000
#811: staircase/6b9221/step-a     <- staircase/6b9221/step-b at c520000
#812: staircase/6b9221/step-b     <- staircase/6b9221/step-c at d530000
```

All exact current heads satisfy policy. The developer lands only the bottom step by squash:

```console
$ git staircase land billing --through billing:1 --method squash
Landing 'billing' through step 1.
  provider: github
  pull request: acme/payments#810
  method requested: squash
  destination before: a500000
  reviewed head: b510000

GitHub landing result:
  method observed: squash
  destination after: q610000
  landed commit: q610000
  original reviewed commit reachable from destination: no

Repairing remaining staircase:
  integration anchor: a500000 -> q610000
  step 2: c520000 -> c620000
  step 3: d530000 -> d630000
  lease-updated remote heads: 2
  retargeted pull request #811: staircase/6b9221/step-a -> main
  pull request #812 remains based on staircase/6b9221/step-b
  deleted remote branch: staircase/6b9221/step-a, after #811 retarget was confirmed

Review consequences:
  #811 head changed and base changed: checks stale, review relevance re-evaluated
  #812 head changed: checks stale
  auto-merge and merge-queue state revalidated: none active

Remaining active steps: 2
  local primary branches renumbered: billing-1, billing
  structural state: clean
  structure revision: 9f18d02 -> ac9925d
  record revision: 3a8bd40 -> b5c113e
```

The next status shows that review identity survived while exact review revisions changed:

```console
$ git staircase review status billing
billing
  mapping: stacked
  integration anchor: q610000

  step  pull request  local     remote head  base  sync     review      checks
  1     #811          c620000   c620000      main  current  re-evaluate pending
  2     #812          d630000   d630000      staircase/6b9221/step-b
                                                     current  re-evaluate pending
```

The provider will not land #812 before #811. After current checks and review policy pass again, `git staircase land billing --stepwise --method squash` may continue bottom to top.

---

## A.6 Journey 5: Attach an existing pull request from detached `HEAD`, archive locally, and restore branchlessly

A developer is in a detached worktree at manifest or CI checkout `a700000`. Elsewhere in the repository, branch `audit` materializes one implicit step ending at `b710000`. An existing pull request `acme/security#412` has exact head `b710000` and base `main` at `a700000`.

Attaching the existing review requires durable identity, so the command adopts automatically:

```console
$ git staircase review attach audit \
    --provider github \
    --review github.com/acme/security#412
Adopted implicit staircase 'audit'.
  reason: durable existing GitHub review association
  lineage: 4e01b781-72d6-4ca6-ab26-aab9766dc854

Attached GitHub pull request.
  identity: github.com/acme/security#412
  base: acme/security refs/heads/main at a700000
  head: acme/security refs/heads/audit at b710000
  exact review revision: b710000
  mapping subject: aggregate staircase
  current worktree: detached at a700000; unchanged
  record revision: 77d89a0
```

The developer archives the local staircase. Archive is offline and does not touch GitHub:

```console
$ git staircase archive audit --reason "Security review paused"
Archived 'audit'.
  lineage: 4e01b781-72d6-4ca6-ab26-aab9766dc854
  removed owned active local branches: audit
  preserved archive cut refs: 1
  preserved GitHub association: acme/security#412
  GitHub pull request changed: no
  remote branch changed: no
```

Ordinary active listings no longer show it, while the pull request remains open remotely:

```console
$ git staircase list
No staircases.

$ git staircase list --archived
archive  audit  1 step  clean  GitHub acme/security#412
```

Later the developer restores the managed staircase without recreating the local branch:

```console
$ git staircase unarchive audit --branches=none
Unarchived 'audit'.
  lineage: 4e01b781-72d6-4ca6-ab26-aab9766dc854
  active local primary branches: none
  internal cut refs restored: 1
  GitHub association retained; remote validation required before mutation
```

An explicit status refresh revalidates the remote state:

```console
$ git staircase review status audit
Revalidated GitHub association.
  pull request: acme/security#412
  state: open
  local top: b710000
  remote head: b710000
  base OID: a700000
  synchronization: current
  branchless local staircase: yes
```

This journey demonstrates that detached `HEAD`, local branch materialization, managed structure, archive lifecycle, and GitHub pull-request identity are independent layers.