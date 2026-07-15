use super::{Presentation, PresentationOutput, ToPresentation};
use crate::GitRepo;
use crate::monorepo::{
    CreateWorktreeOptions, MonorepoWorktreeEntry, create_monorepo_worktree, load_registry,
    prune_monorepo_worktrees, remove_monorepo_worktree,
};
use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub struct MonorepoCmd {
    #[command(subcommand)]
    pub command: MonorepoSubcommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum MonorepoSubcommands {
    /// Worktree image management for single and multi-repository monorepos
    Worktree(WorktreeCmd),
}

#[derive(Args, Clone, Debug)]
pub struct WorktreeCmd {
    #[command(subcommand)]
    pub command: WorktreeSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum WorktreeSubcommand {
    /// Create a new monorepo worktree
    Create(CreateWorktreeArgs),
    /// List active monorepo worktrees
    List(ListWorktreeArgs),
    /// Remove a monorepo worktree
    Remove(RemoveWorktreeArgs),
    /// Prune stale or orphaned monorepo worktrees
    Prune(PruneWorktreeArgs),
}

#[derive(Args, Clone, Debug)]
pub struct CreateWorktreeArgs {
    /// Path to active repository (multiple --repo flags may be specified)
    #[arg(short, long = "repo", num_args = 0..)]
    pub repos: Vec<PathBuf>,

    /// Create/checkout specified branch in active repository
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Checkout specified commit (detached HEAD) in active repository
    #[arg(short, long)]
    pub commit: Option<String>,

    /// Set upstream branch for created branch
    #[arg(long)]
    pub upstream: Option<String>,

    /// Base branch to branch off from
    #[arg(long)]
    pub base: Option<String>,

    /// Human-readable name/prefix for worktree directory
    #[arg(long)]
    pub name: Option<String>,

    /// Optional target path for the worktree
    pub path: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct ListWorktreeArgs {}

#[derive(Args, Clone, Debug)]
pub struct RemoveWorktreeArgs {
    /// Worktree ID or path to remove
    pub id_or_path: String,

    /// Force removal
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Clone, Debug)]
pub struct PruneWorktreeArgs {
    /// Force removal of old worktrees
    #[arg(long)]
    pub force: bool,

    /// Maximum age duration (e.g. 7d)
    #[arg(long)]
    pub age: Option<String>,
}

impl MonorepoCmd {
    pub fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            MonorepoSubcommands::Worktree(wt_cmd) => match &wt_cmd.command {
                WorktreeSubcommand::Create(args) => {
                    let options = CreateWorktreeOptions {
                        repo_paths: args.repos.clone(),
                        branch: args.branch.clone(),
                        commit: args.commit.clone(),
                        upstream: args.upstream.clone(),
                        base: args.base.clone(),
                        name: args.name.clone(),
                        custom_target_path: args.path.clone(),
                    };
                    let entry = create_monorepo_worktree(repo, &options)?;
                    Ok(Box::new(CreateWorktreeOutput(entry)))
                }
                WorktreeSubcommand::List(_) => {
                    let reg = load_registry()?;
                    Ok(Box::new(ListWorktreesOutput(reg.worktrees)))
                }
                WorktreeSubcommand::Remove(args) => {
                    let removed = remove_monorepo_worktree(&args.id_or_path, args.force)?;
                    let message = if removed {
                        format!("Removed monorepo worktree '{}'", args.id_or_path)
                    } else {
                        format!("No monorepo worktree found matching '{}'", args.id_or_path)
                    };
                    Ok(Box::new(MonorepoMessageOutput(message)))
                }
                WorktreeSubcommand::Prune(args) => {
                    let pruned = prune_monorepo_worktrees(args.force, None)?;
                    let message = format!("Pruned {} monorepo worktree(s)", pruned.len());
                    Ok(Box::new(MonorepoMessageOutput(message)))
                }
            },
        }
    }
}

#[derive(Serialize)]
pub struct CreateWorktreeOutput(pub MonorepoWorktreeEntry);

impl ToPresentation for CreateWorktreeOutput {
    fn to_presentation(&self) -> Presentation {
        let path_str = self.0.path.display().to_string();
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(path_str.clone()))),
            Presentation::Porcelain(Box::new(Presentation::Plain(path_str))),
        ])
    }
}

#[derive(Serialize)]
pub struct ListWorktreesOutput(pub Vec<MonorepoWorktreeEntry>);

impl ToPresentation for ListWorktreesOutput {
    fn to_presentation(&self) -> Presentation {
        let human_items = self
            .0
            .iter()
            .map(|w| {
                let active_str = w
                    .active_repos
                    .iter()
                    .map(|r| r.relative_path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Presentation::Section {
                    title: format!("Worktree: {}", w.id),
                    children: vec![
                        Presentation::Field {
                            label: "Path".into(),
                            value: w.path.display().to_string(),
                        },
                        Presentation::Field {
                            label: "Primary Root".into(),
                            value: w.primary_root.display().to_string(),
                        },
                        Presentation::Field {
                            label: "Active Repos".into(),
                            value: active_str,
                        },
                        Presentation::Field {
                            label: "Created At".into(),
                            value: w.created_at.clone(),
                        },
                    ],
                }
            })
            .collect();

        let porcelain_records = self
            .0
            .iter()
            .map(|w| {
                let active_str = w
                    .active_repos
                    .iter()
                    .map(|r| r.relative_path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                Presentation::Record(vec![
                    w.id.clone(),
                    w.path.display().to_string(),
                    w.primary_root.display().to_string(),
                    active_str,
                    w.created_at.clone(),
                ])
            })
            .collect();

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::List(human_items))),
            Presentation::Porcelain(Box::new(Presentation::List(porcelain_records))),
        ])
    }
}

#[derive(Serialize)]
pub struct MonorepoMessageOutput(pub String);

impl ToPresentation for MonorepoMessageOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.0.clone())
    }
}
