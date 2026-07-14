# Git Staircase Implementation Decisions

## 1. Status and scope

This document fixes implementation choices intentionally left open by the Git Staircase specification and its `repo`, GitHub, and Gerrit provider addenda. It does not weaken their requirements. Where this document is more specific, the implementation MUST use this document.

The first conforming implementation is a clean break from pre-specification prototypes. Its storage generation, provider protocol, machine schemas, and canonical encodings are version `1`. A prototype `git-staircase-descriptor 1` blob, an unversioned JSON object, or a prototype workspace record is not generation-1 data. It MUST fail with `unsupported-schema`, MUST NOT be interpreted by field resemblance, and MUST NOT be rewritten automatically.

Every schema carries both a stable schema name and an integer version. Readers accept only explicitly supported `(schema, version)` pairs. A new meaning for an existing field, a new required field, or a canonicalization change requires a new version. Optional fields may be added only where the schema explicitly declares them extensible. Unknown namespaced extension values are preserved as exact JSON values and canonically reserialized without semantic change, but do not become authoritative without a registered schema.

## 2. Canonical octet encoding and digests

All Staircase digests that are not Git object IDs use SHA-256. Machine output and persistent state use all 64 lowercase hexadecimal digits. Human output may abbreviate only to a unique prefix in the displayed scope.

The version-1 canonical structural encoding is a small positional encoding:

```text
frame     = bytes("git-staircase") bytes(domain) u64(version) value
value     = 00                                      # absent
          | 01                                      # false
          | 02                                      # true
          | 03 u64                                  # unsigned integer
          | 04 u64 octet*                           # byte string
          | 05 u64 value*                           # ordered sequence
```

`u64` is unsigned big-endian. Each `bytes` value is tag `04`, an eight-byte length, and exactly that many octets. Text is UTF-8 encoded as `bytes`; filesystem paths use the path encoding defined below. Maps are not encoded directly: each domain defines a fixed field order or a lexically sorted sequence of `(key, value)` pairs. OIDs are encoded as `(object-format, raw-oid-bytes)`, never as abbreviated hexadecimal. No locale, map iteration order, traversal order, timestamp, display name, alias, or provenance enters an identity unless its domain explicitly says so.

The SHA-256 input is the complete frame. Version-1 domains and values are:

```text
repository
    (repository-kind, platform-path-kind, canonical-primary-object-directory)

integration-context
    (integration-set-kind, lexically sorted irredundant typed anchor OIDs)

implicit-structure
    (repository identity, object format, integration-context identity,
     ordered active typed cut OIDs)

implicit-family
    (repository identity, object format, integration-context identity,
     lexically sorted paths, each path containing ordered active typed cut OIDs)

body
    (repository identity, object format, integration-context identity,
     typed aggregate-top OID)

decomposition
    (body identity, ordered typed cut OIDs)

outcome
    (repository identity, object format, integration-context identity,
     typed aggregate-top tree OID)

fingerprint
    (schema name, schema version, lexically sorted typed input pairs)
```

Symbolic integration locators are retained as provenance but do not enter `integration-context`; locators resolving to the same canonical integration set therefore collapse. Version 1 does not expose `patch-series` identity because no patch normalization in the specifications is complete enough to make it portable. A later implementation may add it only with a new domain and explicit treatment of whitespace, renames, binary content, modes, submodules, and conflict resolutions.

External forms are:

```text
repository:sha256:<64-hex>
integration-context:sha256:<64-hex>
implicit@<64-hex>
family@<64-hex>
body:sha256:<64-hex>
decomposition:sha256:<64-hex>
outcome:sha256:<64-hex>
fingerprint:sha256:<64-hex>
```

Git object IDs remain full typed OIDs in the repository's object format. A SHA-256 Staircase digest never masquerades as a Git OID.

## 3. Repository, workspace, and provider identity

The core repository identity names the local primary object database, not a checkout path, branch, remote, hosted repository, or provider project. The core obtains the primary object directory through Git, resolves it physically without following alternates as the primary store, and hashes:

```text
repository-kind       = local-object-database
platform-path-kind    = unix-bytes | windows-utf16
canonical object directory
```

