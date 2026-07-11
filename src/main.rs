use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::core;
use git_staircase::{Discovery, GitRepo, StaircaseFamily, StaircaseMetadata, Step};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "git-staircase")]
#[command(about = "Manage git staircases", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover potential staircases
    Discover {
        #[arg(long, default_value = "main")]
        onto: String,
    },
    /// Adopt a discovered staircase
    Adopt {
        name: String,
        #[arg(long, default_value = "main")]
        onto: String,
        /// List of branch names in order (root to tip)
        branches: Vec<String>,
    },
    /// List managed staircases
    List {
        #[arg(long)]
        managed: bool,
        #[arg(long)]
        discovered: bool,
        #[arg(long, default_value = "main")]
        onto: String,
    },
    /// Show details of a staircase
    Show { name: String },
    /// Show status of a staircase (clean/stale/modified)
    Status { name: String },
    /// Split a step into two
    Split {
        /// Format: <staircase_name>:<step_number> (1-based)
        step: String,
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
    },
    /// Rebase the entire staircase onto a new target
    Rebase {
        name: String,
        #[arg(long)]
        onto: String,
    },
    /// Restack stale steps
    Restack { name: String },
    /// Delete a managed staircase
    Delete {
        name: String,
        #[arg(long)]
        delete_branches: bool,
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let repo_root = find_repo_root()?;
    let repo = GitRepo::new(repo_root);

    match cli.command {
        Commands::Discover { onto } => {
            let discovered = core::discover(&repo, &onto)?;
            if discovered.is_empty() {
                println!("No potential staircases discovered relative to '{}'.", onto);
            } else {
                for (i, d) in discovered.iter().enumerate() {
                    match d {
                        Discovery::Linear(s) => {
                            println!("Discovered Staircase {}:", i + 1);
                            print_staircase(s);
                        }
                        Discovery::Ambiguous(f) => {
                            println!("Discovered Ambiguous Family {}:", i + 1);
                            print_family(f);
                        }
                    }
                    println!();
                }
            }
        }
        Commands::Adopt {
            name,
            onto,
            branches,
        } => {
            if branches.is_empty() {
                return Err(anyhow!("At least one branch must be specified to adopt"));
            }
            // We need to resolve branches to OIDs to build the Step list
            let mut steps = Vec::new();
            for b in branches {
                let full_ref = if b.starts_with("refs/heads/") {
                    b.clone()
                } else {
                    format!("refs/heads/{}", b)
                };
                let oid = repo
                    .resolve_ref(&full_ref)
                    .with_context(|| format!("Failed to resolve branch '{}'", b))?;
                let short_name = b.strip_prefix("refs/heads/").unwrap_or(&b).to_string();
                steps.push(Step {
                    name: short_name.clone(),
                    cut: oid,
                    branch: Some(short_name),
                });
            }

            let staircase = StaircaseMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                target: onto,
                steps,
            };

            core::adopt(&repo, &staircase)?;
            println!("Adopted staircase '{}' (ID: {}).", name, staircase.id);
        }
        Commands::List {
            managed,
            discovered,
            onto,
        } => {
            let show_all = !managed && !discovered;

            if managed || show_all {
                let list = repo.list_staircases()?;
                if !list.is_empty() {
                    println!("Managed Staircases:");
                    for s in list {
                        println!("  {} (id: {})", s.name, s.id);
                    }
                } else if managed {
                    println!("No managed staircases found.");
                }
            }

            if discovered || show_all {
                let list = core::discover(&repo, &onto)?;
                if !list.is_empty() {
                    println!("Discovered Staircases (relative to {}):", onto);
                    for d in list {
                        match d {
                            Discovery::Linear(s) => {
                                println!(
                                    "  {} (branches: {})",
                                    s.name,
                                    s.steps
                                        .iter()
                                        .map(|s| s.name.as_str())
                                        .collect::<Vec<&str>>()
                                        .join(" -> ")
                                );
                            }
                            Discovery::Ambiguous(f) => {
                                println!(
                                    "  {} [AMBIGUOUS FAMILY] ({} branches)",
                                    f.name,
                                    f.steps.len()
                                );
                            }
                        }
                    }
                } else if discovered {
                    println!("No potential staircases discovered.");
                }
            }
        }
        Commands::Show { name } => {
            let s = core::find_by_name(&repo, &name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            print_staircase(&s);
        }
        Commands::Status { name } => {
            let s = core::find_by_name(&repo, &name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            let status = core::get_status(&repo, &s.id)?;
            print_status(&status);
        }
        Commands::Split { step, at, name } => {
            let (sc_name, step_num) = parse_step_spec(&step)?;
            let s = core::find_by_name(&repo, &sc_name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name))?;

            if step_num == 0 {
                return Err(anyhow!("Step number must be 1-based"));
            }
            core::split(&repo, &s.id, step_num - 1, &at, name.as_deref())?;
            println!(
                "Split step {} of staircase '{}' at {}.",
                step_num, sc_name, at
            );
        }
        Commands::Join { step1, step2 } => {
            let (sc_name1, step_num1) = parse_step_spec(&step1)?;
            let (sc_name2, step_num2) = parse_step_spec(&step2)?;

            if sc_name1 != sc_name2 {
                return Err(anyhow!(
                    "Cannot join steps from different staircases: '{}' and '{}'",
                    sc_name1,
                    sc_name2
                ));
            }

            let s = core::find_by_name(&repo, &sc_name1)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name1))?;

            if step_num1 == 0 || step_num2 == 0 {
                return Err(anyhow!("Step numbers must be 1-based"));
            }

            core::join(&repo, &s.id, step_num1 - 1, step_num2 - 1)?;
            println!(
                "Joined steps {} and {} of staircase '{}'.",
                step_num1, step_num2, sc_name1
            );
        }
        Commands::Rebase { name, onto } => {
            let s = core::find_by_name(&repo, &name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::rebase(&repo, &s.id, &onto)?;
            println!("Rebased staircase '{}' onto '{}'.", name, onto);
        }
        Commands::Restack { name } => {
            let s = core::find_by_name(&repo, &name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::restack(&repo, &s.id)?;
            println!("Restacked staircase '{}'.", name);
        }
        Commands::Delete {
            name,
            delete_branches,
        } => {
            let s = core::find_by_name(&repo, &name)?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::delete(&repo, &s.id, delete_branches)?;
            println!("Deleted staircase '{}'.", name);
        }
    }

    Ok(())
}

