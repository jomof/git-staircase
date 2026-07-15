use crate::core::refs::StaircaseRefs;
use crate::error::{Result, StaircaseError};
use crate::memoization::{MemoKey, Memoizable, Memoizer};
use crate::model::BranchInfo;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub workdir: PathBuf,
    pub memoizer: Memoizer,
}

pub struct GitCommand<'a> {
    repo: &'a GitRepo,
    args: Vec<String>,
    stdin: Option<String>,
    interactive: bool,
    check_status: bool,
    trim: bool,
    envs: std::collections::HashMap<String, String>,
}

impl<'a> GitCommand<'a> {
    pub fn new(repo: &'a GitRepo) -> Self {
        Self {
            repo,
            args: Vec::new(),
            stdin: None,
            interactive: false,
            check_status: true,
            trim: true,
            envs: std::collections::HashMap::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<S: AsRef<str>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        for arg in args {
            self.args.push(arg.as_ref().to_string());
        }
        self
    }

    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn check_status(mut self, check: bool) -> Self {
        self.check_status = check;
        self
    }

    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }
    pub fn run(self) -> Result<String> {
        let trim = self.trim;
        let output = self.run_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if trim {
            Ok(stdout.trim().to_string())
        } else {
            Ok(stdout.into_owned())
        }
    }

