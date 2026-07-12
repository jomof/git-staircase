# Addendum E: `repo` Workspace Provider

## 1. Status and Scope

This addendum defines the behavior of the Staircase `repo` provider.

It depends on the provider and bootstrap model defined by:

* **Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts**

The `repo` provider implements the following capabilities:

```text
workspace
project-mapping
integration-context
workspace-hints
```

It may provide typed hints to:

```text
review
verification
transport
```

It does not itself implement Gerrit review or presubmit semantics.

The provider is intended to work with workspaces managed by the `repo` multi-repository tool while keeping the Staircase core independent of `repo`.

A `repo` manifest describes a client composed of multiple Git projects. Manifest project entries may define revisions, upstream refs, review destinations, remotes, and project paths.

---

## 2. Provider Identity

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
    "mutates_workspace": false
  }
}
```

---

## 3. Non-Goals

The provider does not:

* Upload changes.
* Interpret Gerrit Change-Ids.
* Query review status.
* Interpret Gerrit labels or submit requirements.
* Run presubmit.
* Call `repo sync`.
* Modify the active manifest.
* Modify files under `.repo`.
* Start, checkout, or abandon topic branches.
* Treat every manifest review endpoint as automatically reachable.
* Require the current project to have an attached branch.

The `repo` internal metadata directory is managed by `repo`; Staircase must not place its own configuration there or manually alter that state.

---

## 4. Passive Workspace Probe

### 4.1 Candidate detection

The provider searches for a valid enclosing `repo` client containing the current Git repository.

A high-confidence candidate requires:

1. A recognizable `repo` client root.
2. A valid active manifest.
3. Proof that the current Git repository corresponds to one manifest project.
4. A stable project path or identity.
5. No network access.
6. No workspace mutation.

A directory named `.repo` alone is insufficient if project membership cannot be established.

---

### 4.2 Root selection

If several enclosing `repo` clients are plausible:

* Select the innermost client that declares the current repository as a project.
* Respect submanifest relationships when available.
* Do not select solely by nearest directory name.
* Do not treat `.repo/repo`, `.repo/manifests`, or other internal Git repositories as ordinary source projects unless explicitly requested.

---

### 4.3 Symlinks and canonical paths

The provider must compare:

```text
current Git common directory
canonical project worktree path
manifest project path
workspace root
```

using canonicalized paths.

A symlinked current directory must not cause one physical project to be registered twice.

---

### 4.4 Probe output

Example:

```json
{
  "provider": "repo",
  "workspace_root": "/work/studio-main",
  "workspace_key": "repo:/work/studio-main",
  "claim": "authoritative",
  "confidence": "high",
  "current_project": {
    "name": "tools/vendor/example",
    "path": "tools/vendor/example",
    "git_common_dir": "/work/studio-main/.repo/projects/tools/vendor/example.git"
  },
  "evidence": [
    "valid repo client",
    "current Git repository matches one manifest project"
  ]
}
```

---

## 5. Obtaining Project Metadata

### 5.1 Prefer supported `repo` interfaces

The provider should prefer supported, machine-readable `repo` command output when the installed version supplies it.

When no suitable structured operation exists, the provider may use a controlled `repo forall` invocation to extract the environment for the current project.

`repo forall` exposes project identity and path, manifest remote, translated local revision, original manifest revision, upstream, review destination, and resolved fetch URL through variables such as:

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

---

### 5.2 Controlled execution

When invoking `repo`:

* The provider supplies the command.
* Repository data is not interpolated into shell source.
* Output is NUL-delimited or otherwise unambiguously encoded.
* The invocation targets only the current project where possible.
* Hooks unrelated to metadata inspection are not invoked.
* The operation has a timeout.
* Standard error is captured for diagnostics.
* A failure degrades the provider rather than corrupting workspace configuration.

---

### 5.3 Manifest parsing fallback

Direct manifest parsing may be used only as a versioned fallback.

The provider must account for:

* Default inheritance.
* Per-project overrides.
* Remote overrides.
* `extend-project`.
* Local manifests.
* Includes.
* Submanifests.
* Project path changes.
* Revision-locked manifests.

It must not parse only the top-level XML file and assume that it represents the effective manifest.

---

## 6. Project Identity

### 6.1 Provider project identity

The provider project identity is the effective manifest project name.

Example:

```text
tools/vendor/google/alt-lang/soong
```

It is distinct from:

* Filesystem path.
* Git remote name.
* Fetch URL.
* Gerrit numeric change ID.
* Staircase lineage ID.

---

### 6.2 Project path

The provider records the effective project path relative to the applicable client root.

Project name and project path may differ.

Both are preserved.

---

### 6.3 Duplicate project checkouts

A manifest may contain multiple checkouts of the same project name at different paths.

The provider must distinguish them using at least:

```text
workspace ID
project name
project path
Git common directory
```

Project name alone is insufficient as a local checkout identity.

---

## 7. Integration-Context Output

### 7.1 Typed outputs

The provider may return all of the following independently:

```text
workspace checkout anchor
manifest revision
translated local revision
upstream ref
review destination hint
```

These values must not be collapsed into one “branch.”

---

### 7.2 Detached workspace checkout

When the current project is detached and no transient Git operation is active, the provider returns the full `HEAD` OID as a workspace-checkout anchor candidate.

Example:

```json
{
  "workspace_checkout_anchor": {
    "oid": "ba19eb864d...",
    "source": "detached-project-head"
  }
}
```

This represents the exact commit currently materialized in the workspace.

It does not imply that `HEAD` is a moving target.

---

### 7.3 Manifest revision

The provider returns the manifest revision exactly as declared:

```text
REPO_RREV
```

It may be:

* A branch name.
* A full refname.
* A tag.
* An exact commit OID.
* Another supported revision expression.

The provider must not assume that it is always a branch.

---

### 7.4 Translated local revision

When available, the provider returns the locally usable translated revision:

```text
REPO_LREV
```

`repo` documents this as the manifest revision translated to a local tracking branch and recommends it when the revision is to be supplied to a local Git command.

The core resolves it to a full commit OID before use.

---

### 7.5 Upstream ref

The provider returns:

```text
REPO_UPSTREAM
```

when present.

An upstream ref is a locator for finding a revision in some manifest modes. It is not automatically:

* The exact workspace anchor.
* The review destination.
* The staircase target.
* The local branch upstream.

The manifest format gives `upstream` a distinct role, particularly for revision-locked manifests.

---

### 7.6 Candidate ordering

The provider returns evidence rather than selecting the final integration context unilaterally.

Recommended candidate order is:

1. Explicit exact manifest revision, when the manifest revision is an OID.
2. Detached project `HEAD`, as exact workspace checkout state.
3. Resolved translated local revision.
4. Resolved manifest revision.
5. Resolved upstream ref.

The Staircase core applies Addendum D’s authority and compatibility rules.

---

### 7.7 Attached development branches

When the current project is attached to a local development branch:

* The branch tip is not reported as the workspace integration anchor.
* The provider still reports manifest and translated revision candidates.
* Branch upstream information may be considered separately by the core.
* The current branch may be a staircase cut or tip.

---

### 7.8 Detached review checkout

A detached `HEAD` may represent a downloaded review or arbitrary historical commit.

The provider must return both:

```text
detached checkout candidate
manifest-derived candidate
```

when both exist.

If detached `HEAD` is also a staircase cut or is incompatible with the manifest candidate, the core must not assume it is the integration anchor.

---

## 8. Review and Verification Hints

### 8.1 Review endpoint hint

A manifest remote may specify a Gerrit review hostname used by `repo upload`.

The `repo` provider may emit:

```json
{
  "review_hint": {
    "kind": "gerrit",
    "endpoint": "review.example.com",
    "source": "manifest-remote-review"
  }
}
```

This is a typed hint.

The provider does not:

* Authenticate to the endpoint.
* Confirm server availability.
* Query Gerrit.
* Become the review provider.

---

### 8.2 Destination branch hint

The effective project `dest-branch` is returned as a review-destination hint.

If `dest-branch` is absent, the effective manifest revision may be returned as a fallback hint, reflecting `repo`’s manifest semantics. The manifest defines `dest-branch` as the branch used by `repo upload`, falling back to `revision` when not set.

The hint must retain provenance:

```json
{
  "destination_branch": {
    "value": "main",
    "source": "project-dest-branch"
  }
}
```

or:

```json
{
  "destination_branch": {
    "value": "main",
    "source": "manifest-revision-fallback"
  }
}
```

---

### 8.3 Remote and project hints

The provider may return:

```text
manifest remote name
resolved fetch URL
push URL, if known
project name
review endpoint
destination branch
```

These values allow a review provider to establish a project-specific route.

They do not authorize upload.

---

## 9. Automatic Binding

### 9.1 Workspace binding

The `repo` provider is automatically bound for `workspace` when:

* The provider is trusted.
* Probe succeeds locally.
* The current Git repository maps uniquely to a manifest project.
* No stronger explicit workspace configuration exists.

---

### 9.2 Project-mapping binding

`project-mapping=repo` is bound together with the workspace capability when project identity is available.

---

### 9.3 Integration-context binding

`integration-context=repo` is automatically bound when the provider returns at least one valid integration-context candidate.

The binding may remain valid even when a particular project has temporarily unresolved history.

---

### 9.4 Review provider hinting

A Gerrit-shaped manifest review hint may cause a separately installed Gerrit provider to be probed.

It does not directly set:

```text
review = gerrit
```

The Gerrit provider must independently accept the hint.

---

## 10. Manifest and Workspace Changes

### 10.1 Fingerprint

The provider discovery fingerprint should include:

```text
canonical client root
effective manifest identity
current manifest project revision
current project name and path
effective remote
effective revision
effective upstream
effective destination branch
effective review endpoint
provider version
```

Large manifest content need not be stored directly; a stable digest may be used.

---

### 10.2 Revalidation

Revalidation is required when:

* `repo init` changes the manifest branch or manifest file.
* `repo sync` changes the effective project mapping.
* Local manifests change.
* The project is moved.
* The current repository ceases to match the manifest project.
* A submanifest changes ownership of the project.
* Review or destination metadata changes.
* The provider version changes.

---

### 10.3 Project removed from manifest

If the current Git repository is no longer declared:

* Existing managed staircases remain local Git objects.
* The workspace provider binding becomes degraded for that project.
* The project is not silently reassigned.
* The core may temporarily use `core.git` semantics.
* Review and integration hints inherited from the former manifest become stale.

---

### 10.4 Workspace moved

If the workspace root moves but the provider-native identity and project mapping remain consistent:

* The workspace registry may update the canonical root.
* The workspace ID remains stable.
* Path-dependent fingerprints are refreshed.

---

## 11. No Automatic Workspace Mutation

The provider must never automatically run:

```text
repo init
repo sync
repo start
repo checkout
repo upload
repo abandon
repo prune
```

Provider bootstrap is observational.

A future explicit Staircase command may delegate to such operations, but that behavior belongs to a separate mutating capability and requires explicit invocation.

---

## 12. Example Bootstrap

Current state:

```text
workspace root:
  /projects/studio-main.bot

