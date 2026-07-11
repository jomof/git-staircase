use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::GitRepo;
use git_staircase::IdentityKind;
use std::path::PathBuf;

use git_staircase::cli::{self, StaircaseSelectorArgs};

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
    Reorder {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        /// New order of steps by 1-based index.
        #[arg(long, value_delimiter = ',')]
        order: Option<Vec<usize>>,
    },
    /// Move commits between steps
    Move {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(long)]
        from: usize,
        #[arg(long)]
        to: usize,
        commits: Vec<String>,
    },
    /// Drop a step from a staircase
    Drop {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        /// Step number (1-based). Can be part of the staircase name (e.g. name:1)
        #[arg(long)]
        step: Option<usize>,
    },
    /// Discover potential staircases
    Discover {
        #[arg(long)]
        onto: Option<String>,
    },
    /// Adopt a discovered staircase
    Adopt {
        name: String,
        #[arg(long)]
        onto: Option<String>,
        /// List of branch names in order (root to tip)
        branches: Vec<String>,
        #[arg(long)]
        build_command: Option<String>,
        #[arg(long)]
        test_command: Option<String>,
        #[arg(long)]
        verify_each_prefix: bool,
    },
    /// List managed staircases
    List {
        #[arg(long)]
        managed: bool,
        #[arg(long)]
        discovered: bool,
        #[arg(long, short)]
        families: bool,
        #[arg(long)]
        implicit: bool,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Show details of a staircase
    Show {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
    },
    /// Show status of a staircase (clean/stale/modified)
    Status {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
    },
    /// Split a step into two
    Split {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        /// Step number (1-based). Can be part of the staircase name (e.g. name:1)
        #[arg(long)]
        step: Option<usize>,
        #[arg(long)]
        at: String,
        /// Name of the new step.
        #[arg(long)]
        step_name: Option<String>,
    },
    /// Join two adjacent steps
    Join {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        /// First step number (1-based). Can be part of the staircase name (e.g. name:1)
        #[arg(long)]
        step: Option<usize>,
        /// Second step number (1-based).
        #[arg(long)]
        step2: Option<usize>,
        /// Second step number if not using --step2.
        step2_pos: Option<String>,
    },
    /// Rebase the entire staircase onto a new target
    Rebase {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(long = "to")]
        to: String,
    },
    /// Restack stale steps
    Restack {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
    },
    /// Verify a staircase
    Verify {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(long)]
        aggregate: bool,
        #[arg(long)]
        each_prefix: bool,
        #[arg(long)]
        build_command: Option<String>,
        #[arg(long)]
        test_command: Option<String>,
    },
    /// Show identities of a staircase
    Id {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(long, value_enum, default_value = "lineage")]
        kind: IdentityKind,
    },
    /// Delete a managed staircase
    Delete {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(long)]
        delete_branches: bool,
    },
    /// Show log for a staircase
    Log {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// Show diff for a staircase
    Diff {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// Show graph for a staircase
    Graph {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// List steps of a staircase
    Steps {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
    },
    /// List commits in each step of a staircase
    Commits {
        #[command(flatten)]
        staircase: StaircaseSelectorArgs,
    },
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

    match cli.command {
        Commands::Reorder { staircase, order } => {
            cli::dispatch(format, cli::reorder::run(&repo, staircase, order))
        }
        Commands::Move {
            staircase,
            from,
            to,
            commits,
        } => cli::dispatch(
            format,
            cli::move_cmd::run(&repo, staircase, from, to, commits),
        ),
        Commands::Drop { staircase, step } => {
            cli::dispatch(format, cli::drop::run(&repo, staircase, step))
        }
        Commands::Discover { onto } => cli::discover::run(&repo, format, onto),
        Commands::Adopt {
            name,
            onto,
            branches,
            build_command,
            test_command,
            verify_each_prefix,
        } => cli::adopt::run(
            &repo,
            format,
            name,
            onto,
            branches,
            build_command,
            test_command,
            verify_each_prefix,
        ),
        Commands::List {
            managed,
            implicit,
            onto,
            discovered,
            families,
        } => cli::dispatch(
            format,
            cli::list::run(&repo, managed, implicit, discovered, families, onto),
        ),
        Commands::Show { staircase } => cli::dispatch(format, cli::show::run(&repo, staircase)),
        Commands::Status { staircase } => cli::dispatch(format, cli::status::run(&repo, staircase)),
        Commands::Split {
            staircase,
            step,
            at,
            step_name,
        } => cli::dispatch(
            format,
            cli::split::run(&repo, staircase, step, at, step_name),
        ),
        Commands::Join {
            staircase,
            step,
            step2,
            step2_pos,
        } => cli::dispatch(
            format,
            cli::join::run(&repo, staircase, step, step2, step2_pos),
        ),
        Commands::Rebase { staircase, to } => {
            cli::dispatch(format, cli::rebase::run(&repo, staircase, to))
        }
        Commands::Restack { staircase } => {
            cli::dispatch(format, cli::restack::run(&repo, staircase))
        }
        Commands::Verify {
            staircase,
            aggregate,
            each_prefix,
            build_command,
            test_command,
        } => cli::verify::run(
            &repo,
            format,
            staircase,
            aggregate,
            each_prefix,
            build_command,
            test_command,
        ),
        Commands::Id { staircase, kind } => cli::id::run(&repo, format, staircase, kind),
        Commands::Delete {
            staircase,
            delete_branches,
        } => cli::dispatch(format, cli::delete::run(&repo, staircase, delete_branches)),
        Commands::Log {
            staircase,
            git_args,
        } => cli::log::run(&repo, format, staircase, git_args),
        Commands::Diff {
            staircase,
            git_args,
        } => cli::diff::run(&repo, format, staircase, git_args),
        Commands::Graph {
            staircase,
            git_args,
        } => cli::graph::run(&repo, format, staircase, git_args),
        Commands::Steps { staircase } => cli::dispatch(format, cli::steps::run(&repo, staircase)),
        Commands::Commits { staircase } => cli::commits::run(&repo, format, staircase),
    }
}
