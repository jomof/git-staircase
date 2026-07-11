use super::{ReorderResult, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use anyhow::anyhow;

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    order: Option<Vec<usize>>,
) -> anyhow::Result<ReorderResult> {
    let rs = staircase.resolve(repo)?;
    let order = order.ok_or_else(|| anyhow!("--order (indices) must be provided"))?;
    let mut zero_based_steps = Vec::new();
    for &s in &order {
        if s == 0 {
            return Err(anyhow!("Step indices must be 1-based (got 0)"));
        }
        zero_based_steps.push(s - 1);
    }
    core::reorder(repo, &rs, &zero_based_steps)?;

    let updated_rs = core::resolve_staircase(repo, &rs.metadata().name, staircase.onto.as_deref())?
        .ok_or_else(|| anyhow!("Staircase '{}' not found after reorder", rs.metadata().name))?;
    let status = core::get_status_metadata(
        repo,
        updated_rs.metadata().clone(),
        !updated_rs.is_managed(),
    )?;

    Ok(ReorderResult { status })
}
