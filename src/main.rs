use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::GitRepo;
use std::path::PathBuf;

use git_staircase::cli::{self, Command};

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
}

#[derive(Subcommand)]
enum Commands {
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
    /// List commits in each step of a staircase
    Commits(cli::commits::Commits),
}

impl Commands {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn cli::PresentationOutput>> {
        match self {
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
            Commands::Commits(cmd) => cmd.run(repo),
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
    let format = if cli.json {
        cli::OutputFormat::Json
    } else if cli.porcelain {
        cli::OutputFormat::Porcelain
    } else {
        cli::OutputFormat::Human
    };

    cli::dispatch(format, &repo, cli.command.run(&repo))
}
