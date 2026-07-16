# Addendum G: Monorepo Worktree Management

## 1. Status and scope

This document defines the specification for **Git Monorepo Worktree**, a command-line interface and provider contract for creating isolated, lightweight worktree images of a multi-repository or single-repository monorepo.

It defines:
* The command structure: `git monorepo worktree`.
* The lifecycle of monorepo worktrees (create, list, remove, prune).
* The construction of "shadow workspaces" for multi-repository environments.
* The interaction with the core Workspace Provider.
* Integration requirements to support agentic workflows (e.g., `repox`).

This specification extends the **Git Staircase Specification** and composes with **Addendum E: `repo` Workspace Provider**.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

---

## 2. Motivation

In large monorepos (whether single Git repositories or multi-repository layouts managed by tools like `repo`), executing automated tasks, refactorings, or code reviews (such as `repox posthoc` or `repox stack codereview`) requires an isolated environment.

Creating a full copy of a monorepo is prohibitively expensive in terms of time and disk space.
Using standard `git worktree` only works for a single Git repository and does not capture the multi-repository context of tools like `repo`.

An agent needs a workspace that:
1. **Is Isolated**: Changes made by the agent must not pollute the user's primary workspace.
2. **Is Complete**: The agent must be able to build, test, and run language servers across the entire monorepo.
3. **Is Lightweight**: Creation and deletion must be fast (seconds, not minutes) and consume minimal additional disk space.
4. **Preserves Git State**: The active repository must be a true Git worktree, allowing normal Git operations (commit, branch, diff).

---

## 3. Goals and non-goals

### 3.1 Goals

The implementation MUST:
1. Support both single Git repository monorepos and multi-repository monorepos (e.g., `repo` workspaces).
2. Create a worktree image that is sufficient for running builds (e.g., Bazel), tests, and language servers.
3. Share Git object databases to minimize disk space and avoid network fetches.
4. Provide a command-line interface that integrates cleanly with Git.
5. Define a robust lifecycle for managing these temporary workspaces.
6. Ensure that the primary workspace remains completely unaffected by actions in the shadow workspace.
7. Support multiple active repositories within the same shadow workspace if needed.

### 3.2 Non-goals

This specification does not:
* Replace the `repo` tool for general workspace management.
* Support syncing or updating the shadow workspace from remotes (it is intended to be short-lived).
* Guarantee virtual filesystem performance; it relies on standard OS symlinks and Git worktrees.

---

## 4. Conceptual Model

### 4.1 Monorepo Root (Workspace Root)

The **Monorepo Root** is the top-level directory containing the entire project structure.
* In a single Git repository, it is the Git root.
* In a `repo` workspace, it is the directory containing the `.repo` directory.
* In a Bazel project, it is the directory containing the root `WORKSPACE` or `MODULE.bazel` file.

### 4.2 Active Repository

The **Active Repository** is the specific Git repository within the monorepo where the user (or agent) intends to make changes.
* In a single Git repository monorepo, the active repository is the monorepo itself.
* In a multi-repository monorepo, it is a sub-directory that is a Git repository.

### 4.3 Shadow Workspace

A **Shadow Workspace** is a directory structure that mirrors the Monorepo Root.
It is constructed such that:
* The Active Repository is materialized as a standard **Git Worktree** linked to the primary repository.
* All other directories and files are **symbolically linked** to their counterparts in the primary workspace.
* Metadata directories (like `.git`, `.repo`) and build output directories are excluded or handled specially.

### 4.4 Monorepo Worktree Registry

To allow tools to identify shadow workspaces and resolve them back to their primary counterparts, the system MUST maintain a user-local registry.
This registry maps shadow workspace paths to their metadata.

---

## 5. Command Porcelain

The command namespace is `git monorepo worktree`.

### 5.1 `git monorepo worktree create`

Creates a new monorepo worktree.

```console
git monorepo worktree create [<options>] [<path>]
```

#### 5.1.1 Options

* `-r, --repo <path>`: The path to the active repository. Defaults to the Git repository containing the current working directory. Multiple `--repo` flags MAY be specified to make multiple repositories active (non-symlinked) in the created worktree.
* `-b, --branch <branch>`: Create/checkout the specified branch in the active repository.
* `-c, --commit <commit-ish>`: Checkout the specified commit (detached HEAD) in the active repository.
* `--upstream <branch>`: Set the upstream branch for the created branch.
* `--base <branch>`: The base branch to branch off from (defaults to current branch or upstream).
* `--name <name>`: A human-readable name/prefix for the worktree directory.

#### 5.1.2 Output

Upon success, the command MUST print the absolute path to the created workspace to `stdout` as the last line of output.
No other information should be printed to `stdout` unless requested by a verbose flag. Diagnostics and progress MUST go to `stderr`.

```console
$ git monorepo worktree create --branch dry-feature-1
/usr/local/google/home/user/.shared/git-monorepo/worktrees/shadow-a1b2c3d4
```

### 5.2 `git monorepo worktree list`

Lists all active monorepo worktrees managed by this tool.

```console
git monorepo worktree list
```

The output MUST be machine-readable (e.g., JSON or tab-separated values) and include:
* Worktree ID
* Path to the shadow workspace
* Path to the primary monorepo root
* Active repository paths (relative to monorepo root)
* Active branches/commits
* Creation timestamp

### 5.3 `git monorepo worktree remove`

Removes a specific monorepo worktree and cleans up associated Git worktrees.

```console
git monorepo worktree remove <id-or-path> [--force]
```

* It MUST remove the shadow workspace directory.
* It MUST run `git worktree remove` on all active repositories' worktrees associated with this shadow workspace.
* It MUST run `git worktree prune`.
* It MUST remove the entry from the registry.

