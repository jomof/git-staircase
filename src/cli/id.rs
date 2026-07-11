use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::IdentityKind;
use crate::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    kind: IdentityKind,
) -> anyhow::Result<()> {
    let rs = staircase.resolve(repo)?;
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
