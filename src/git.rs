use crate::core::refs::StaircaseRefs;
use crate::error::{Result, StaircaseError};
use crate::memoization::Memoizer;
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
        cmd
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
        self.command().args(&["rev-parse", "--verify", rev]).run()
    }

    pub fn resolve_commit(&self, rev: &str) -> Result<String> {
        if !rev.starts_with("refs/") {
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
        }
        self.command()
            .args(&["rev-parse", "--verify", &format!("{}^{{commit}}", rev)])
            .run()
    }

    pub fn resolve_symbolic_full_name(&self, name: &str) -> Result<String> {
        if name.starts_with("refs/") {
            return Ok(name.to_string());
        }
        if name != "HEAD" {
            if let Some(res) = self.memoizer.get_symbolic_name(name) {
                return Ok(res);
            }
        }
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
        if name != "HEAD" {
            self.memoizer.set_symbolic_name(name, &full_name);
        }
        Ok(full_name)
    }

    pub fn resolve_commit_opt(&self, rev: &str) -> Result<Option<String>> {
        match self.resolve_commit(rev) {
            Ok(sha) => Ok(Some(sha)),
            Err(_) => Ok(None),
        }
    }

    pub fn resolve_ref_opt(&self, rev: &str) -> Result<Option<String>> {
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
        if let Some(res) = self.memoizer.get_ancestry(&anc_oid, &desc_oid) {
            return Ok(res);
        }

        let output = self
            .command()
            .args(&["merge-base", "--is-ancestor", &anc_oid, &desc_oid])
            .check_status(false)
            .run_output()?;

        let res = match output.status.code() {
            Some(0) => true,
            Some(1) => false,
            _ => {
                if !output.status.success() {
                    return Err(StaircaseError::GitCommandFailed {
                        command: format!("git merge-base --is-ancestor {} {}", anc_oid, desc_oid),
                        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    });
                }
                false
            }
        };

        self.memoizer.set_ancestry(&anc_oid, &desc_oid, res);
        Ok(res)
    }

    pub fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        let a_oid = self.resolve_commit(a)?;
        let b_oid = self.resolve_commit(b)?;
        if let Some(mb) = self.memoizer.get_merge_base(&a_oid, &b_oid) {
            return Ok(mb);
        }
        let mb = self.command().args(&["merge-base", &a_oid, &b_oid]).run()?;
        self.memoizer.set_merge_base(&a_oid, &b_oid, &mb);
        Ok(mb)
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
        if let Some(fmt) = self.memoizer.get_object_format() {
            return Ok(fmt);
        }
        let fmt = self
            .command()
            .args(&["rev-parse", "--show-object-format"])
            .run()?;
        self.memoizer.set_object_format(&fmt);
        Ok(fmt)
    }

    pub fn get_tree_id(&self, rev: &str) -> Result<String> {
        let commit_oid = self.resolve_commit(rev)?;
        if let Some(tree) = self.memoizer.get_tree_id(&commit_oid) {
            return Ok(tree);
        }
        let tree = self
            .command()
            .args(&["rev-parse", &format!("{}^{{tree}}", commit_oid)])
            .run()?;
        self.memoizer.set_tree_id(&commit_oid, &tree);
        Ok(tree)
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
        if let Some(hash) = self.memoizer.get_hash_data(data) {
            return Ok(hash);
        }
        let hash = self
            .command()
            .args(&["hash-object", "--stdin"])
            .stdin(data)
            .run()?;
        self.memoizer.set_hash_data(data, &hash);
        Ok(hash)
    }

    pub fn get_patch_id(&self, base: &str, tip: &str) -> Result<String> {
        if let Some(pid) = self.memoizer.get_patch_id(base, tip) {
            return Ok(pid);
        }
        let diff = self.command().args(&["diff-tree", "-p", base, tip]).run()?;
        let stdout = self.command().args(&["patch-id"]).stdin(diff).run()?;
        let pid = stdout.split_whitespace().next().unwrap_or("").to_string();
        self.memoizer.set_patch_id(base, tip, &pid);
        Ok(pid)
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

    pub fn update_refs_transaction(&self, commands: &[String]) -> Result<()> {
        if commands.is_empty() {
            return Ok(());
        }
        let input = commands.join("\n") + "\n";
        self.command()
            .args(&["update-ref", "--stdin"])
            .stdin(input)
            .run()?;
        self.memoizer.clear();
        Ok(())
    }
}
