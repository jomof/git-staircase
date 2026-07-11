use anyhow::{Context, anyhow};
use clap::{Parser, Subcommand};
use git_staircase::core;
use git_staircase::{
    Discovery, GitRepo, IdentityKind, StaircaseFamily, StaircaseMetadata, Step, ToPorcelain,
    VerificationPolicy,
};
use std::path::PathBuf;

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
        name: String,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Show status of a staircase (clean/stale/modified)
    Status {
        name: String,
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
        name: String,
        #[arg(long)]
        onto: String,
        #[arg(long)]
        resolve_onto: Option<String>,
    },
    /// Restack stale steps
    Restack {
        name: String,
        #[arg(long)]
        onto: Option<String>,
    },
    /// Verify a staircase
    Verify {
        name: String,
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
        name: String,
        #[arg(long)]
        onto: Option<String>,
        #[arg(long, value_enum, default_value = "lineage")]
        kind: IdentityKind,
    },
    /// Delete a managed staircase
    Delete {
        name: String,
        #[arg(long)]
        onto: Option<String>,
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
            let discovered = core::discover(&repo, onto.as_deref())?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&discovered)?);
            } else if cli.porcelain {
                for d in &discovered {
                    println!("{}", d.to_porcelain());
                }
            } else {
                if discovered.is_empty() {
                    println!("No potential staircases discovered.");
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
        }
        Commands::Adopt {
            name,
            onto,
            branches,
            build_command,
            test_command,
            verify_each_prefix,
        } => {
            if branches.is_empty() {
                return Err(anyhow!("At least one branch must be specified to adopt"));
            }
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

            let verification_policy = if build_command.is_some() || test_command.is_some() {
                Some(VerificationPolicy {
                    build_command,
                    test_command,
                    verify_each_prefix,
                })
            } else {
                None
            };

            let target = match onto {
                Some(o) => o,
                None => core::infer_onto(&repo)?,
            };
            let staircase = StaircaseMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                target,
                steps,
                verification_policy,
            };

            core::adopt(&repo, &staircase)?;
            if !cli.json && !cli.porcelain {
                println!("Adopted staircase '{}' (ID: {}).", name, staircase.id);
            } else if cli.json {
                println!("{}", serde_json::to_string_pretty(&staircase)?);
            } else if cli.porcelain {
                println!("{}", staircase.to_porcelain());
            }
        }
        Commands::List {
            managed,
            implicit,
            onto,
        } => {
            let show_all = !managed && !implicit;
            let mut all_results = Vec::new();

            if managed || show_all {
                let list = repo.list_staircases()?;
                for s in list {
                    all_results.push(git_staircase::ResolvedStaircase::Managed(s));
                }
            }

            if implicit || show_all {
                let list = core::discover(&repo, onto.as_deref())?;
                for d in list {
                    if let Discovery::Linear(s) = d {
                        all_results.push(git_staircase::ResolvedStaircase::Implicit(s));
                    }
                }
            }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&all_results)?);
            } else if cli.porcelain {
                for r in &all_results {
                    println!("{}", r.to_porcelain());
                }
            } else {
                if all_results.is_empty() {
                    println!("No staircases found.");
                } else {
                    for r in all_results {
                        let m = r.metadata().clone();
                        let status = core::get_status_metadata(&repo, m.clone())?;
                        let state = if status.steps.iter().any(|s| s.is_stale) {
                            "stale"
                        } else {
                            "clean"
                        };
                        let steps_count = m.steps.len();
                        let steps_word = if steps_count == 1 { "step" } else { "steps" };
                        let implicit_marker = if r.is_managed() { "" } else { " (implicit)" };
                        print!("{} {} {} {}", m.name, steps_count, steps_word, state);
                        if !implicit_marker.is_empty() {
                            println!("{}", implicit_marker);
                        } else {
                            println!();
                        }
                    }
                }
            }
        }
        Commands::Show { name, onto } => {
            let rs = core::resolve_staircase(&repo, &name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&rs)?);
            } else if cli.porcelain {
                println!("{}", rs.to_porcelain());
            } else {
                print_resolved_staircase(&rs);
            }
        }
        Commands::Status { name, onto } => {
            let rs = core::resolve_staircase(&repo, &name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            let status = core::get_status_metadata(&repo, rs.metadata().clone())?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else if cli.porcelain {
                println!("{}", status.to_porcelain());
            } else {
                if !rs.is_managed() {
                    println!("(Implicit staircase)");
                }
                print_status(&status);
            }
        }
        Commands::Split {
            step,
            at,
            name,
            onto,
        } => {
            let (sc_name, step_num) = parse_step_spec(&step)?;
            let rs = core::resolve_staircase(&repo, &sc_name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name))?;

            if step_num == 0 {
                return Err(anyhow!("Step number must be 1-based"));
            }
            core::split(&repo, &rs, step_num - 1, &at, name.as_deref())?;
            if !cli.json && !cli.porcelain {
                println!(
                    "Split step {} of staircase '{}' at {}.",
                    step_num, sc_name, at
                );
            }
        }
        Commands::Join { step1, step2, onto } => {
            let (sc_name1, step_num1) = parse_step_spec(&step1)?;
            let (sc_name2, step_num2) = parse_step_spec(&step2)?;

            if sc_name1 != sc_name2 {
                return Err(anyhow!(
                    "Cannot join steps from different staircases: '{}' and '{}'",
                    sc_name1,
                    sc_name2
                ));
            }

            let rs = core::resolve_staircase(&repo, &sc_name1, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", sc_name1))?;

            if step_num1 == 0 || step_num2 == 0 {
                return Err(anyhow!("Step numbers must be 1-based"));
            }

            core::join(&repo, &rs, step_num1 - 1, step_num2 - 1)?;
            if !cli.json && !cli.porcelain {
                println!(
                    "Joined steps {} and {} of staircase '{}'.",
                    step_num1, step_num2, sc_name1
                );
            }
        }
        Commands::Rebase {
            name,
            onto,
            resolve_onto,
        } => {
            let rs = core::resolve_staircase(&repo, &name, resolve_onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::rebase(&repo, &rs, &onto)?;
            if !cli.json && !cli.porcelain {
                println!("Rebased staircase '{}' onto '{}'.", name, onto);
            }
        }
        Commands::Restack { name, onto } => {
            let rs = core::resolve_staircase(&repo, &name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::restack(&repo, &rs)?;
            if !cli.json && !cli.porcelain {
                println!("Restacked staircase '{}'.", name);
            }
        }
        Commands::Id { name, kind, onto } => {
            let rs = core::resolve_staircase(&repo, &name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            let id = core::compute_identity(&repo, &rs, kind)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({"id": id}))?
                );
            } else {
                println!("{}", id);
            }
        }
        Commands::Verify {
            name,
            onto,
            aggregate,
            each_prefix,
            build_command,
            test_command,
        } => {
            let aggregate_opt = if aggregate { Some(true) } else { None };
            let each_prefix_opt = if each_prefix { Some(true) } else { None };
            let results = core::verify(
                onto.as_deref(),
                &repo,
                &name,
                build_command,
                test_command,
                aggregate_opt,
                each_prefix_opt,
            )?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else if cli.porcelain {
                for r in &results {
                    println!("{}", r.to_porcelain());
                }
            } else {
                for result in results {
                    println!(
                        "Step {}: {}",
                        result.step_name,
                        if result.success { "PASSED" } else { "FAILED" }
                    );
                    if !result.success {
                        println!("Stdout:\n{}", result.stdout);
                        println!("Stderr:\n{}", result.stderr);
                    }
                }
            }
        }
        Commands::Delete {
            name,
            onto,
            delete_branches,
        } => {
            let rs = core::resolve_staircase(&repo, &name, onto.as_deref())?
                .ok_or_else(|| anyhow!("Staircase '{}' not found", name))?;
            core::delete(&repo, &rs.metadata().id, delete_branches)?;
            if !cli.json && !cli.porcelain {
                println!("Deleted staircase '{}'.", name);
            }
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
    if let Some(ref policy) = s.verification_policy {
        println!("  Verification Policy:");
        if let Some(ref cmd) = policy.build_command {
            println!("    Build: {}", cmd);
        }
        if let Some(ref cmd) = policy.test_command {
            println!("    Test:  {}", cmd);
        }
        println!("    Verify each prefix: {}", policy.verify_each_prefix);
    }
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

fn print_resolved_staircase(rs: &git_staircase::ResolvedStaircase) {
    let s = rs.metadata();
    if rs.is_managed() {
        println!("Managed Staircase: {}", s.name);
    } else {
        println!("Implicit Staircase: {}", s.name);
    }
    print_staircase(s);
}
