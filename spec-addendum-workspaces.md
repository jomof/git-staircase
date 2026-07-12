# Addendum D: Automatic Workspace Discovery, Provider Bootstrap, and Integration Contexts

## 1. Status and Scope

This addendum supersedes the previous version of **Addendum D: Integration Contexts and Detached-HEAD Workspaces**.

It modifies:

1. **Git Staircase: Conceptual Specification**
2. **Addendum A: Implicit Staircases and Automatic Adoption**
3. **Addendum B: Staircase Names, Selectors, References, and Identifiers**
4. **Addendum C: Sequential Primary-Branch Naming for Linear Staircases**

It defines:

* Automatic discovery of the workspace containing the current Git repository.
* Automatic discovery and binding of applicable providers.
* Persistent but local workspace configuration.
* Capability-specific provider composition.
* Provider-free operation in an ordinary standalone Git repository.
* Integration-context resolution in attached and detached worktrees.
* Safe behavior when discovery is ambiguous or incomplete.
* Revalidation when workspace structure or provider evidence changes.
* Security constraints on provider probing and automatic configuration.

The intended behavior is:

> When any `git staircase` command is run in an unconfigured workspace, Staircase discovers the workspace and applicable trusted providers, records an unambiguous safe configuration, and then continues the original command.

No separate initialization command is required for the common case.

---

## 2. Goals

The bootstrap system must satisfy all of the following:

1. A standalone Git repository works without external providers.
2. A multi-repository workspace may be recognized automatically.
3. Several provider capabilities may be composed without creating one monolithic provider.
4. Provider discovery does not require network access.
5. Automatic configuration does not execute repository-supplied code.
6. Ambiguous detection does not silently select consequential semantics.
7. Read-only commands remain useful when some capabilities cannot be resolved.
8. Explicit user configuration always overrides automatic inference.
9. Detached `HEAD` is treated as a normal workspace state.
10. Workspace-manager, review-system, and verification semantics remain separate.

---

## 3. Architectural Model

### 3.1 Workspace

A **Staircase workspace** is the configuration scope that contains one or more Git repositories participating in a common development environment.

A workspace may be:

* One ordinary Git worktree.
* One bare Git repository.
* A collection of Git repositories managed by another tool.
* A nested workspace within a larger workspace.
* A virtual workspace identified by a provider.

A workspace has:

```text
workspace ID
workspace kind
canonical root or provider-native locator
participating repositories
capability bindings
configuration provenance
discovery fingerprint
```

---

### 3.2 Provider implementation

A **provider implementation** is an installed extension that may implement one or more typed capabilities.

Provider names identify implementations:

```text
repo
gerrit
github
custom-workspace
custom-presubmit
```

A provider name alone does not define how it is used.

---

### 3.3 Capability binding

A **capability binding** assigns one provider implementation to one capability within a workspace or repository.

Defined capability classes include:

```text
workspace
project-mapping
integration-context
review
verification
transport
```

Example:

```text
workspace             = repo
project-mapping       = repo
integration-context   = repo
review                = gerrit
verification          = gerrit
transport             = git
```

A single provider may implement several capabilities.

Capabilities remain logically separate even when implemented by the same executable.

---

### 3.4 Provider profile

A **provider profile** is configuration shorthand that populates several capability bindings.

For example:

```text
profile repo+gerrit
```

may expand to:

```text
workspace             = repo
project-mapping       = repo
integration-context   = repo
review                = gerrit
verification          = gerrit
```

The profile is not itself a provider.

The stored configuration must contain the expanded capability bindings so that any one capability may later be replaced independently.

---

### 3.5 Built-in single-Git mode

The Staircase core always provides a built-in fallback workspace mode:

```text
workspace             = core.git
project-mapping       = core.git
integration-context   = core.git
transport             = git
```

This mode requires no external provider.

It treats the current Git repository as a one-project workspace.

The absence of a review or verification provider does not prevent:

* Staircase discovery.
* Listing.
* Inspection.
* Reshaping.
* Rebase and restack operations.
* Local verification commands.
* Managed staircase storage.

---

## 4. Workspace Configuration Storage

### 4.1 Local, non-versioned configuration

Automatically generated workspace configuration must be local to the user.

It must not be committed into source repositories by default.

The implementation should use a user-local Staircase registry, conceptually:

```text
<user-config>/git-staircase/workspaces/
<user-state>/git-staircase/workspaces/
```

A workspace record contains:

```text
workspace ID
canonical root
provider-native workspace key
capability bindings
binding provenance
discovery fingerprint
last successful validation
```

