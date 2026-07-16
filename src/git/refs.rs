use super::GitRepo;
use crate::core::refs::StaircaseRefs;
use crate::error::{Result, StaircaseError};
use crate::model::BranchInfo;

impl GitRepo {
    pub fn resolve_ref(&self, rev: &str) -> Result<String> {
        self.resolve_ref_opt(rev)?
            .ok_or_else(|| StaircaseError::Other(format!("Could not resolve ref: {}", rev)))
    }

    pub fn resolve_commit(&self, rev: &str) -> Result<String> {
        if rev != "HEAD" {
            if let Some(oid) = self.memoizer.get_resolve_commit(rev) {
                return Ok(oid);
            }
        }

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
                if rev != "HEAD" {
                    self.memoizer.set_resolve_commit(rev, &sha);
                }
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
                if rev != "HEAD" {
                    self.memoizer.set_resolve_commit(rev, &sha);
                }
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

        if rev != "HEAD" {
            self.memoizer.set_resolve_commit(rev, &oid);
        }
        Ok(oid)
    }

    pub fn resolve_symbolic_full_name(&self, name: &str) -> Result<String> {
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
        if rev != "HEAD" {
            if let Some(res) = self.memoizer.get_resolve_ref(rev) {
                return Ok(res);
            }
        }
        let result = self
            .command()
            .args(&["rev-parse", "--verify", rev])
            .check_status(false)
            .run_output()?;

        let res = if result.status.success() {
            Some(String::from_utf8_lossy(&result.stdout).trim().to_string())
        } else {
            None
        };
        if rev != "HEAD" {
            self.memoizer.set_resolve_ref(rev, res.as_deref());
        }
        Ok(res)
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
        let input = format!("start\n{}\nprepare\ncommit\n", commands.join("\n"));
        self.command()
            .args(&["update-ref", "--stdin"])
            .stdin(input)
            .run()?;
        self.memoizer.clear();
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

    pub fn update_ref(&self, refname: &str, new_oid: &str, old_oid: Option<&str>) -> Result<()> {
        let mut cmd = self.command().arg("update-ref").arg(refname).arg(new_oid);
        if let Some(old) = old_oid {
            cmd = cmd.arg(old);
        }
        cmd.run()?;
        self.memoizer.clear();
        Ok(())
    }
}
