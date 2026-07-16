use super::GitRepo;
use crate::error::{Result, StaircaseError};

impl GitRepo {
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

    pub fn has_merges(&self, base: &str, tip: &str) -> Result<Option<String>> {
        let output = self
            .command()
            .args(&["rev-list", "--min-parents=2", &format!("{}..{}", base, tip)])
            .run()?;
        Ok(output.lines().next().map(|s| s.to_string()))
    }
}