---

### 4.2 Do not write into provider-owned metadata

The core must not place its configuration inside directories owned by another workspace manager unless that provider explicitly defines a safe extension location.

Automatic configuration must not modify:

* Workspace manifests.
* Git remotes.
* Workspace-manager internal state.
* Review-system configuration.
* Source-controlled files.

---

### 4.3 Workspace lookup

On each command, Staircase attempts to locate an existing workspace record using:

1. An explicit `--workspace` selector.
2. A worktree or repository-local workspace ID pointer, if present.
3. A registered workspace whose canonical root contains the current repository.
4. A provider-native workspace identity returned by a passive probe.
5. Automatic bootstrap.

When several registered workspace roots contain the current path, the most specific applicable registered workspace is selected unless a provider declares a parent-child workspace relationship.

---

### 4.4 Configuration provenance

Every capability binding records how it was established:

```text
explicit
profile
auto-discovered
inherited
default
```

Example:

```text
workspace = repo
origin = auto-discovered
evidence = repo-client-membership
```

Explicit and profile bindings must not be replaced automatically.

Automatically discovered bindings may be revalidated or replaced under the rules in Section 13.

---

## 5. Trusted Provider Discovery

### 5.1 Installed providers

The core discovers provider implementations from trusted installation registries.

Trusted implementations include:

* Providers bundled with Staircase.
* Providers installed into designated system or user provider directories.
* Providers explicitly registered by the user.
* Providers explicitly named on the command line.

The core must not automatically execute every executable on `PATH` whose name resembles a provider.

Unregistered `PATH` candidates may be displayed by a diagnostic command, but they require explicit approval before automatic execution.

---

### 5.2 Provider descriptor

Each provider exposes a static descriptor containing:

```json
{
  "protocol_version": 1,
  "name": "example",
  "version": "1.2.3",
  "capabilities": [
    "workspace",
    "integration-context"
  ],
  "probe": {
    "passive": true,
    "network": false,
    "mutates_workspace": false
  }
}
```

The descriptor is read before any probe is invoked.

---

### 5.3 Passive probe requirements

A provider probe eligible for automatic bootstrap must:

* Be read-only.
* Avoid network access.
* Avoid authentication.
* Avoid modifying Git refs or configuration.
* Avoid changing workspace-manager state.
* Avoid running repository-provided hooks.
* Avoid evaluating shell fragments from repository data.
* Complete within a bounded time.
* Produce bounded, schema-validated output.
* Explain the evidence supporting each candidate.

A provider requiring network access may still be configured manually, but it is not eligible for passive automatic bootstrap.

---

## 6. Bootstrap Protocol

Every `git staircase` command runs the following protocol before executing its command-specific behavior.

### 6.1 Phase 1: Establish Git context

The core resolves:

```text
current directory
worktree root, if any
Git common directory
repository object format
HEAD state
active Git operation state
```

If the command does not require a Git repository, it may continue without one.

---

### 6.2 Phase 2: Locate existing workspace configuration

If a valid workspace record is found:

1. Load its capability bindings.
2. Validate that the current repository remains a member.
3. Revalidate any stale automatic bindings.
4. Continue the original command.

No provider-selection probe is needed unless the configuration is stale, incomplete, or explicitly refreshed.

---

### 6.3 Phase 3: Probe workspace providers

If no workspace configuration exists, the core invokes all trusted passive workspace probes.

Each probe may return zero or more workspace candidates.

A candidate contains:

```json
{
  "provider": "example",
  "workspace_root": "/canonical/path",
  "workspace_key": "provider-native-key",
  "current_project": {
    "path": "project/path",
    "identity": "project-id"
  },
  "claim": "authoritative",
  "confidence": "high",
  "evidence": [
    "recognized workspace metadata",
    "current Git repository is a declared project"
  ],
  "fingerprint": "..."
}
```

The core also creates the built-in `core.git` fallback candidate.

---

### 6.4 Phase 4: Select the workspace candidate

Candidate selection follows this order:

1. Explicit command-line selection.
2. Existing registered workspace.
3. A provider candidate proving that the current Git repository is a member of a larger managed workspace.
4. A provider candidate proving ownership of the current Git repository itself.
5. The built-in `core.git` candidate.

The built-in candidate never creates ambiguity with a stronger provider candidate. It is a fallback.

When several non-core providers make incompatible claims:

* Prefer an explicitly declared nested workspace containing the current project.
* Otherwise require disambiguation.
* Do not select by provider name, filesystem enumeration order, or arbitrary confidence arithmetic.