current project:
  tools/vendor/google/alt-lang/soong

HEAD:
  detached at ba19eb864d
```

Provider result:

```text
workspace:
  provider: repo
  root: /projects/studio-main.bot

project:
  name: tools/vendor/google/alt-lang/soong
  path: tools/vendor/google/alt-lang/soong

integration candidates:
  workspace checkout: ba19eb864d
  translated manifest revision: refs/remotes/m/studio-main

review hint:
  kind: gerrit
  endpoint: review.example.com
  destination: studio-main
```

The core may select:

```text
integration anchor:
  ba19eb864d

symbolic target:
  refs/remotes/m/studio-main
```

A separately bound Gerrit provider consumes the review hint.

---

## 13. Corner Cases

### 13.1 Revision-locked manifest

When `revision` is an exact OID:

* Preserve it as an exact candidate.
* Preserve `upstream` separately.
* Do not replace the OID with the upstream branch tip.

---

### 13.2 Manifest target advanced after checkout

When the translated manifest revision has advanced beyond detached `HEAD`:

```text
workspace checkout anchor:
  detached HEAD OID

moving manifest target:
  translated local revision
```

Both are returned.

The exact checkout remains reproducible; the moving target may indicate that the workspace is behind.

---

### 13.3 Local commit while detached

If detached `HEAD` contains a local commit not represented by the manifest target:

* Do not certify it as the manifest baseline.
* Return it only as current checkout evidence.
* Prefer a valid manifest-derived integration candidate.
* Report the discrepancy.

---

### 13.4 Multiple remotes

The provider uses the effective manifest remote for the project.

It does not assume that a remote named `origin` is authoritative.

---

### 13.5 Missing `repo` executable

If the provider implementation requires the `repo` executable and it is unavailable:

* Probe fails cleanly.
* The core may use single-Git mode.
* Existing cached provider facts are not treated as current without validation.

---

### 13.6 Corrupt or incomplete manifest

The provider reports a degraded candidate rather than guessing from directory layout alone.

---

### 13.7 Bare or mirror clients

Mirror clients do not have ordinary project worktrees.

The provider may identify the workspace, but integration-context and staircase-discovery behavior must reflect the absence of a normal checked-out project.

---

## 14. Normative Invariants

### 14.1 `repo` is a workspace provider

It is not implicitly a Gerrit review provider.

### 14.2 Project identity and path are separate

Neither may silently replace the other.

### 14.3 Detached project `HEAD` is first-class evidence

It is not required to have a local branch name.

### 14.4 Manifest revision, upstream, and destination are distinct

They must be returned as separately typed values.

### 14.5 The provider does not mutate `.repo`

Staircase configuration remains outside provider-owned metadata.

### 14.6 No network or sync during bootstrap

Automatic discovery is local and observational.

### 14.7 Review hints are hints

A Gerrit provider must independently accept and bind them.

### 14.8 No `origin` assumption

The effective manifest remote governs provider metadata.

### 14.9 Exact manifest OIDs remain exact

They are not replaced by moving refs.

### 14.10 Effective manifest semantics are required

The provider must account for inheritance, overrides, includes, local manifests, and submanifests rather than reading one shallow XML fragment.

---

## 15. Summary

The `repo` provider translates a multi-repository workspace into generic Staircase facts:

```text
workspace root
project identity
project path
exact checkout anchor
manifest revision
translated local revision
upstream ref
review endpoint hint
review destination hint
```

It does not perform review operations.

The governing rule is:

> The `repo` provider explains where the project lives and what the workspace selected; it does not decide what Gerrit means.