On Unix, the canonical path is the absolute `realpath` byte sequence. On Windows, it is the absolute normalized UTF-16 code-unit sequence after resolving junctions, encoded as big-endian code units, with drive and separator normalization performed by the operating-system path API. Linked worktrees sharing one primary object directory share repository identity. Separate clones, bare repositories, and repositories with distinct primary object directories remain distinct even when their objects have equal OIDs. Alternates are fingerprint inputs and cache-invalidation evidence, but do not merge repository identities. Moving an object directory changes this structural repository identity; managed lineage IDs and provider review identities remain unchanged and require re-resolution.

A workspace ID is a user-local lowercase UUIDv4. It is not derived from a root path. The workspace record stores its canonical root or provider-native locator separately and may update that locator only when continuity is proven under the applicable provider rules.

Provider-native identities remain structured tuples, not hashes of display URLs:

* `repo` project identity is the effective manifest project name; local checkout identity additionally includes workspace ID, canonical project-relative path, and Git common-directory identity.
* GitHub repository identity is canonical installation identity plus immutable repository node/database identity after validation; owner/name is retained as a locator. Before validation, the provisional tuple is normalized host plus owner/name.
* Gerrit project identity is canonical server identity plus exact project name. A confirmed review identity adds server-issued change number; a pending key retains server, project, destination branch, and Change-Id.

## 4. Persistent record schemas

Persistent JSON uses UTF-8, no BOM, RFC 8785 JSON Canonicalization Scheme, and one trailing LF. Floats and JSON numbers outside the exact signed 64-bit range are forbidden. OIDs, UUIDs, refs, paths, and URLs are strings validated independently of JSON parsing. Empty and absent are distinct. Arrays preserve semantic order; set-valued arrays are sorted by their canonical encoded value before publication.

Generation 1 has no persistent text descriptor. The structure, metadata, lifecycle, archive manifest, workspace, cache, and journal schemas below are JSON; line-oriented text is reserved for porcelain output. This makes a prototype `git-staircase-descriptor 1` unambiguously unsupported.

### 4.1 Structure

The `structure` blob is canonical JSON:

```json
{
  "schema": "git-staircase/structure",
  "version": 1,
  "kind": "linear",
  "object_format": "sha1",
  "lineage_id": "<uuid>",
  "integration_context": {
    "kind": "single-anchor",
    "anchors": [{"algorithm": "sha1", "hex": "<full-oid>"}],
    "symbolic_targets": ["refs/remotes/origin/main"]
  },
  "steps": [{
    "id": "<uuid>",
    "cut_oid": {"algorithm": "sha1", "hex": "<full-oid>"},
    "materializing_refs": ["refs/heads/feature"],
    "owned_refs": ["refs/heads/feature"]
  }],
  "structural_state": {"kind": "clean"},
  "layout": {"kind": "none"},
  "policies": {},
  "discovery_overrides": [],
  "extensions": {},
  "parent_structure_revision_oid": null
}
```

`kind` is `linear` or `family`. The canonical name is excluded. `parent_structure_revision_oid` is a typed `{algorithm, hex}` Git OID object when present. Provider mappings that affect interpretation, publication, or landing are versioned objects beneath `extensions.<provider>`.

### 4.2 Managed families

A family uses one lineage ID as its family ID and one record-level CAS unit. Each unique conceptual step occurs once:

```json
{
  "kind": "family",
  "steps": [{
    "id": "<uuid>",
    "cut_oid": {"algorithm": "sha1", "hex": "<full-oid>"},
    "parent_step_id": null,
    "materializing_refs": [],
    "owned_refs": []
  }],
  "paths": [{
    "id": "<uuid>",
    "name": "platform-ui",
    "step_ids": ["<shared-step-id>", "<ui-step-id>"]
  }]
}
```

The `steps` array is sorted by step ID; `paths` is sorted by path ID. Every path's `step_ids` is dependency order. Version 1 permits a rooted prefix tree only: paths may share an exact prefix, but reconvergence and a step with several parents are rejected. A shared step has one stable step ID, one cut, and one ownership set. Path names are selectors, not lineage. A family mutation locks and compares the whole family record, and active/archive step refs exist once per unique step.

### 4.3 Metadata, lifecycle, and archive manifest

The other record blobs use:

```text
git-staircase/metadata version 1
    title, description, labels, links, per_step keyed by step ID,
    extensions, creation_provenance, modification_provenance

git-staircase/lifecycle version 1
    state, ordered events, archive_reason, reserved_name, retention,
    restoration, provenance

git-staircase/archive-manifest version 1
    lineage_id, archived_record_predecessor, canonical_name,
    owned_refs with exact archived and restoration OIDs,
    worktree dispositions, draft snapshots, provider disposition
```

Each is one canonical JSON object with `schema` and `version`. Labels are a sorted unique array. Links are sorted by canonical URI and relation. `per_step` is a JSON object whose keys are full step UUIDs. Lifecycle events are append-only within a lifecycle revision and contain stable event kind, exact affected identities, provenance, and a timestamp only when the event's meaning requires it. Active lifecycle fields that do not apply are explicit `null`, not omitted.

The record revision is exactly this Git tree:

```text
100644 blob <oid> structure
100644 blob <oid> metadata
100644 blob <oid> lifecycle
100644 blob <oid> archive-manifest    # archived only
```

No other tree entry is accepted in version 1. Readers type-check the tree and every blob before use. Read failure is nonmutating.

### 4.4 User-local records, caches, and journals

Workspace records use `git-staircase/workspace` version 1 and contain workspace ID, kind, locator, participating repository identities and paths, capability bindings, binding provenance, discovery fingerprint, and last successful validation.

Provider observations use `git-staircase/provider-cache` version 1 and contain provider name and schema version, complete typed cache key, observation kind, exact subject/revision, observed time, freshness conditions, and namespaced observation data. Cache entries are evidence only.

Operation journals use `git-staircase/operation-journal` version 1 and contain operation ID and kind, phase, repository and lineage identities, expected record revision, complete expected and planned ref maps, worktree/draft snapshots, provider request IDs and per-item outcomes, recovery refs, and deterministic `continue`, `abort`, or `reconcile` disposition.

## 5. Storage locations

The implementation follows XDG locations, with environment overrides used only for tests or explicit administration:

```text
configuration
    ${GIT_STAIRCASE_CONFIG_DIR}
    ${XDG_CONFIG_HOME:-$HOME/.config}/git-staircase/

user-local durable state
    ${GIT_STAIRCASE_STATE_DIR}
    ${XDG_STATE_HOME:-$HOME/.local/state}/git-staircase/

cache
    ${GIT_STAIRCASE_CACHE_DIR}
    ${XDG_CACHE_HOME:-$HOME/.cache}/git-staircase/
```

Workspace records are `state/workspaces/<workspace-id>.json`. The locator index is `state/workspaces/index.json`; it is rebuildable and never authoritative identity. Provider registrations and explicit profiles are under `config/providers/` and `config/profiles/`. Provider observations are under `cache/providers/<provider>/<first-two-digest-bytes>/<cache-key>.json`. Discovery caches are under `cache/repositories/<repository-digest>/`. No fallback to a shared temporary directory is permitted; absence of a usable home/state directory disables persistence with a diagnostic.

Repository-coordinated state is beneath the Git common directory:

```text
<git-common-dir>/staircase/locks/
<git-common-dir>/staircase/journals/
<git-common-dir>/staircase/tmp/
```

It is never placed in a linked worktree's per-worktree Git directory. Temporary files use mode `0600`, are created in the destination filesystem, and are atomically renamed. Secrets are forbidden in every location above.

## 6. Locks, journals, and recovery refs

Repository mutation takes `locks/repository.lock`. A mutation of an existing lineage also takes `locks/lineage-<uuid>.lock`; provider operations additionally take `locks/provider-<provider>-<route-digest>.lock`. Locks are acquired in that order. Lock files contain schema version, process identity, start time, operation ID, and a random nonce. A process may break a stale lock only after proving the owner is gone and preserving the old lock as diagnostic evidence. Locks do not replace expected-old-value leases.

The durable journal is `journals/<operation-id>.json`. It is published and fsynced before the first non-transactional effect and after every phase transition. Git objects and these recovery refs protect pre-operation and planned state:

```text
refs/staircase-recovery/<operation-id>/record
refs/staircase-recovery/<operation-id>/steps/<step-id>
refs/staircase-recovery/<operation-id>/planned/steps/<step-id>
refs/staircase-recovery/<operation-id>/drafts/<worktree-id>
refs/staircase-recovery/<operation-id>/provider-plan
```

