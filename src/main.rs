use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::GitRepo;
use std::path::PathBuf;

use git_staircase::cli::{self, Command};
use git_staircase::workspace::{BootstrapOptions, bootstrap};

#[derive(Parser)]
#[command(name = "git-staircase")]
#[command(about = "Manage git staircases", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    porcelain: bool,

    #[arg(long, global = true)]
    format: Option<String>,

    #[arg(long, global = true)]
    no_bootstrap: bool,

    #[arg(long, global = true)]
    no_configure: bool,

    #[arg(long, global = true)]
    workspace: Option<String>,

    #[arg(long, global = true)]
    workspace_provider: Option<String>,

    #[arg(long, global = true)]
    review_provider: Option<String>,

    #[arg(long, global = true)]
    provider_profile: Option<String>,

    #[arg(long, global = true)]
    workspace_mode: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Workspace configuration and provider management
    Workspace(cli::workspace::WorkspaceCmd),
    /// Land a staircase into its target branch
    Land(cli::land::Land),
    /// Reorder steps of a staircase
    Reorder(cli::reorder::Reorder),
    /// Move commits between steps
    Move(cli::move_cmd::Move),
    /// Drop a step from a staircase
    Drop(cli::drop::Drop),
    /// Discover potential staircases
    Discover(cli::discover::Discover),
    /// Adopt a discovered staircase
    Adopt(cli::adopt::Adopt),
    /// List managed staircases
    List(cli::list::List),
    /// Show details of a staircase
    Show(cli::show::Show),
    /// Show status of a staircase (clean/stale/modified)
    Status(cli::status::Status),
    /// Split a step into two
    Split(cli::split::Split),
    /// Join two adjacent steps
    Join(cli::join::Join),
    /// Rebase the entire staircase onto a new target
    Rebase(cli::rebase::Rebase),
    /// Restack stale steps
    Restack(cli::restack::Restack),
    /// Verify a staircase
    Verify(cli::verify::Verify),
    /// Show identities of a staircase
    Id(cli::id::Id),
    /// Delete a managed staircase
    Delete(cli::delete::Delete),
    /// Show log for a staircase
    Log(cli::log::Log),
    /// Show diff for a staircase
    Diff(cli::diff::Diff),
    /// Show graph for a staircase
    Graph(cli::graph::Graph),
    /// List steps of a staircase
    Steps(cli::steps::Steps),
    /// Review management and Gerrit integration
    Review(cli::review::ReviewCmd),
    /// List commits in each step of a staircase
    Commits(cli::commits::Commits),
    /// Worktree draft management and materialization
    Draft(cli::draft::DraftCmd),
    /// Describe staircase title and description
    Describe(cli::describe::Describe),
    /// User-facing metadata management
    Metadata(cli::metadata::MetadataCmd),
    /// Archive a staircase
    Archive(cli::archive::ArchiveCmd),
    /// Unarchive a staircase
    Unarchive(cli::unarchive::UnarchiveCmd),
}

impl Commands {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn cli::PresentationOutput>> {
        match self {
            Commands::Workspace(cmd) => cmd.run(repo),
            Commands::Land(cmd) => cmd.run(repo),
            Commands::Reorder(cmd) => cmd.run(repo),
            Commands::Move(cmd) => cmd.run(repo),
            Commands::Drop(cmd) => cmd.run(repo),
            Commands::Discover(cmd) => cmd.run(repo),
            Commands::Adopt(cmd) => cmd.run(repo),
            Commands::List(cmd) => cmd.run(repo),
            Commands::Show(cmd) => cmd.run(repo),
            Commands::Status(cmd) => cmd.run(repo),
            Commands::Split(cmd) => cmd.run(repo),
            Commands::Join(cmd) => cmd.run(repo),
            Commands::Rebase(cmd) => cmd.run(repo),
            Commands::Restack(cmd) => cmd.run(repo),
            Commands::Verify(cmd) => cmd.run(repo),
            Commands::Id(cmd) => cmd.run(repo),
            Commands::Delete(cmd) => cmd.run(repo),
            Commands::Log(cmd) => cmd.run(repo),
            Commands::Diff(cmd) => cmd.run(repo),
            Commands::Graph(cmd) => cmd.run(repo),
            Commands::Steps(cmd) => cmd.run(repo),
            Commands::Review(cmd) => cmd.run(repo),
            Commands::Commits(cmd) => cmd.run(repo),
            Commands::Draft(cmd) => cmd.run(repo),
            Commands::Describe(cmd) => cmd.run(repo),
            Commands::Metadata(cmd) => cmd.run(repo),
            Commands::Archive(cmd) => cmd.run(repo),
            Commands::Unarchive(cmd) => cmd.run(repo),
        }
    }
}

fn find_repo_root() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .context("Failed to run git rev-parse")?;
    if !output.status.success() {
        return Err(anyhow!(
            "Not a git repository (or any parent up to mount point)"
        ));
    }
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path_str))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = find_repo_root()?;
    let repo = GitRepo::new(repo_root);
    let is_json = cli.json || matches!(cli.format.as_deref(), Some("json"));
    let is_porcelain = cli.porcelain || matches!(cli.format.as_deref(), Some("porcelain"));
    let format = if is_json {
        cli::OutputFormat::Json
    } else if is_porcelain {
        cli::OutputFormat::Porcelain
    } else {
        cli::OutputFormat::Human
    };

    let options = BootstrapOptions {
        no_bootstrap: cli.no_bootstrap,
        no_configure: cli.no_configure,
        workspace_id: cli.workspace,
        workspace_provider: cli.workspace_provider,
        review_provider: cli.review_provider,
        provider_profile: cli.provider_profile,
        workspace_mode: cli.workspace_mode,
        is_porcelain_or_json: cli.json || cli.porcelain,
    };

    let bootstrap_res = bootstrap(&repo, &options)?;
    if let Some(ref msg) = bootstrap_res.message {
        if matches!(format, cli::OutputFormat::Human) {
            eprintln!("{}", msg);
        }
    }

    cli::dispatch(format, &repo, cli.command.run(&repo))
}
