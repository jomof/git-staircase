use super::OutputFormat;
use crate::GitRepo;
use git_staircase::IdentityKind;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    name: Option<String>,
    steps: Option<Vec<String>>,
    kind: IdentityKind,
    onto: Option<String>,
) -> anyhow::Result<()> {
    let rs = super::resolve_rs(repo, name, steps, onto)?;
    let was_implicit = !rs.is_managed();
    let id = core::compute_identity(repo, &rs, kind)?;
    if was_implicit && kind == IdentityKind::Lineage && matches!(format, OutputFormat::Human) {
        println!("adopted implicit staircase '{}'", rs.metadata().name);
    }
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({"id": id}))?
            );
        }
        _ => {
            println!("{}", id);
        }
    }
    Ok(())
}
