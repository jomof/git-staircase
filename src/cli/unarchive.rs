use anyhow::{Result, anyhow};
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs, ToHuman, ToPorcelain};
use crate::core::{self, UnarchiveBranchesMode, UnarchiveOptions, UnarchiveResult};
use crate::git::GitRepo;
use clap::Args;
use serde::Serialize;

#[derive(Args, Clone, Debug)]
pub struct UnarchiveCmd {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,

    /// Restore canonical staircase name as <name>
    #[arg(long)]
    pub name: Option<String>,

    /// Rename branches using sequential layout base name
    #[arg(long)]
    pub branch_base: Option<String>,

    /// Branch restoration mode (exact, rename, none)
    #[arg(long, default_value = "exact")]
    pub branches: String,

    /// Allow adopting existing local branches pointing to step cuts
    #[arg(long)]
    pub adopt_existing_branches: bool,

    /// Reattach eligible worktrees
    #[arg(long)]
    pub reattach_worktrees: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct UnarchiveOutput {
    pub result: UnarchiveResult,
}

impl ToHuman for UnarchiveOutput {
    fn to_human(&self) -> String {
        let mut out = format!(
            "Restored staircase '{}' ({}) to active state\n",
            self.result.canonical_name, self.result.restored_staircase_id
        );
        if !self.result.restored_branches.is_empty() {
            out.push_str("Restored local branches:\n");
            for b in &self.result.restored_branches {
                out.push_str(&format!("  refs/heads/{}\n", b));
            }
        }
        out
    }
}

impl ToPorcelain for UnarchiveOutput {
    fn to_porcelain(&self) -> String {
        format!(
            "unarchived\t{}\t{}",
            self.result.canonical_name, self.result.restored_staircase_id
        )
    }
}

impl Command for UnarchiveCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let sel = self.selector.resolve(repo)?;

        let branches_mode = match self.branches.as_str() {
            "exact" => UnarchiveBranchesMode::Exact,
            "rename" => UnarchiveBranchesMode::Rename,
            "none" => UnarchiveBranchesMode::None,
            other => {
                return Err(anyhow!(
                    "Invalid --branches mode '{}' (expected exact, rename, or none)",
                    other
                ));
            }
        };

        let options = UnarchiveOptions {
            new_name: self.name.clone(),
            branch_base: self.branch_base.clone(),
            branches_mode,
            adopt_existing_branches: self.adopt_existing_branches,
            reattach_worktrees: self.reattach_worktrees,
        };

        let res = core::unarchive_staircase(repo, &sel, &options)?;
        Ok(Box::new(UnarchiveOutput { result: res }))
    }
}