Read-only commands may continue using `core.git` as a temporary fallback while reporting the unresolved provider ambiguity.

Commands requiring workspace-wide semantics must fail until the ambiguity is resolved.

---

### 6.5 Phase 5: Probe dependent capabilities

After selecting the workspace candidate, the core supplies its typed output to other trusted providers.

Examples include:

```text
workspace root
current project identity
Git remotes
manifest-derived target hints
review endpoint hints
verification hints
```

Review, verification, and transport providers then perform passive applicability probes.

Providers may consume hints from another provider, but they may not alter the meaning of those hints.

For example:

```text
repo provider:
  review endpoint hint = review.example.com

gerrit provider:
  interprets the hint as a Gerrit review endpoint
```

The workspace provider does not thereby become a Gerrit provider.

---

### 6.6 Phase 6: Bind unambiguous capabilities

A capability is automatically bound when:

1. The provider is trusted.
2. Its probe is passive.
3. The evidence is local.
4. Exactly one materially distinct high-confidence candidate exists.
5. No explicit binding conflicts.
6. Recording the binding does not itself initiate network or mutating behavior.

Several capabilities may be bound during the same bootstrap.

Example:

```text
workspace             = repo
project-mapping       = repo
integration-context   = repo
review                = gerrit
verification          = gerrit
```

---

### 6.7 Phase 7: Persist configuration

The core creates a workspace record containing:

```text
workspace ID
selected workspace candidate
expanded capability bindings
provider versions
evidence fingerprints
binding provenance
canonical root
project membership information
```

Configuration publication must use:

* A workspace-registry lock.
* Write-to-temporary-file.
* Atomic replacement.
* Compare-and-swap behavior when another process bootstraps concurrently.

If another process created an equivalent record, the current command adopts it.

If the competing record is incompatible, bootstrap fails without overwriting it.

---

### 6.8 Phase 8: Continue the original command

The command that triggered bootstrap continues automatically.

Example:

```console
$ git staircase list
Configured Staircase workspace:
  workspace: repo
  review: gerrit
  verification: gerrit

feature  3 steps  clean  (implicit)
```

The configuration message should be shown only when configuration changes.

In porcelain or JSON mode, it must be represented as structured metadata rather than unsolicited human text.

---

## 7. Ambiguous and Partial Bootstrap

### 7.1 Ambiguous workspace provider

If two specialized providers make incompatible workspace claims:

* Do not persist either binding.
* Continue in temporary `core.git` mode for commands that remain meaningful.
* Report the ambiguity.
* Require explicit selection for workspace-wide commands.

---

### 7.2 Ambiguous review provider

If workspace discovery succeeds but review-provider discovery is ambiguous:

* Persist the workspace binding.
* Leave `review` unbound.
* Continue all provider-independent behavior.
* Fail only commands that require review semantics.

---

### 7.3 Provider unavailable

If evidence indicates a larger workspace but its provider is unavailable, the core is not required to recognize that workspace type.

The current repository remains usable through `core.git`.

A diagnostic may recommend an applicable known provider when the evidence was produced without executing an unavailable provider.

---

### 7.4 Noninteractive behavior

Bootstrap must not require interactive confirmation when configuration is unambiguous and auto-configurable.

In an ambiguous case:

* Noninteractive commands use safe fallback behavior or fail narrowly.
* They must not wait for input.
* They must not choose a consequential provider arbitrarily.

---

## 8. Explicit Controls

Recommended options include:

```console
git staircase --no-bootstrap <command>
git staircase --no-configure <command>
git staircase --workspace <workspace-id> <command>
git staircase --workspace-provider <provider> <command>
git staircase --review-provider <provider> <command>
git staircase --provider-profile <profile> <command>
git staircase --workspace-mode=single-git <command>
```

Semantics:

```text
--no-bootstrap
    Do not probe providers or create configuration.

--no-configure
    Providers may be probed for this invocation, but no configuration
    is persisted.

--workspace-mode=single-git
    Ignore enclosing multi-repository workspace candidates for this invocation.
```

---

## 9. Integration Context

### 9.1 Integration context is not necessarily a branch

A staircase requires an ancestor-closed integration set, not a checked-out integration branch.

For the common single-anchor case:

[
U = \operatorname{Ancestors}(T)
]

where (T) is an exact integration-anchor commit.

The anchor may be obtained from:

* Explicit `--onto`.
* Managed staircase metadata.
* A workspace provider.
* A configured target.
* A remote-tracking ref.
* Detached `HEAD`.
* Branch upstream information.