### 5.4 `git monorepo worktree prune`

Cleans up stale or orphaned worktrees.

```console
git monorepo worktree prune [--force] [--age <duration>]
```

* It MUST identify and remove worktrees whose primary workspace no longer exists.
* It SHOULD remove worktrees older than the specified duration (default: 7 days) if `--force` is specified.

---

## 6. Construction of Shadow Workspace

When a shadow workspace is required (i.e., in a multi-repository monorepo where the active repository is a subset of the workspace):

### 6.1 Directory Mirroring

The tool MUST recreate the directory structure of the Monorepo Root in the target directory, with the following rules:

1. **Active Repo Paths**: The directories corresponding to the active repositories MUST be symlinks pointing to the newly created Git worktrees of those repositories.
2. **Exclusions**: The following directories and files MUST NOT be symlinked or copied to the shadow workspace:
   * `.git` (in any repository, except as managed by Git worktree)
   * `.repo` (for `repo` workspaces)
   * `.venv`, `node_modules` (language-specific virtual environments)
   * `bazel-bin`, `bazel-out`, `bazel-testlogs`, `bazel-<workspace_name>` (Bazel build outputs)
   * Any user-configured ignore patterns.
3. **Other Files/Dirs**: All other files and directories MUST be symbolically linked to the corresponding path in the primary workspace.

### 6.2 Symlink Resolution

Symlinks created in the shadow workspace MUST point to the absolute paths in the primary workspace.
This ensures they remain valid regardless of where the shadow workspace is located.

### 6.3 Git Worktree Creation

For each active repository, the tool MUST:
1. Create a Git worktree using `git worktree add` pointing to a location inside the shadow workspace (or a dedicated worktree storage area, with a symlink in the shadow workspace).
2. Apply the requested `--branch` or `--commit` configuration.
3. Configure the worktree to ignore common scratch directories/files (e.g., via `.git/info/exclude` in the worktree).

---

## 7. Build System & Tool Integration

To ensure that builds and tests can be run in the shadow workspace without disturbing the primary workspace, the tool MUST apply the following configurations:

### 7.1 Bazel Integration

If the monorepo uses Bazel (detected by `WORKSPACE` or `MODULE.bazel` at the root):

1. **Output Base Isolation**: The tool MUST ensure that Bazel runs in the shadow workspace use a distinct output base to avoid lock collisions with the primary workspace.
2. **Configuration Override**: The tool SHOULD create a `.bazelrc` file in the root of the shadow workspace that:
   * Imports the primary workspace's `.bazelrc` (if it exists) to preserve project-specific flags.
   * Sets `--output_base` to a path unique to this shadow workspace (e.g., under `~/.cache/bazel/_bazel_user/shadow-<id>`).
   * Configures shared repository cache (e.g., `--repository_cache=~/.cache/bazel-repo-cache`) to avoid re-downloading dependencies.

Example generated `.bazelrc` in shadow root:
```text
# Import primary workspace settings
try-import /path/to/primary/workspace/.bazelrc

# Isolate output base
startup --output_base=/usr/local/google/home/user/.cache/bazel/_bazel_user/shadow-a1b2c3d4

# Share repository cache
common --repository_cache=/usr/local/google/home/user/.cache/bazel-repo-cache
```

### 7.2 Language Server (LSP) Integration

Language servers running in the shadow workspace MUST see the resolved symlinks.
Most LSPs follow symlinks automatically. The registry (Section 8) can be used by editor plugins to map diagnostics back to the primary workspace if necessary.

---

## 8. Registry Specification

The registry MUST be stored in the user's home directory:
`~/.config/git-monorepo/worktrees.json`

### 8.1 Schema

```json
{
  "worktrees": [
    {
      "id": "shadow-a1b2c3d4",
      "path": "/usr/local/google/home/user/.shared/git-monorepo/worktrees/shadow-a1b2c3d4",
      "primary_root": "/usr/local/google/home/user/projects/monorepo",
      "active_repos": [
        {
          "relative_path": "tools/vendor/google",
          "worktree_path": "/usr/local/google/home/user/projects/monorepo/tools/vendor/google/.git/worktrees/dry-worktree-temp-12345"
        }
      ],
      "created_at": "2026-07-15T20:08:42Z"
    }
  ]
}
```

---

## 9. Workspace Provider Extensions

This section defines how existing providers (specifically `repo`) MUST extend their behavior to support this specification.

### 9.1 `repo` Provider Extensions

The `repo` provider (Addendum E) MUST support querying project paths.

#### 9.1.1 Project Discovery API

The provider MUST expose a way to list all manifest projects and their paths.
This can be via a new command or an internal API used by the `git monorepo worktree` tool.

If using CLI, the provider SHOULD support:
```console
git staircase provider repo list-projects
```
Output format MUST be a list of relative paths from the workspace root.

```text
common
tools/other-tool
tools/vendor/other-vendor
tools/vendor/google
```

This list is used by `git monorepo worktree` to determine which directories in the primary workspace are individual Git repositories, allowing it to correctly decide whether to symlink them or create worktrees.

#### 9.1.2 Workspace Identification from Shadow

When `Workspace.find()` is run inside a shadow workspace:
1. It MUST check the registry (`~/.config/git-monorepo/worktrees.json`) to see if the current directory is within an active shadow workspace.
2. If it is, `Workspace.find()` MUST return a workspace object representing the *primary* workspace, but with capability bindings aware of the shadow redirection.
3. This allows tools like `repox` to resolve the true repo root and manifest even when executing inside the shadow workspace.