Each ref points to the exact Git object described by the journal. Absent categories have no ref. Ref creation and deletion use Git ref transactions. Successful completion publishes authoritative refs first, records completion in the journal, removes recovery refs transactionally, then removes the journal. Abort restores only values proven by the journal's leases. An uncertain remote outcome retains the journal, provider plan, and required recovery refs until `reconcile` proves a result. Startup and every mutating command scan only the current repository's journals and report `operation-in-progress` rather than guessing.

## 7. Policy and schema registry

One in-process registry validates persistent policy, provider extensions, provider messages, and machine output. A registry entry contains:

```text
schema name and version
owning component
namespace
JSON shape and limits
semantic validator
storage class: structure | metadata | lifecycle | cache | journal
capabilities and provider operations that may use it
```

Core keys use the namespaces `discovery.`, `verification.`, `review.`, `landing.`, `merge.`, `retention.`, and `layout.`. Provider-only keys use the exact provider namespace, including `github.` and `gerrit.`. Unnamespaced keys are rejected.

Built-in core, `repo`, GitHub, and Gerrit schemas are compiled into the same release as the corresponding implementation. An external provider may register schemas only through its trusted, versioned descriptor; the descriptor supplies schema names, versions, SHA-256 definition digests, and size limits. Registration does not grant capabilities.

Readers preserve unknown namespaced extension JSON values. They report the containing policy or extension as `schema-unavailable`, and no operation may reinterpret or mutate that value. An operation that does not semantically touch it may copy it unchanged. `policy set` and `policy unset` validate the complete resulting policy set and therefore fail if any affected schema is unavailable. Cache-only schemas never authorize structure changes.

## 8. Provider process and network boundaries

Built-in providers use the same logical protocol as subprocess providers. A subprocess is selected only from a trusted registration or provider directory. The core executes its absolute path directly, never through a shell or ordinary `PATH`, with a fixed operation argument. Standard input and output each contain exactly one bounded JSON document:

```json
{
  "schema": "git-staircase/provider-request",
  "version": 1,
  "request_id": "<uuid>",
  "operation": "probe-workspace",
  "provider": "repo",
  "network": "forbidden",
  "mutation": "forbidden",
  "context": {},
  "input": {}
}
```

The response is `git-staircase/provider-response` version 1 with the same request ID, `ok`, one typed `result` or typed `error`, and bounded diagnostics. Provider domain failures are `ok: false`; nonzero process exit, signal, timeout, malformed output, or excess output is a provider execution failure. Standard error is bounded diagnostic text and never machine data.

Passive `describe`, `probe-workspace`, `probe-capability`, local route composition, and local revalidation always use `network: forbidden` and `mutation: forbidden`. The core supplies canonical paths, Git facts, and already collected hints; the provider MUST NOT execute repository hooks or repository-supplied programs. A passive provider may run only its documented fixed local tools, with fixed argument construction and no shell fragments.

Only an explicit network-requiring review, verification refresh, transport, landing, authentication test, or provider refresh operation uses `network: allowed`. The mutation field independently permits `remote-allowed` only for a command whose plan authorizes remote mutation. Planning and doctor operations remain `mutation: forbidden`. The core never sends credential contents. Providers use explicitly configured authentication mechanisms and redact secrets from all results. A subprocess provider is trusted code; the flags are a protocol obligation and audit boundary, not a sandbox claim.

Default bounds are 10 seconds and 1 MiB for passive operations, 60 seconds and 8 MiB for local planning, and operation-specific explicit bounds for network or mutation calls. A provider may request pagination through subsequent core-issued requests; it may not evade output bounds.

The `repo` provider never runs `repo init`, `repo sync`, `repo start`, `repo checkout`, `repo upload`, `repo abandon`, or `repo prune`. GitHub and Gerrit passive probes never invoke network-capable CLI queries. External Git and `repo sync` sequencer state remains externally owned.

## 9. Provider readiness and synchronization vocabulary

Readiness is reported independently for each capability and complete route. Machine values are exactly:

```text
not-applicable
applicable-unbound
bound-incomplete
bound-ready
bound-authentication-required
bound-stale
```

