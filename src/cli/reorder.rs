use super::{OutputFormat, StaircaseSelectorArgs};
use crate::GitRepo;
use anyhow::anyhow;
use git_staircase::core;

pub fn run(
    repo: &GitRepo,
    format: OutputFormat,
    staircase: StaircaseSelectorArgs,
    order: Option<Vec<usize>>,
) -> anyhow::Result<()> {
    let onto = staircase.onto.clone();
    let rs = super::resolve_rs(repo, &staircase)?;
    let order = order.ok_or_else(|| anyhow!("--order (indices) must be provided"))?;
    let mut zero_based_steps = Vec::new();
    for &s in &order {
        if s == 0 {
            return Err(anyhow!("Step indices must be 1-based (got 0)"));
        }
        zero_based_steps.push(s - 1);
    }
    core::reorder(repo, &rs, &zero_based_steps)?;

    match format {
        OutputFormat::Json => {
            let updated_rs = core::resolve_staircase(repo, &rs.metadata().name, onto.as_deref())?
                .ok_or_else(|| {
                anyhow!("Staircase '{}' not found after reorder", rs.metadata().name)
            })?;
            let status = core::get_status_metadata(
                repo,
                updated_rs.metadata().clone(),
                !updated_rs.is_managed(),
            )?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        OutputFormat::Human => {
            println!("Reordered staircase.");
            Ok(())
        }
        OutputFormat::Porcelain => Ok(()),
    }
}