    pub fn run_output(self) -> Result<std::process::Output> {
        let mut cmd = self.repo.git_cmd();
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        cmd.args(&self.args);

        let output = if self.interactive {
            let status = cmd.status()?;
            std::process::Output {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        } else if let Some(input) = self.stdin {
            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut child_stdin = child.stdin.take().ok_or_else(|| {
                StaircaseError::Other("Failed to open stdin for git command".into())
            })?;

            thread::scope(|s| {
                s.spawn(move || {
                    let _ = child_stdin.write_all(input.as_bytes());
                });
                child.wait_with_output()
            })?
        } else {
            cmd.output()?
        };

        if self.check_status && !output.status.success() {
            return Err(StaircaseError::GitCommandFailed {
                command: format!("git {}", self.args.join(" ")),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(output)
    }
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: String,
    pub oid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: Option<String>,
    pub branch: Option<String>,
}

impl TreeEntry {
    pub fn blob(oid: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            mode: "100644".to_string(),
            kind: "blob".to_string(),
            oid: oid.into(),
            name: name.into(),
        }
    }
}

impl GitRepo {
    pub fn new(workdir: PathBuf) -> Self {
        GitRepo {
            workdir,
            memoizer: Memoizer::new(),
        }
    }

    pub fn with_memoizer(workdir: PathBuf, memoizer: Memoizer) -> Self {
        GitRepo { workdir, memoizer }
    }

    pub fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.workdir);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd.env("GIT_OPTIONAL_LOCKS", "0");
        cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
        cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
        cmd
    }

    pub fn memoize<T, F>(&self, key: Option<MemoKey>, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
        T: Memoizable,
    {
        if let Some(ref k) = key {
            if let Some(val) = self.memoizer.get(k) {
                if let Some(res) = T::from_value(val) {
                    return Ok(res);
                }
            }
        }
        let res = f()?;
        if let Some(k) = key {
            self.memoizer.put(k, res.to_value());
        }
        Ok(res)
    }

    pub fn memoize_rev<T, F>(
        &self,
        rev: &str,
        key_gen: impl FnOnce(&str) -> MemoKey,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
        T: Memoizable,
    {
        let key = if rev != "HEAD" {
            Some(key_gen(rev))
        } else {
            None
        };
        self.memoize(key, f)
    }

    pub fn command(&self) -> GitCommand<'_> {
        GitCommand::new(self)
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let trim = !args.first().map_or(false, |&cmd| cmd == "cat-file");
        self.command().args(args).trim(trim).run()
    }

    pub fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String> {
        self.command().args(args).stdin(stdin).run()
    }

    pub fn run_interactive(&self, args: &[&str]) -> Result<()> {
        self.command().args(args).interactive(true).run_output()?;
        Ok(())
    }

    pub fn resolve_ref(&self, rev: &str) -> Result<String> {
        self.resolve_ref_opt(rev)?
            .ok_or_else(|| StaircaseError::Other(format!("Could not resolve ref: {}", rev)))
    }

    pub fn resolve_commit(&self, rev: &str) -> Result<String> {
        self.memoize_rev(
            rev,
            |r| MemoKey::ResolveCommit { rev: r.to_string() },
            || {
                let oid = if !rev.starts_with("refs/") {
                    if let Ok(sha) = self
                        .command()
                        .args(&[
                            "rev-parse",
                            "--verify",
                            &format!("refs/tags/{}^{{commit}}", rev),
                        ])
                        .run()
                    {
                        return Ok(sha);
                    }
                    if let Ok(sha) = self
                        .command()
                        .args(&[
                            "rev-parse",
                            "--verify",
                            &format!("refs/heads/{}^{{commit}}", rev),
                        ])
                        .run()
                    {
                        return Ok(sha);
                    }
                    self.command()
                        .args(&["rev-parse", "--verify", &format!("{}^{{commit}}", rev)])
                        .run()?
                } else {
                    self.command()
                        .args(&["rev-parse", "--verify", &format!("{}^{{commit}}", rev)])
                        .run()?
                };
                Ok(oid)
            },
        )
    }

    pub fn resolve_symbolic_full_name(&self, name: &str) -> Result<String> {
        self.memoize_rev(
            name,
            |n| MemoKey::ResolveSymbolic {
                name: n.to_string(),
            },
            || {
                let full_name = self
                    .command()
                    .args(&["rev-parse", "--symbolic-full-name", name])
                    .run()?;
                if full_name.is_empty() {
                    return Err(StaircaseError::Other(format!(
                        "Could not resolve \"{}\" to a full refname",
                        name
                    )));
                }
                Ok(full_name)
            },
        )
    }

    pub fn resolve_commit_opt(&self, rev: &str) -> Result<Option<String>> {
        match self.resolve_commit(rev) {
            Ok(sha) => Ok(Some(sha)),
            Err(_) => Ok(None),
        }
    }

    pub fn resolve_ref_opt(&self, rev: &str) -> Result<Option<String>> {
        self.memoize_rev(
            rev,
            |r| MemoKey::ResolveRef { rev: r.to_string() },
            || {
                let result = self
                    .command()
                    .args(&["rev-parse", "--verify", rev])
                    .check_status(false)
                    .run_output()?;

                if result.status.success() {
                    Ok(Some(
                        String::from_utf8_lossy(&result.stdout).trim().to_string(),
                    ))
                } else {
                    Ok(None)
                }
            },
        )
    }

    pub fn preload_ancestry(&self, oids: &[&str]) -> Result<()> {
        self.preload_ancestry_ext(oids, &[])
    }

    pub fn preload_ancestry_ext(&self, oids: &[&str], exclude_oids: &[&str]) -> Result<()> {
        use std::collections::{HashMap, HashSet, VecDeque};
        let mut resolved_oids = Vec::new();
        for &oid in oids {
            if oid.is_empty() {
                continue;
            }
            if let Ok(resolved) = self.resolve_commit(oid) {
                resolved_oids.push(resolved);
            }
        }

        let unique_oids: Vec<String> = resolved_oids
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if unique_oids.is_empty() {
            return Ok(());
        }

        let mut resolved_exclude = Vec::new();
        for &ex in exclude_oids {
            if ex.is_empty() {
                continue;
            }
            if let Ok(resolved) = self.resolve_commit(ex) {
                resolved_exclude.push(format!("^{resolved}"));
            }
        }

        let cmd_args = vec!["rev-list", "--parents", "--ignore-missing"];
        let mut args_oids: Vec<String> = unique_oids.clone();
        args_oids.extend(resolved_exclude);
        let str_args: Vec<&str> = args_oids.iter().map(String::as_str).collect();

        let output = self
            .command()
            .args(&cmd_args)
            .args(&str_args)
            .check_status(false)
            .run_output()?;

        if !output.status.success() {
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut parents_map: HashMap<String, Vec<String>> = HashMap::new();

        for line in stdout.lines() {
            let mut parts = line.split_whitespace();
            if let Some(commit) = parts.next() {
                let parents: Vec<String> = parts.map(|s| s.to_string()).collect();
                parents_map.insert(commit.to_string(), parents);
            }
        }

        let mut all_target_oids = unique_oids.clone();
        for &ex in exclude_oids {
            if let Ok(resolved) = self.resolve_commit(ex) {
                all_target_oids.push(resolved);
            }
        }

        for start_oid in &unique_oids {
            let mut reachable = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start_oid.clone());
            reachable.insert(start_oid.clone());

            while let Some(curr) = queue.pop_front() {
                if let Some(parents) = parents_map.get(&curr) {
                    for p in parents {
                        if reachable.insert(p.clone()) {
                            queue.push_back(p.clone());
                        }
                    }
                }
            }

            for other_oid in &all_target_oids {
                let is_anc = reachable.contains(other_oid);
                if is_anc || exclude_oids.is_empty() {
                    self.memoizer.set_ancestry(other_oid, start_oid, is_anc);
                }
            }
        }

        Ok(())
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let anc_oid = self.resolve_commit(ancestor)?;
        let desc_oid = self.resolve_commit(descendant)?;
        if anc_oid == desc_oid {
            return Ok(true);
        }
        self.memoize(
            Some(MemoKey::Ancestry {
                ancestor: anc_oid.clone(),
                descendant: desc_oid.clone(),
            }),
            || {
                let output = self
                    .command()
                    .args(&["merge-base", "--is-ancestor", &anc_oid, &desc_oid])
                    .check_status(false)
                    .run_output()?;

                match output.status.code() {
                    Some(0) => Ok(true),
                    Some(1) => Ok(false),
                    _ => {
                        if !output.status.success() {
                            return Err(StaircaseError::GitCommandFailed {
                                command: format!(
                                    "git merge-base --is-ancestor {} {}",
                                    anc_oid, desc_oid
                                ),
                                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                            });
                        }
                        Ok(false)
                    }
                }
            },
        )
    }

    pub fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        let a_oid = self.resolve_commit(a)?;
        let b_oid = self.resolve_commit(b)?;
        self.memoize(
            Some(MemoKey::MergeBase {
                a: a_oid.clone(),
                b: b_oid.clone(),
            }),
            || self.command().args(&["merge-base", &a_oid, &b_oid]).run(),
        )
    }

    pub fn commits_between(&self, base: &str, tip: &str) -> Result<Vec<String>> {
        let stdout = self
            .command()
            .args(&["rev-list", "--reverse", &format!("{}..{}", base, tip)])
            .run()?;
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }

    pub fn update_branch(&self, branch_name: &str, oid: &str) -> Result<()> {
        let ref_name = format!("refs/heads/{}", branch_name);
        self.command().args(&["update-ref", &ref_name, oid]).run()?;
        self.memoizer.clear();
        Ok(())
    }

    pub fn update_step_ref(&self, id: &str, step_id: &str, cut: &str) -> Result<()> {
        let ref_name = StaircaseRefs::state_step(id, step_id);
        self.command().args(&["update-ref", &ref_name, cut]).run()?;
        self.memoizer.clear();
        Ok(())
    }

    pub fn delete_step_ref(&self, id: &str, step_id: &str) -> Result<()> {
        let ref_name = StaircaseRefs::state_step(id, step_id);
        self.command()
            .args(&["update-ref", "-d", &ref_name])
            .run()?;
        self.memoizer.clear();
        Ok(())
    }

    pub fn get_object_format(&self) -> Result<String> {
        self.memoize(Some(MemoKey::ObjectFormat), || {
            self.command()
                .args(&["rev-parse", "--show-object-format"])
                .run()
        })
    }

    pub fn repository_identity(&self) -> Result<String> {
        let object_directory = self
            .command()
            .args(&["rev-parse", "--git-path", "objects"])
            .run()?;
        let path = PathBuf::from(object_directory);
        let path = if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        };
        let canonical = path.canonicalize()?;
        Ok(canonical.to_string_lossy().into_owned())
    }

