use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs};
use crate::core::{self, ArchiveOptions, ArchiveResult};
use crate::git::GitRepo;
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

#[derive(Args, Clone, Debug)]
pub struct ArchiveCmd {
    #[command(subcommand)]
    pub command: Option<ArchiveSubcommands>,

    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,

    /// Reason for archiving
    #[arg(long)]
    pub reason: Option<String>,

    /// Show planned changes without performing archive
    #[arg(long)]
    pub dry_run: bool,

    /// Losslessly snapshot active worktree drafts before detach
    #[arg(long)]
    pub snapshot_drafts: bool,

    /// Detach dirty worktrees at current tip OID
    #[arg(long)]
    pub detach_dirty_worktrees: bool,

    /// Leave worktree unchanged (if not attached to a branch being removed)
    #[arg(long)]
    pub leave_worktrees: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ArchiveSubcommands {
    /// Release canonical staircase name reservation while remaining archived
    ReleaseName(ReleaseNameArgs),
}

#[derive(Args, Clone, Debug)]
pub struct ReleaseNameArgs {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Serialize, Debug, Clone)]
pub struct ArchiveOutput {
    pub result: ArchiveResult,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReleaseNameOutput {
    pub record_oid: String,
}

impl Command for ArchiveCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        if let Some(ArchiveSubcommands::ReleaseName(ref args)) = self.command {
            let sel = args.selector.resolve(repo)?;
            let oid = core::release_staircase_name(repo, &sel)?;
            return Ok(Box::new(ReleaseNameOutput { record_oid: oid }));
        }

        let sel = self.selector.resolve(repo)?;
        let options = ArchiveOptions {
            reason: self.reason.clone(),
            dry_run: self.dry_run,
            snapshot_drafts: self.snapshot_drafts,
            detach_dirty_worktrees: self.detach_dirty_worktrees,
            leave_worktrees: self.leave_worktrees,
        };

        let res = core::archive_staircase(repo, &sel, &options)?;
        Ok(Box::new(ArchiveOutput { result: res }))
    }
}
