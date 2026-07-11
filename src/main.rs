use anyhow::{Context, anyhow, Result};
use clap::{Parser, Subcommand};
use git_staircase::GitRepo;
use git_staircase::IdentityKind;
use std::path::PathBuf;

pub mod cli;

use cli::StaircaseSelectorArgs;

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
        /// Format: <staircase_name>:<step_number> (1-based)
        step: String,
        #[arg(long)]
        onto: Option<String>,
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
        /// Format: <staircase_name>:<step_number> (1-based)
        step: String,
        #[arg(long)]
        onto: Option<String>,
        #[arg(long)]
        at: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Join two adjacent steps
    Join {
        /// Format: <staircase_name>:<step_number> (1-based)
        step1: String,
        /// Format: <staircase_name>:<step_number> (1-based)
        step2: String,
        #[arg(long)]
        onto: Option<String>,
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

pub fn parse_step_spec(spec: &str) -> Result<(String, usize)> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid step spec '{}'. Expected format: <staircase_name>:<step_number>",
            spec
        ));
    }
    let name = parts[0].to_string();
    let num = parts[1]
        .parse::<usize>()
        .context("Failed to parse step number")?;
    Ok((name, num))
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
        Commands::Reorder {
            staircase,
            order,
        } => cli::reorder::run(&repo, format, staircase, order),
        Commands::Move {
            staircase,
            from,
            to,
            commits,
        } => cli::move_cmd::run(&repo, format, staircase, from, to, commits),
        Commands::Drop { step, onto } => cli::drop::run(&repo, format, step, onto),
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
        } => cli::list::run(&repo, format, managed, implicit, onto),
        Commands::Show { staircase } => cli::show::run(&repo, format, staircase),
        Commands::Status { staircase } => cli::status::run(&repo, format, staircase),
        Commands::Split {
            step,
            at,
            name,
            onto,
        } => cli::split::run(&repo, format, step, at, name, onto),
        Commands::Join { step1, step2, onto } => cli::join::run(&repo, format, step1, step2, onto),
        Commands::Rebase {
            staircase,
            to,
        } => cli::rebase::run(&repo, format, staircase, to),
        Commands::Restack { staircase } => {
            cli::restack::run(&repo, format, staircase)
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
        Commands::Id {
            staircase,
            kind,
        } => cli::id::run(&repo, format, staircase, kind),
        Commands::Delete {
            staircase,
            delete_branches,
        } => cli::delete::run(&repo, format, staircase, delete_branches),
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
        Commands::Steps { staircase } => cli::steps::run(&repo, format, staircase),
        Commands::Commits { staircase } => {
            cli::commits::run(&repo, format, staircase)
        }
    }
}
