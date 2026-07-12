# Addendum G: GitHub Hosting, Pull Request, Verification, and Landing Provider

## 1. Status and Scope

This addendum defines the Staircase GitHub provider.

It depends on:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**
5. **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**

The provider may implement the following capabilities:

```text
repository-routing
review
review-identity
verification
review-transport
landing
```

It may provide integration-context candidates derived from an explicitly selected pull-request base branch, but it is not a general workspace or integration-context provider.

The provider supports:

* GitHub.com.
* GitHub Enterprise Server.
* Same-repository pull requests.
* Pull requests from forks.
* Aggregate pull requests.
* Branch-based chains of stacked pull requests.
* Status checks and check runs.
* Branch protection and ruleset requirements.
* Auto-merge.
* Merge queues.
* Merge, squash, and rebase landing methods.

The provider must remain usable in:

* A standalone Git repository.
* A provider-managed multi-repository workspace.
* An attached or detached worktree.
* A repository with several GitHub remotes.
* A repository with separate fetch, push, base, and fork remotes.

GitHub pull requests propose merging a head branch into a base branch, and may use a head branch from another repository in a fork network.

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
integration target source
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

## 7. Integration Context and Review Destination

### 7.1 Review destination as target evidence

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
* The correct target for every local branch.

The default branch may be used automatically only when a configured policy explicitly states that new reviews target the repository default.

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
  "installation_id": "github.com",
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

The head OID is required before verification evidence may be attached to a local staircase revision.

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

Under Addendum C, local names such as:

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

After a pull request is created, its remote head branch should remain stable for the lifetime of that review.

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

A workspace may instead use provider-generated remote branch names based on stable identity:

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
staircase revision
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
* Whether the staircase revision changed during planning.
* Whether a head branch moved remotely.
* Whether a pull request already exists for the intended branch pair.

---

### 13.3 Pull-request creation

A pull request must be created against exact planned:

```text
base repository
base branch
head repository
head branch
```

The local branch spelling is not substituted after planning without revalidation.

---

### 13.4 Existing pull-request discovery

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

### 13.5 Local pull-request refs

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

### 13.6 Association persistence

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
last query timestamp
```

Remote observations are cached evidence and must be revalidated before mutation.

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

GitHub-related verification may apply to different commits:

1. The pull-request head commit.
2. GitHub’s simulated test merge commit.
3. A merge-queue merge-group commit.
4. The eventual landed commit.
5. A local staircase prefix commit.

These subjects must not be conflated.

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

Checks for a previous head OID do not verify a rewritten staircase revision.

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

The provider must reconcile by patch or outcome relationship and then produce a new exact staircase revision.

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

This sequence is one managed Staircase operation.

It may span several remote transactions and therefore requires:

* A durable operation journal.
* Exact old and new OIDs.
* Resume.
* Abort where feasible.
* Reconciliation after uncertain network outcomes.

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

This permits the provider to compose naturally with the detached-workspace model in Addendum D.

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

* The local staircase revision changes.
* The head branch moves.
* The base branch moves.
* The pull request is retargeted.
* The authenticated account changes.
* Repository policy changes.
* The provider route changes.
* The cache exceeds configured age.

`git staircase list` may display cached state with an age marker but must not silently perform a network refresh.

---

## 27. Commands

Recommended commands include:

```console
git staircase provider github doctor

git staircase review plan auth
git staircase review create auth
git staircase review upload auth
git staircase review status auth
git staircase review reconcile auth
git staircase review open auth:2

git staircase verify auth --provider github

git staircase land auth
git staircase land auth --method merge
git staircase land auth --method squash
git staircase land auth --method rebase
git staircase land auth --queue
```

Mapping-specific commands may include:

```console
git staircase review plan auth --mapping aggregate
git staircase review plan auth --mapping stacked
git staircase review plan auth --mapping cumulative
```

Route overrides may include:

```console
git staircase review create auth \
    --github-base github.com/organization/project \
    --base-branch release \
    --github-head github.com/user/project