Human output may render `bound-ready` as `ready` only when the binding and route are shown separately. Provider installation capability is not readiness. `repo` commonly reports readiness for workspace, project-mapping, integration-context, and workspace-hints; GitHub and Gerrit report it separately for each capability they implement.

Review association synchronization uses exactly:

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

`current` always proves equality of the exact local planned revision, provider review revision, and required transport ref revision. `closed`, `merged`, and `abandoned` are historical synchronization states, not provider-native review status replacements. GitHub draft/review/check/queue fields and Gerrit change status/labels/checks/submit requirements remain separately typed. Landing uncertainty is operation state `landing-unknown`, not review synchronization. Workspace freshness uses `current`, `stale`, `unavailable`, or `ambiguous`; it is never called review synchronization.

## 10. Porcelain, JSON, diagnostics, and errors

Successful JSON output retains each command's specified top-level type. In particular, `list --json` is always an array. JSON uses full IDs and schema-stable field names. Commands that return an object include `"schema"` and `"version"`; array elements include `"schema_version": 1` when their element schema is not fixed by the command.

Version-1 porcelain is one LF-terminated record per line. Fields are TAB-separated. The first two fields are record type and schema version. Every remaining string field is a canonical JSON string token, so raw tabs, newlines, NULs, and control characters never occur. Integer, boolean, and null fields use JSON lexical form. A command's record types and field order are schema, not display formatting. Empty porcelain list output is zero bytes.

Machine-readable standard output is reserved for successful command data, including data emitted before a strict diagnostic failure. Structured errors and diagnostics go to standard error. With `--json`, each error is one compact JSON line:

```json
{"schema":"git-staircase/error","version":1,"code":"selector-ambiguous","message":"<safe human summary>","exit_status":2,"details":{},"provider":null,"remedies":[]}
```

`details` contains typed command-specific facts and full identifiers. `provider`, when present, contains `name`, namespaced `code`, and typed redacted details beneath the stable core code. `remedies` is an array of complete argument vectors, not shell strings.

With `--porcelain`, the same error is:

```text
error<TAB>1<TAB>"selector-ambiguous"<TAB>2<TAB>"safe human summary"<TAB>{}<TAB>null<TAB>[]
```

Human mode prints Git-style prose to standard error and need not print the envelope. One command emits one primary error envelope; additional bounded diagnostics use schema `git-staircase/diagnostic` version 1 and reference the primary code. Bootstrap announcements and progress are suppressed in both machine modes.

Exit status follows the core classes. The stable error code is authoritative. A successful no-op includes `"transition":"none"` in JSON or a `result 1 "no-op" ...` porcelain record.

## 11. Adoption boundaries

Core adoption follows reconstructibility, not command spelling. Observation, exact revision-derived identities, and a clean final ref-materialized reshape remain implicit. Stable lineage or step identity, metadata-only boundaries, stale intended structure, persistent policy or metadata, archive state, durable recovery, provider association, or partial-landing continuity require management. `--no-adopt` fails before every local or remote mutation.

Provider differences are:

* `repo` never adopts a staircase. Bootstrap and refresh persist only user-local workspace binding and evidence. A changed manifest, project mapping, or checkout anchor marks facts or bindings stale; it does not create lineage or alter cuts.
* Gerrit planning, cached or exact status, verification queries, and explicitly ephemeral upload may operate on an implicit staircase. Preparing pending review keys, creating, attaching, uploading with continuity, preserving confirmed identities, provider policy, or landing continuity requires existing management or automatic adoption.
* GitHub route discovery, pull-request discovery, planning, and exact-revision comparison may remain implicit. Durable pull-request associations, stable remote-head mappings, stacked or cumulative topology, provider policy, stepwise landing continuity, and relinquished recovery state require management. Ephemeral publication is permitted only for one aggregate pull request with no retained continuity.

Adoption and the triggering operation share one journal and one logical result. Machine success includes an ordered event:

```json
{"kind":"adoption","reason":"<stable-code>","lineage_id":"<uuid>","record_revision_oid":"<full-oid>"}
```

Failure before any durable effect removes provisional adoption. Uncertain remote effect retains the managed record and recovery state needed for identity-based reconciliation.
