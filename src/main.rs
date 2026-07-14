use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand, error::ErrorKind};
use git_staircase::{GitRepo, StaircaseError};
use std::path::PathBuf;

use git_staircase::cli::{self, Command, OutputFormat};
use git_staircase::workspace::{BootstrapOptions, bootstrap};

#[derive(Parser)]
#[command(name = "git-staircase")]
#[command(about = "Manage git staircases", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    format: FormatArgs,

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

#[derive(Args, Debug, Clone, Copy)]
struct FormatArgs {
    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    porcelain: bool,

    #[arg(long, global = true)]
    format: Option<OutputFormat>,
}

impl FormatArgs {
    fn determine_format(&self) -> OutputFormat {
        if self.json || matches!(self.format, Some(OutputFormat::Json)) {
            OutputFormat::Json
        } else if self.porcelain || matches!(self.format, Some(OutputFormat::Porcelain)) {
            OutputFormat::Porcelain
        } else {
            OutputFormat::Human
        }
    }

    /// Detect requested format from raw command line arguments.
    /// Used as a fallback when full argument parsing fails.
    fn detect_from_args() -> OutputFormat {
        let args: Vec<String> = std::env::args().collect();
        let is_json = args.iter().any(|arg| arg == "--json")
            || args
                .windows(2)
                .any(|pair| pair[0] == "--format" && pair[1] == "json")
            || args.iter().any(|arg| arg == "--format=json");
        let is_porcelain = args.iter().any(|arg| arg == "--porcelain")
            || args
                .windows(2)
                .any(|pair| pair[0] == "--format" && pair[1] == "porcelain")
            || args.iter().any(|arg| arg == "--format=porcelain");

        if is_json {
            OutputFormat::Json
        } else if is_porcelain {
            OutputFormat::Porcelain
        } else {
            OutputFormat::Human
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Workspace configuration and provider management
    Workspace(cli::workspace::WorkspaceCmd),
    /// Provider-specific offline diagnostics
    Provider(cli::provider::ProviderCmd),
    /// Land a staircase into its target branch
    Land(cli::land::Land),
    /// Append committed history to a staircase
    Append(cli::append::Append),
    /// Reorder steps of a staircase
    Reorder(cli::reorder::Reorder),
    /// Move commits between steps
    Move(cli::move_cmd::Move),
    /// Drop a step from a staircase
    Drop(cli::drop::Drop),
    /// Discover potential staircases
    Discover(cli::discover::Discover),
    /// Manage persistent discovery overrides
    Discovery(cli::discovery::DiscoveryCmd),
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
    /// Manage persistent typed policy
    Policy(cli::policy::PolicyCmd),
    /// Inspect or repair primary branch layout
    Layout(cli::layout::LayoutCmd),
    /// Repair derived local representation
    Normalize(cli::normalize::Normalize),
    /// Continue the active Staircase operation
    Continue(cli::operation::Continue),
    /// Abort the active Staircase operation
    Abort(cli::operation::Abort),
    /// Inspect the active Staircase operation
    Operation(cli::operation::OperationCmd),
    /// Assign a public name to a staircase
    Name(cli::naming::Name),
    /// Rename a staircase public name
    Rename(cli::naming::Rename),
    /// Remove a staircase public name
    Unname(cli::naming::Unname),
    /// Create an immutable Staircase snapshot tag
    Tag(cli::tag::Tag),
    /// Resolve a selector to a typed canonical identity
    RevParse(cli::rev_parse::RevParse),
    /// Transport Staircase records and refs
    Push(cli::transport::Push),
    /// Fetch Staircase records and refs
    Fetch(cli::transport::Fetch),
    /// Archive a staircase
    Archive(cli::archive::ArchiveCmd),
    /// Unarchive a staircase
    Unarchive(cli::unarchive::UnarchiveCmd),
}

impl Commands {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn cli::PresentationOutput>> {
        match self {
            Commands::Workspace(cmd) => cmd.run(repo),
            Commands::Provider(cmd) => cmd.run(repo),
            Commands::Land(cmd) => cmd.run(repo),
            Commands::Append(cmd) => cmd.run(repo),
            Commands::Reorder(cmd) => cmd.run(repo),
            Commands::Move(cmd) => cmd.run(repo),
            Commands::Drop(cmd) => cmd.run(repo),
            Commands::Discover(cmd) => cmd.run(repo),
            Commands::Discovery(cmd) => cmd.run(repo),
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
            Commands::Policy(cmd) => cmd.run(repo),
            Commands::Layout(cmd) => cmd.run(repo),
            Commands::Normalize(cmd) => cmd.run(repo),
            Commands::Continue(cmd) => cmd.run(repo),
            Commands::Abort(cmd) => cmd.run(repo),
            Commands::Operation(cmd) => cmd.run(repo),
            Commands::Name(cmd) => cmd.run(repo),
            Commands::Rename(cmd) => cmd.run(repo),
            Commands::Unname(cmd) => cmd.run(repo),
            Commands::Tag(cmd) => cmd.run(repo),
            Commands::RevParse(cmd) => cmd.run(repo),
            Commands::Push(cmd) => cmd.run(repo),
            Commands::Fetch(cmd) => cmd.run(repo),
            Commands::Archive(cmd) => cmd.run(repo),
            Commands::Unarchive(cmd) => cmd.run(repo),
        }
    }

    fn requires_clear_operation(&self) -> bool {
        !matches!(
            self,
            Commands::Workspace(_)
                | Commands::Provider(_)
                | Commands::Discover(_)
                | Commands::List(_)
                | Commands::Show(_)
                | Commands::Status(_)
                | Commands::Id(_)
                | Commands::Log(_)
                | Commands::Diff(_)
                | Commands::Graph(_)
                | Commands::Steps(_)
                | Commands::Commits(_)
                | Commands::RevParse(_)
                | Commands::Operation(_)
                | Commands::Continue(_)
                | Commands::Abort(_)
        )
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

fn run(cli: Cli) -> Result<()> {
    let repo_root = find_repo_root()?;
    let repo = GitRepo::new(repo_root);
    if cli.command.requires_clear_operation() {
        git_staircase::core::ensure_no_active(&repo)?;
        if let Some((operation, owner)) = git_staircase::core::external_git_operation(&repo)? {
            return Err(StaircaseError::ExternalOperation { operation, owner }.into());
        }
    }
    let format = cli.format.determine_format();

    let options = BootstrapOptions {
        no_bootstrap: cli.no_bootstrap,
        no_configure: cli.no_configure,
        workspace_id: cli.workspace,
        workspace_provider: cli.workspace_provider,
        review_provider: cli.review_provider,
        provider_profile: cli.provider_profile,
        workspace_mode: cli.workspace_mode,
        is_porcelain_or_json: !matches!(format, OutputFormat::Human),
    };

    if !matches!(&cli.command, Commands::Provider(_)) {
        let bootstrap_res = bootstrap(&repo, &options)?;
        if let Some(ref msg) = bootstrap_res.message {
            if matches!(format, OutputFormat::Human) {
                eprintln!("{}", msg);
            }
        }
    }

    cli::dispatch(format, &repo, cli.command.run(&repo))
}

fn escape_machine_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn render_error(error: &anyhow::Error, format: OutputFormat) -> i32 {
    let typed = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<StaircaseError>());
    let code = typed.map_or("validation-failed", StaircaseError::code);
    let status = typed.map_or(1, |error| error.exit_class().status());
    let message = typed.map_or_else(|| error.to_string(), ToString::to_string);
    let details = typed.map_or(serde_json::Value::Null, StaircaseError::details);

    match format {
        OutputFormat::Json => {
            let diagnostic = serde_json::json!({
                "error": {
                    "code": code,
                    "message": message,
                    "exit_status": status,
                    "details": details,
                }
            });
            eprintln!(
                "{}",
                serde_json::to_string(&diagnostic)
                    .unwrap_or_else(|_| r#"{"error":{"code":"serialization-error"}}"#.into())
            );
        }
        OutputFormat::Porcelain => {
            eprintln!("error\t{}\t{}", code, escape_machine_field(&message));
        }
        OutputFormat::Human => {
            eprintln!("error [{}]: {}", code, message);
            if !details.is_null() {
                if let Ok(rendered) = serde_json::to_string_pretty(&details) {
                    eprintln!("{}", rendered);
                }
            }
        }
    }
    status
}

fn main() {
    let cli_res = Cli::try_parse();
    let format = match &cli_res {
        Ok(cli) => cli.format.determine_format(),
        Err(_) => FormatArgs::detect_from_args(),
    };

    match cli_res {
        Ok(cli) => {
            if let Err(error) = run(cli) {
                std::process::exit(render_error(&error, format));
            }
        }
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return;
            }
            let message = error.to_string();
            let error = anyhow!(message);
            std::process::exit(render_error(&error, format));
        }
    }
}