```

---

## 28. Diagnostics

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

## 29. Security

### 29.1 Passive discovery

Passive discovery must not:

* Contact a remote server.
* Invoke authentication.
* Display tokens.
* Execute repository hooks.
* Parse arbitrary repository files as executable provider configuration.
* Trust a repository-controlled executable.
* Push a test ref.

---

### 29.2 Remote URLs

Remote URLs are untrusted input.

They must be:

* Parsed structurally.
* Bounded in length.
* Validated before display.
* Passed as process arguments rather than shell fragments.
* Prevented from injecting command-line options.

---

### 29.3 Pull-request content

Titles, descriptions, comments, reviewer names, labels, check output, and URLs returned by GitHub are untrusted display data.

They must not be evaluated as:

* Shell commands.
* Refnames.
* Local file paths.
* Provider configuration.
* Staircase identity.

---

### 29.4 Force updates

Every non-fast-forward remote update requires:

```text
expected old remote OID
new local OID
explicit associated review route
```

No generic `--force` option may bypass the lease.

---

### 29.5 Uncertain mutations

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

## 30. Interaction with Implicit Staircases

### 30.1 Read-only planning

An implicit staircase may use the GitHub provider for:

* Route discovery.
* Review-plan preview.
* Existing pull-request discovery.
* Read-only verification.
* Aggregate review comparison.

---

### 30.2 Adoption triggers

Adoption is required before storing:

* Stable pull-request associations.
* Stable remote review branch mappings.
* A stacked pull-request topology.
* Review mapping policy.
* Landing policy.
* Durable upload recovery state.
* Cross-operation reconciliation state.
* Stepwise landing progress.

Creating or associating a pull request therefore normally adopts an implicit staircase automatically.

---

### 30.3 One-time aggregate upload

A strictly one-time aggregate pull-request creation could theoretically operate without adoption if no association is retained.

The provider should not make this the default because it would discard the review continuity needed for later updates.

---

## 31. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 31.1 GitHub is not a workspace provider

A GitHub remote does not redefine the containing workspace.

### 31.2 Provider applicability and review route are distinct

The provider may be bound while base, head, or destination remains unresolved.

### 31.3 Pull-request identity is base-repository scoped

A number alone is insufficient outside an already resolved base repository.

### 31.4 Head OID is the exact review revision

Checks and verification are attached to exact OIDs.

### 31.5 Local branch names and remote review branches are separate

Sequential local renaming does not silently rename an established pull-request head branch.

### 31.6 Pull-request mapping is explicit

Aggregate, stacked, and cumulative mappings are not interchangeable.

### 31.7 Fork topology is modeled honestly

The provider does not pretend that branches in a fork are upstream base branches.

### 31.8 Non-fast-forward pushes are lease-protected

Unconditional force push is prohibited.

### 31.9 Checks, statuses, reviews, and mergeability remain distinct

No single green indicator is silently promoted to complete verification.

### 31.10 Verification subject is typed

Head, test merge, merge group, and landed commit evidence are not conflated.

### 31.11 Merge method affects staircase repair

Squash and rebase landing may require upper-step restacking.

### 31.12 Upper pull requests do not land before dependencies

The provider enforces staircase order unless an explicit policy changes the topology.

### 31.13 Remote branch deletion respects dependencies

A branch used as an active pull-request base is retained.

### 31.14 Network discovery is explicit

Ordinary local listing does not authenticate or query GitHub by default.

### 31.15 Uncertain remote results are reconciled

Potentially successful mutations are not blindly repeated.

---

## 32. Example: Standalone Repository with One GitHub Remote

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

## 33. Example: Fork and Upstream Remotes

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

## 34. Example: Same-Repository Stacked Pull Requests

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

## 35. Example: Squash Landing of the Bottom Step

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

## 36. Summary

The GitHub provider translates between:

```text
Staircase concepts:
  lineage
  ordered steps
  cut OIDs
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
    publishes stable remote review branches
    creates or associates pull requests
    maps checks and reviews to exact OIDs
    lands reviews while preserving staircase order
```

The governing rule is:

> GitHub pull requests name review relationships between branches. Staircase supplies the durable identity and dependency structure that those relationships do not provide on their own.
