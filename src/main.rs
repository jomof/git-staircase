use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::GitRepo;
use git_staircase::IdentityKind;
use std::path::PathBuf;

pub mod cli;

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
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<usize>>,
        #[arg(long, value_delimiter = ',')]
        staircase_steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Move commits between steps
    Move {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        from: usize,
        #[arg(long)]
        to: usize,
        #[arg(long)]
        onto: Option<String>,
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
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Show status of a staircase (clean/stale/modified)
    Status {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
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
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: String,
        #[arg(long)]
        resolve_onto: Option<String>,
    },
    /// Restack stale steps
    Restack {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Verify a staircase
    Verify {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
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
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
        #[arg(long, value_enum, default_value = "lineage")]
        kind: IdentityKind,
    },
    /// Delete a managed staircase
    Delete {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
        #[arg(long)]
        delete_branches: bool,
    },
    /// Show log for a staircase
    Log {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// Show diff for a staircase
    Diff {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// Show graph for a staircase
    Graph {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
        #[arg(last = true)]
        git_args: Vec<String>,
    },
    /// List steps of a staircase
    Steps {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
    },
    /// List commits in each step of a staircase
    Commits {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        steps: Option<Vec<String>>,
        #[arg(long)]
        onto: Option<String>,
    },
}

fn find_repo_root() -> anyhow::Result<PathBuf> {
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

pub fn parse_step_spec(spec: &str) -> anyhow::Result<(String, usize)> {
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

fn main() -> anyhow::Result<()> {
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
            name,
            steps,
            staircase_steps,
            onto,
        } => cli::reorder::run(&repo, format, name, steps, staircase_steps, onto),
        Commands::Move {
            name,
            steps,
            from,
            to,
            onto,
            commits,
        } => cli::move_cmd::run(&repo, format, name, steps, from, to, onto, commits),
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
        Commands::Show { name, steps, onto } => cli::show::run(&repo, format, name, steps, onto),
        Commands::Status { name, steps, onto } => {
            cli::status::run(&repo, format, name, steps, onto)
        }
        Commands::Split {
            step,
            at,
            name,
            onto,
        } => cli::split::run(&repo, format, step, at, name, onto),
        Commands::Join { step1, step2, onto } => cli::join::run(&repo, format, step1, step2, onto),
        Commands::Rebase {
            name,
            steps,
            onto,
            resolve_onto,
        } => cli::rebase::run(&repo, format, name, steps, onto, resolve_onto),
        Commands::Restack { name, steps, onto } => {
            cli::restack::run(&repo, format, name, steps, onto)
        }
        Commands::Verify {
            name,
            steps,
            onto,
            aggregate,
            each_prefix,
            build_command,
            test_command,
        } => cli::verify::run(
            &repo,
            format,
            name,
            steps,
            onto,
            aggregate,
            each_prefix,
            build_command,
            test_command,
        ),
        Commands::Id {
            name,
            steps,
            kind,
            onto,
        } => cli::id::run(&repo, format, name, steps, kind, onto),
        Commands::Delete {
            name,
            steps,
            onto,
            delete_branches,
        } => cli::delete::run(&repo, format, name, steps, onto, delete_branches),
        Commands::Log {
            name,
            steps,
            onto,
            git_args,
        } => cli::log::run(&repo, format, name, steps, onto, git_args),
        Commands::Diff {
            name,
            steps,
            onto,
            git_args,
        } => cli::diff::run(&repo, format, name, steps, onto, git_args),
        Commands::Graph {
            name,
            steps,
            onto,
            git_args,
        } => cli::graph::run(&repo, format, name, steps, onto, git_args),
        Commands::Steps { name, steps, onto } => cli::steps::run(&repo, format, name, steps, onto),
        Commands::Commits { name, steps, onto } => {
            cli::commits::run(&repo, format, name, steps, onto)
        }
    }
}