fn parse_step_spec(spec: &str) -> anyhow::Result<(String, usize)> {
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

fn print_staircase(s: &StaircaseMetadata) {
    println!("  Name: {}", s.name);
    println!("  ID: {}", s.id);
    println!("  Target: {}", s.target);
    println!("  Steps:");
    for (i, step) in s.steps.iter().enumerate() {
        println!("    Step {}:", i + 1);
        println!("      Name: {}", step.name);
        println!("      Cut: {}", step.cut);
        if let Some(ref b) = step.branch {
            println!("      Branch: {}", b);
        }
    }
}

fn print_status(status: &git_staircase::StaircaseStatus) {
    println!("Staircase: {}", status.metadata.name);
    println!("ID: {}", status.metadata.id);
    println!("Target: {}", status.metadata.target);
    println!("Clean: {}", status.is_clean);
    println!("Steps:");
    for (i, step) in status.steps.iter().enumerate() {
        let meta_step = &status.metadata.steps[i];
        print!("  Step {} ({}):", i + 1, step.name);
        if step.is_modified {
            print!(" [MODIFIED]");
        }
        if step.is_stale {
            print!(" [STALE]");
        }
        println!();
        println!("    Expected Cut: {}", step.expected_cut);
        if let Some(ref act) = step.actual_oid {
            println!("    Actual OID:   {}", act);
        } else {
            println!("    Actual OID:   [MISSING BRANCH]");
        }
        if let Some(ref b) = meta_step.branch {
            println!("    Branch:       {}", b);
        }
    }
}

fn print_family(f: &StaircaseFamily) {
    println!("  Name: {}", f.name);
    println!("  ID: {}", f.id);
    println!("  Target: {}", f.target);
    println!("  Roots: {}", f.roots.join(", "));
    println!("  Steps:");
    for (name, step) in &f.steps {
        println!("    Step {}:", name);
        println!("      Cut: {}", step.cut);
        if let Some(ref b) = step.branch {
            println!("      Branch: {}", b);
        }
        if !step.children.is_empty() {
            println!("      Children: {}", step.children.join(", "));
        }
    }
}