---

### 9.2 Resolution order

For a selected staircase, integration context is resolved in this order:

1. Explicit command input.
2. Managed staircase descriptor.
3. Interrupted staircase operation state.
4. Explicit worktree or repository configuration.
5. Bound integration-context provider.
6. Applicable branch upstream configuration.
7. Eligible detached `HEAD`.
8. Unique compatible remote-default evidence.
9. No resolved context.

A weaker source must not overwrite a stronger source.

---

### 9.3 Detached `HEAD`

Detached `HEAD` is eligible as an exact integration anchor when:

* It resolves to a commit.
* No transient Git operation makes it unreliable.
* It is not clearly the selected staircase’s work tip or cut.
* It is compatible with the staircase graph.

When used, the persisted exact value is the full OID.

The literal expression `HEAD` is never stored as moving target intent.

---

### 9.4 Provider-supplied context

An integration-context provider may return:

```json
{
  "anchor_oid": "<full-oid>",
  "symbolic_target": "refs/remotes/example/main",
  "mode": "workspace",
  "provenance": "provider-name"
}
```

The core independently verifies:

* Object existence.
* Commit type.
* Graph compatibility.
* Repository object format.

A provider cannot declare an arbitrary non-commit value to be an integration anchor.

---

### 9.5 Review destination remains separate

The following may all differ:

```text
workspace integration anchor
moving integration target
review destination branch
upload transport ref
current HEAD
```

No provider may collapse these concepts without returning separate typed fields.

---

## 10. `git staircase list`

### 10.1 Best-effort listing

`git staircase list` must not require one repository-wide integration boundary before listing anything.

It must:

1. List managed staircases from their descriptors.
2. Discover implicit staircase candidates.
3. Resolve integration context per candidate.
4. List candidates whose context is resolved.
5. Report unresolved candidates diagnostically.
6. Return success if listing itself completed.

---

### 10.2 Empty result

If no staircases exist:

```text
No staircases.
```

is a successful result even when no integration context was required or resolved.

---

### 10.3 Strict mode

A strict mode may require all candidates to resolve:

```console
git staircase list --strict
```

Unresolved or ambiguous candidates then produce a nonzero exit status.

---

## 11. Provider Protocol

### 11.1 Operations

A provider protocol should support at least:

```text
describe
probe-workspace
probe-capability
resolve-project
resolve-integration-context
validate-binding
```

Capability-specific providers may additionally support:

```text
review-plan
review-upload
review-query
verification-query
transport-plan
```

---

### 11.2 Input

Provider input contains only typed data required for the operation.

Example:

```json
{
  "protocol_version": 1,
  "operation": "probe-capability",
  "capability": "review",
  "cwd": "/workspace/project",
  "git_common_dir": "/workspace/.git-state/project",
  "workspace": {
    "provider": "repo",
    "root": "/workspace",
    "project_id": "tools/example"
  },
  "hints": {
    "review_endpoint": "review.example.com",
    "destination_branch": "main"
  },
  "network_allowed": false
}
```

---

### 11.3 Output

A candidate output must distinguish:

```text
fact
hint
inference
requirement
```

Example:

```json
{
  "applicable": true,
  "confidence": "high",
  "facts": {
    "provider_kind": "gerrit"
  },
  "hints": {
    "endpoint": "review.example.com"
  },
  "requirements": {
    "network_for_status": true,
    "authentication_for_upload": true
  },
  "evidence": [
    "workspace provider supplied an explicit Gerrit review endpoint"
  ]
}
```

---

## 12. Security and Trust

### 12.1 Provider execution trust

Automatic bootstrap may execute only providers installed in trusted locations or explicitly approved by the user.

Repository-local executable provider declarations are not trusted automatically.

---

### 12.2 Passive means passive

A passive probe must not:

* Fetch.
* Contact a review server.
* Run presubmit.
* Upload code.
* Modify a manifest.
* Modify `.git/config`.
* Create refs.
* Install hooks.
* Read credentials unnecessarily.

---

### 12.3 Provider hints are untrusted data

All provider output must be:

* Schema validated.
* Size limited.
* Path normalized.
* OIDs resolved independently.
* Refnames validated.
* URLs parsed rather than shell-expanded.
* Rejected when they escape the declared workspace scope.

---

### 12.4 No shell interpolation

Provider commands and returned values must be passed as structured arguments.

Neither repository paths nor provider output may be interpolated into a shell command string.

---

## 13. Revalidation

### 13.1 Discovery fingerprint