    pub fn git_dir(&self) -> Result<PathBuf> {
        let raw = self.command().args(["rev-parse", "--git-dir"]).run()?;
        let path = PathBuf::from(raw);
        Ok(if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        })
    }

    pub fn common_dir(&self) -> Result<PathBuf> {
        let raw = self
            .command()
            .args(["rev-parse", "--git-common-dir"])
            .run()?;
        let path = PathBuf::from(raw);
        Ok(if path.is_absolute() {
            path
        } else {
            self.workdir.join(path)
        })
    }

    pub fn get_tree_id(&self, rev: &str) -> Result<String> {
        let commit_oid = self.resolve_commit(rev)?;
        self.memoize(
            Some(MemoKey::TreeId {
                commit: commit_oid.clone(),
            }),
            || {
                self.command()
                    .args(&["rev-parse", &format!("{}^{{tree}}", commit_oid)])
                    .run()
            },
        )
    }

    pub fn write_blob(&self, content: &str) -> Result<String> {
        self.command()
            .args(&["hash-object", "-w", "--stdin"])
            .stdin(content)
            .run()
    }

    pub fn write_json<T: Serialize>(&self, data: &T) -> Result<String> {
        let json = serde_json::to_string_pretty(data)?;
        self.write_blob(&json)
    }

    pub fn write_tree(&self, entries: &[TreeEntry]) -> Result<String> {
        let mut input = String::new();
        for entry in entries {
            input.push_str(&format!(
                "{} {} {}\t{}\0",
                entry.mode, entry.kind, entry.oid, entry.name
            ));
        }
        self.command().args(&["mktree", "-z"]).stdin(input).run()
    }

    pub fn hash_data(&self, data: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let content_sha = format!("{:x}", hasher.finalize());
        self.memoize(Some(MemoKey::HashData { content_sha }), || {
            self.command()
                .args(&["hash-object", "--stdin"])
                .stdin(data)
                .run()
        })
    }

    pub fn get_patch_id(&self, base: &str, tip: &str) -> Result<String> {
        self.memoize(
            Some(MemoKey::PatchId {
                base: base.to_string(),
                tip: tip.to_string(),
            }),
            || {
                let diff = self.command().args(&["diff-tree", "-p", base, tip]).run()?;
                let stdout = self.command().args(&["patch-id"]).stdin(diff).run()?;
                let pid = stdout.split_whitespace().next().unwrap_or("").to_string();
                Ok(pid)
            },
        )
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = self
            .command()
            .args(&["symbolic-ref", "-q", "HEAD"])
            .check_status(false)
            .run_output()?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn local_branches(&self, pattern: Option<&str>) -> Result<Vec<BranchInfo>> {
        let stdout = self
            .command()
            .args(&[
                "for-each-ref",
                "--format=%(refname)%09%(objectname)%09%(upstream)",
                pattern.unwrap_or("refs/heads/"),
            ])
            .run()?;
        let mut branches = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let refname = parts[0].to_string();
                let oid = parts[1].to_string();
                let upstream = if parts.len() >= 3 && !parts[2].is_empty() {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                branches.push(BranchInfo {
                    refname,
                    oid,
                    upstream,
                });
            }
        }
        Ok(branches)
    }

    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self
            .command()
            .args(["worktree", "list", "--porcelain"])
            .run()?;
        let mut worktrees = Vec::new();
        let mut current: Option<WorktreeInfo> = None;
        for line in output.lines().chain(std::iter::once("")) {
            if let Some(path) = line.strip_prefix("worktree ") {
                if let Some(info) = current.take() {
                    worktrees.push(info);
                }
                current = Some(WorktreeInfo {
                    path: PathBuf::from(path),
                    head: None,
                    branch: None,
                });
            } else if let Some(head) = line.strip_prefix("HEAD ") {
                if let Some(info) = current.as_mut() {
                    info.head = Some(head.into());
                }
            } else if let Some(branch) = line.strip_prefix("branch ") {
                if let Some(info) = current.as_mut() {
                    info.branch = Some(branch.into());
                }
            } else if line.is_empty() {
                if let Some(info) = current.take() {
                    worktrees.push(info);
                }
            }
        }
        Ok(worktrees)
    }

    pub fn update_refs_transaction(&self, commands: &[String]) -> Result<()> {
        if commands.is_empty() {
            return Ok(());
        }
        let input = format!("start\n{}\nprepare\ncommit\n", commands.join("\n"));
        self.command()
            .args(&["update-ref", "--stdin"])
            .stdin(input)
            .run()?;
        self.memoizer.clear();
        Ok(())
    }
    pub fn rev_list(&self, args: &[&str]) -> Result<Vec<String>> {
        let stdout = self.command().arg("rev-list").args(args).run()?;
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }

    pub fn get_parents(&self, commit: &str) -> Result<Vec<String>> {
        let stdout = self
            .command()
            .args(&["show", "-s", "--format=%P", commit])
            .run()?;
        Ok(stdout.split_whitespace().map(|s| s.to_string()).collect())
    }

    pub fn push(&self, remote: &str, refspecs: &[&str], atomic: bool, dry_run: bool) -> Result<()> {
        let mut cmd = self.command().arg("push");
        if atomic {
            cmd = cmd.arg("--atomic");
        }
        if dry_run {
            cmd = cmd.arg("--dry-run");
        }
        cmd.arg(remote).args(refspecs).run()?;
        Ok(())
    }

    pub fn fetch(&self, remote: &str, refspecs: &[&str], dry_run: bool) -> Result<()> {
        let mut cmd = self.command().arg("fetch");
        if dry_run {
            cmd = cmd.arg("--dry-run");
        }
        cmd.arg(remote).args(refspecs).run()?;
        Ok(())
    }

    pub fn check_ref_format(&self, name: &str, branch: bool) -> Result<()> {
        let mut cmd = self.command().arg("check-ref-format");
        if branch {
            cmd = cmd.arg("--branch");
        }
        cmd.arg(name).run()?;
        Ok(())
    }

    pub fn get_object_type(&self, oid: &str) -> Result<String> {
        Ok(self.run(&["cat-file", "-t", oid])?.trim().to_string())
    }

    pub fn cat_file(&self, oid: &str) -> Result<String> {
        self.run(&["cat-file", "-p", oid])
    }

    pub fn ls_tree(&self, oid: &str) -> Result<Vec<TreeEntry>> {
        let output = self.run(&["ls-tree", oid])?;
        let mut entries = Vec::new();
        for line in output.lines() {
            let (metadata, name) = line.split_once("\t").ok_or_else(|| {
                StaircaseError::Other(format!("invalid ls-tree entry in {}", oid))
            })?;
            let fields: Vec<_> = metadata.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(StaircaseError::Other(format!(
                    "invalid ls-tree metadata in {}",
                    oid
                )));
            }
            entries.push(TreeEntry {
                mode: fields[0].to_string(),
                kind: fields[1].to_string(),
                oid: fields[2].to_string(),
                name: name.to_string(),
            });
        }
        Ok(entries)
    }

    pub fn for_each_ref(
        &self,
        pattern: &str,
        format: &str,
        points_at: Option<&str>,
    ) -> Result<Vec<String>> {
        let mut cmd = self.command();
        cmd = cmd.arg("for-each-ref").arg(format!("--format={}", format));
        if let Some(oid) = points_at {
            cmd = cmd.arg(format!("--points-at={}", oid));
        }
        let stdout = cmd.arg(pattern).run()?;
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }

    pub fn has_merges(&self, base: &str, tip: &str) -> Result<Option<String>> {
        let output = self
            .command()
            .args(&["rev-list", "--min-parents=2", &format!("{}..{}", base, tip)])
            .run()?;
        Ok(output.lines().next().map(|s| s.to_string()))
    }

    pub fn commit_tree(&self, tree_oid: &str, parents: &[&str], message: &str) -> Result<String> {
        let mut cmd = self
            .command()
            .arg("commit-tree")
            .arg(tree_oid)
            .arg("-m")
            .arg(message);
        for parent in parents {
            cmd = cmd.arg("-p").arg(*parent);
        }
        cmd.run()
    }

    pub fn update_ref(&self, refname: &str, new_oid: &str, old_oid: Option<&str>) -> Result<()> {
        let mut cmd = self.command().arg("update-ref").arg(refname).arg(new_oid);
        if let Some(old) = old_oid {
            cmd = cmd.arg(old);
        }
        cmd.run()?;
        self.memoizer.clear();
        Ok(())
    }

    pub fn read_tree_file(&self, rev: &str, path: &str) -> Result<String> {
        self.cat_file(&format!("{}:{}", rev, path))
    }
}
