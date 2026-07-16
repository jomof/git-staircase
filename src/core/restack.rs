use crate::error::{Result, StaircaseError};
use crate::git::GitRepo;
use crate::model::Step;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestackStrategy {
    Manual,
    Rebase,
}

pub struct RestackOptions {
    pub strategy: RestackStrategy,
    pub leave_upper_steps_stale: bool,
}

pub struct Restacker<'a> {
    repo: &'a GitRepo,
    original_head: Option<String>,
    original_head_oid: String,
}

impl<'a> Restacker<'a> {
    pub fn prepare(repo: &'a GitRepo, steps: &[Step]) -> Result<Self> {
        let original_head = repo.current_branch()?;
        let original_head_oid = repo.resolve_commit("HEAD")?;
        let _ = steps;
        Ok(Self {
            repo,
            original_head,
            original_head_oid,
        })
    }

    pub fn rollback(&self) {
        let _ = self.repo.run(&["rebase", "--abort"]);
        self.restore_head_silent();
    }

    fn restore_head_silent(&self) {
        if let Some(ref refname) = self.original_head {
            let target = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            let _ = self.repo.run(&["checkout", target]);
        } else {
            let _ = self.repo.run(&["checkout", &self.original_head_oid]);
        }
    }

    pub fn finalize(&self) -> Result<()> {
        if let Some(ref refname) = self.original_head {
            let target = refname.strip_prefix("refs/heads/").unwrap_or(refname);
            self.repo.run(&["checkout", target])?;
        } else {
            self.repo.run(&["checkout", &self.original_head_oid])?;
        }
        Ok(())
    }

    pub fn restack_step(
        &self,
        step: &Step,
        actual_oid: &str,
        old_parent: &str,
        new_parent: &str,
        strategy: RestackStrategy,
    ) -> Result<String> {
        match strategy {
            RestackStrategy::Manual => {
                let mut current_base = new_parent.to_string();
                if let Ok(commits) = self.repo.commits_between(old_parent, actual_oid) {
                    for c in commits {
                        let metadata = self.repo.run(&[
                            "log",
                            "-1",
                            "--format=%an%n%ae%n%at%n%cn%n%ce%n%ct%n%P",
                            &c,
                        ])?;
                        let meta_lines: Vec<&str> = metadata.lines().collect();
                        if meta_lines.len() < 6 {
                            return Err(StaircaseError::Other(format!(
                                "Failed to parse metadata for commit {}",
                                c
                            )));
                        }

                        let parents_raw = if meta_lines.len() >= 7 {
                            meta_lines[6]
                        } else {
                            ""
                        };
                        let parents_list: Vec<&str> = parents_raw.split_whitespace().collect();
                        let merge_base = if let Some(p) = parents_list.first() {
                            p
                        } else {
                            return Err(StaircaseError::Other(format!(
                                "Commit {} has no parents, cannot restack manually",
                                c
                            )));
                        };

                        let merge_output = self
                            .repo
                            .command()
                            .args(&[
                                "merge-tree",
                                "--write-tree",
                                "--merge-base",
                                merge_base,
                                &current_base,
                                &c,
                            ])
                            .check_status(false)
                            .run_output()?;

                        if !merge_output.status.success() {
                            return Err(StaircaseError::Other(format!(
                                "Conflict detected while restacking commit {}. Manual restack aborted to prevent data corruption.",
                                c
                            )));
                        }

                        let stdout = String::from_utf8_lossy(&merge_output.stdout);
                        let tree = stdout.lines().next().unwrap_or("").trim().to_string();

                        if tree.is_empty() {
                            return Err(StaircaseError::Other(format!(
                                "Failed to get tree OID from merge-tree for commit {}",
                                c
                            )));
                        }

                        let author_name = meta_lines[0];
                        let author_email = meta_lines[1];
                        let author_date = meta_lines[2];
                        let committer_name = meta_lines[3];
                        let committer_email = meta_lines[4];
                        let committer_date = meta_lines[5];

                        let mut cmd = self
                            .repo
                            .command()
                            .args(&["commit-tree", &tree])
                            .arg("-p")
                            .arg(&current_base);

                        for i in 1..parents_list.len() {
                            cmd = cmd.arg("-p").arg(parents_list[i]);
                        }

                        let msg = self.repo.run(&["log", "-1", "--format=%B", &c])?;
                        let new_c = cmd
                            .arg("-m")
                            .arg(&msg)
                            .env("GIT_AUTHOR_NAME", author_name)
                            .env("GIT_AUTHOR_EMAIL", author_email)
                            .env("GIT_AUTHOR_DATE", author_date)
                            .env("GIT_COMMITTER_NAME", committer_name)
                            .env("GIT_COMMITTER_EMAIL", committer_email)
                            .env("GIT_COMMITTER_DATE", committer_date)
                            .run()?;
                        current_base = new_c.trim().to_string();
                    }
                }
                Ok(current_base)
            }
            RestackStrategy::Rebase => {
                let mut rebase_target = actual_oid.to_string();
                if let Some(ref branch_name) = step.branch {
                    if self
                        .repo
                        .resolve_commit_opt(&format!("refs/heads/{}", branch_name))?
                        .is_some()
                    {
                        rebase_target = branch_name.clone();
                    }
                }

                self.repo.run_interactive(&[
                    "rebase",
                    "--onto",
                    new_parent,
                    old_parent,
                    &rebase_target,
                ])?;

                self.repo.resolve_commit("HEAD")
            }
        }
    }

    pub fn perform_restack(
        &self,
        _staircase_id: &str,
        steps: &mut [Step],
        base_oid: &str,
        old_parent_oids: &[String],
        options: &RestackOptions,
    ) -> Result<Option<usize>> {
        let mut current_base = base_oid.to_string();
        for i in 0..steps.len() {
            let step = &steps[i];
            let actual_oid = self.repo.resolve_commit(&step.cut)?;
            let old_parent_oid = &old_parent_oids[i];

            let should_restack = &current_base != old_parent_oid
                || !self.repo.is_ancestor(&current_base, &actual_oid)?;

            if should_restack {
                match self.restack_step(
                    step,
                    &actual_oid,
                    old_parent_oid,
                    &current_base,
                    options.strategy,
                ) {
                    Ok(new_oid) => {
                        steps[i].cut = new_oid.clone();
                        current_base = new_oid.clone();

                        if options.leave_upper_steps_stale {
                            return Ok(Some(i));
                        }
                    }
                    Err(e) => {
                        return Err(StaircaseError::Other(format!(
                            "Restack failed for step '{}'. Please resolve conflicts if using Rebase strategy.\nError: {}",
                            step.name, e
                        )));
                    }
                }
            } else {
                current_base = actual_oid.clone();
                if steps[i].cut != actual_oid {
                    steps[i].cut = actual_oid;
                }
            }
        }
        Ok(None)
    }
}
