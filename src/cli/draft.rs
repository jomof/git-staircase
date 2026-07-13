use super::PresentationOutput;
use super::formatting::PlainOutput;
use crate::GitRepo;
use crate::core::draft::{self, DraftDiffMode, MaterializeOptions};
use crate::model::{DraftIntent, RewriteMode};
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Clone, Debug)]
pub struct DraftCmd {
    #[command(subcommand)]
    pub command: DraftSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum DraftSubcommand {
    /// Show current worktree draft status
    Status(DraftStatusArgs),
    /// Show details of current worktree draft
    Show(DraftShowArgs),
    /// Show diff of worktree draft
    Diff(DraftDiffArgs),
    /// Attach draft to a staircase step
    Attach(DraftAttachArgs),
    /// Detach persistent draft attachment
    Detach(DraftDetachArgs),
    /// Create a durable snapshot of worktree draft
    Snapshot(DraftSnapshotArgs),
    /// Restore a worktree draft snapshot
    Restore(DraftRestoreArgs),
    /// Materialize staged draft into staircase commit(s)
    Materialize(DraftMaterializeArgs),
}

impl super::Command for DraftCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            DraftSubcommand::Status(_) => {
                let d = draft::get_worktree_draft(repo)?;
                Ok(Box::new(d))
            }
            DraftSubcommand::Show(_) => {
                let d = draft::get_worktree_draft(repo)?;
                Ok(Box::new(d))
            }
            DraftSubcommand::Diff(args) => {
                let mode = if args.staged {
                    DraftDiffMode::Staged
                } else if args.unstaged {
                    DraftDiffMode::Unstaged
                } else if args.combined {
                    DraftDiffMode::Combined
                } else if args.untracked {
                    DraftDiffMode::Untracked
                } else if args.ignored {
                    DraftDiffMode::Ignored
                } else {
                    DraftDiffMode::Staged
                };
                let diff = draft::diff_draft(repo, mode)?;
                Ok(Box::new(PlainOutput(diff)))
            }
            DraftSubcommand::Attach(args) => {
                let intent = args.mode.as_ref().map(|m| match m {
                    DraftModeArg::ExtendStep => DraftIntent::ExtendStep,
                    DraftModeArg::NewStep => DraftIntent::NewStep,
                    DraftModeArg::Amend => DraftIntent::RewriteStep(RewriteMode::Amend),
                    DraftModeArg::Fixup => DraftIntent::RewriteStep(RewriteMode::Fixup),
                });
                let att = draft::attach_draft(repo, &args.staircase, args.step.as_deref(), intent)?;
                Ok(Box::new(att))
            }
            DraftSubcommand::Detach(_) => {
                draft::detach_draft(repo)?;
                Ok(Box::new(PlainOutput(
                    "Detached draft attachment.".to_string(),
                )))
            }
            DraftSubcommand::Snapshot(args) => {
                let snap = draft::create_snapshot(repo, args.name.as_deref())?;
                Ok(Box::new(snap))
            }
            DraftSubcommand::Restore(args) => {
                let snap = draft::restore_snapshot(repo, &args.snapshot_id)?;
                Ok(Box::new(snap))
            }
            DraftSubcommand::Materialize(args) => {
                let intent = if args.new_step {
                    Some(DraftIntent::NewStep)
                } else if args.extend_step {
                    Some(DraftIntent::ExtendStep)
                } else if args.amend {
                    Some(DraftIntent::RewriteStep(RewriteMode::Amend))
                } else if args.fixup {
                    Some(DraftIntent::RewriteStep(RewriteMode::Fixup))
                } else if let Some(ref step) = args.fold_into {
                    Some(DraftIntent::RewriteStep(RewriteMode::FoldInto(
                        step.clone(),
                    )))
                } else {
                    None
                };

                let opts = MaterializeOptions {
                    all_tracked: args.all_tracked,
                    include_untracked: args.include_untracked,
                    include_ignored: args.include_ignored,
                    allow_empty: args.allow_empty || args.empty,
                    message: args.message.clone(),
                    paths: args.paths.clone(),
                    preserve_draft: args.preserve_draft,
                };

                let res = draft::materialize_draft(repo, args.staircase.as_deref(), intent, &opts)?;
                Ok(Box::new(res))
            }
        }
    }
}

#[derive(Args, Clone, Debug)]
pub struct DraftStatusArgs {}

#[derive(Args, Clone, Debug)]
pub struct DraftShowArgs {}

#[derive(Args, Clone, Debug)]
pub struct DraftDiffArgs {
    #[arg(long)]
    pub staged: bool,
    #[arg(long)]
    pub unstaged: bool,
    #[arg(long)]
    pub combined: bool,
    #[arg(long)]
    pub untracked: bool,
    #[arg(long)]
    pub ignored: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DraftModeArg {
    ExtendStep,
    NewStep,
    Amend,
    Fixup,
}

#[derive(Args, Clone, Debug)]
pub struct DraftAttachArgs {
    /// Staircase name
    pub staircase: String,
    /// Step name or ID
    #[arg(long)]
    pub step: Option<String>,
    /// Draft mode / intent
    #[arg(long, value_enum)]
    pub mode: Option<DraftModeArg>,
}

#[derive(Args, Clone, Debug)]
pub struct DraftDetachArgs {}

#[derive(Args, Clone, Debug)]
pub struct DraftSnapshotArgs {
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct DraftRestoreArgs {
    /// Snapshot ID
    pub snapshot_id: String,
}

#[derive(Args, Clone, Debug)]
pub struct DraftMaterializeArgs {
    /// Target staircase name
    pub staircase: Option<String>,
    #[arg(long)]
    pub new_step: bool,
    #[arg(long)]
    pub extend_step: bool,
    #[arg(long)]
    pub amend: bool,
    #[arg(long)]
    pub fixup: bool,
    #[arg(long)]
    pub fold_into: Option<String>,
    #[arg(long)]
    pub all_tracked: bool,
    #[arg(long)]
    pub include_untracked: bool,
    #[arg(long)]
    pub include_ignored: bool,
    #[arg(long)]
    pub allow_empty: bool,
    #[arg(long)]
    pub empty: bool,
    #[arg(short, long)]
    pub message: Option<String>,
    #[arg(long)]
    pub preserve_draft: bool,
    /// Paths to materialize
    #[arg(num_args = 0..)]
    pub paths: Vec<String>,
}