Every automatic binding records an evidence fingerprint.

The fingerprint may include:

```text
provider version
canonical workspace root
provider-native workspace key
project mapping identity
manifest or workspace metadata identity
relevant Git remote configuration
review endpoint hint
```

---

### 13.2 Revalidation triggers

Automatic bindings are revalidated when:

* The provider version changes.
* The workspace root moves.
* The current repository is no longer a member.
* Provider metadata changes.
* Relevant remotes change.
* The workspace manifest changes.
* An explicitly requested refresh occurs.
* The previous validation exceeds a configured age.

---

### 13.3 Updating automatic bindings

An automatic binding may be updated automatically only when:

* Its replacement is unambiguous.
* No explicit binding conflicts.
* The change is based on passive local evidence.
* The original command does not thereby gain unexpected network or mutation behavior.

The change is recorded and reported.

---

### 13.4 Explicit bindings are sticky

An explicit or profile binding is never silently replaced.

If it becomes invalid, the workspace enters a degraded configuration state and reports the conflict.

---

## 14. Workspace Commands

Recommended commands include:

```console
git staircase workspace show
git staircase workspace discover
git staircase workspace providers
git staircase workspace refresh
git staircase workspace doctor
git staircase workspace configure
git staircase workspace forget
```

`discover` performs passive detection without necessarily persisting it.

`refresh` re-runs discovery and updates eligible automatic bindings.

`doctor` explains:

* Workspace selection.
* Bound providers.
* Missing capabilities.
* Rejected candidates.
* Stale evidence.
* Provider health.

---

## 15. Examples

### 15.1 Standalone Git repository

First command:

```console
$ git staircase list
```

Bootstrap result:

```text
Configured Staircase workspace:
  workspace: single Git repository
  root: /work/widget

No staircases.
```

Stored bindings:

```text
workspace             = core.git
project-mapping       = core.git
integration-context   = core.git
transport             = git
```

No review provider is required.

---

### 15.2 Multi-repository workspace with review provider

First command from one child repository:

```console
$ git staircase list
```

Bootstrap result:

```text
Configured Staircase workspace:
  workspace: repo
  project: tools/example
  review: Gerrit
  verification: Gerrit

feature  3 steps  clean  (implicit)
```

The `repo` and Gerrit providers remain distinct bindings.

---

### 15.3 Workspace discovered, review ambiguous

```text
Configured Staircase workspace:
  workspace: custom-workspace

Review provider was not configured:
  two applicable review endpoints were detected

Local staircase operations remain available.
```

---

### 15.4 Provider explicitly disabled

```console
git staircase --workspace-mode=single-git list
```

The current Git repository is treated as an independent workspace for this invocation.

---

## 16. Normative Invariants

An implementation conforming to this addendum must preserve the following invariants.

### 16.1 The first command may initialize the workspace

No explicit initialization command is required when discovery is unambiguous and safe.

### 16.2 The original command continues

Bootstrap is a prelude to the requested command, not a replacement for it.

### 16.3 Capability bindings are typed

There is no single undifferentiated `provider` setting.

### 16.4 The core always has a standalone fallback

A simple Git repository remains usable without external providers.

### 16.5 Specialized workspace evidence outranks the fallback

The built-in single-Git candidate does not create false ambiguity with a provider that proves membership in a larger workspace.

### 16.6 Automatic bootstrap is local and passive

It performs no network or workspace mutation beyond recording Staircase’s own local configuration.

### 16.7 Ambiguity narrows functionality rather than poisoning everything

Provider-independent commands continue whenever possible.

### 16.8 Explicit configuration wins

Automatic revalidation cannot silently replace explicit capability bindings.

### 16.9 Integration context is independent of review destination

A workspace provider may supply one, both, or neither, but they remain separate typed values.

### 16.10 Detached `HEAD` is a normal input

The absence of an attached local branch is not itself an error.

### 16.11 Configuration records provenance

Users and scripts can determine whether each binding was explicit, inherited, or automatically discovered.

### 16.12 Automatic configuration is reversible

The user may inspect, override, refresh, or forget any automatically created workspace record.

---

## 17. Summary

The bootstrap model is:

```text
discover installed trusted providers
        ↓
probe workspace candidates
        ↓
select specialized workspace or single-Git fallback
        ↓
probe capability providers using typed hints
        ↓
bind unambiguous capabilities
        ↓
persist local workspace configuration
        ↓
continue the original command
```

The governing rule is:

> Discover applicability automatically, persist only unambiguous safe bindings, and leave every consequential capability independently replaceable.
