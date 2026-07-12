use super::{PresentationOutput, ReorderResult, StaircaseSelectorArgs};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Reorder {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    /// New order of steps by 1-based index.
    #[arg(long, value_delimiter = ',')]
    pub order: Option<Vec<usize>>,
}

impl super::Command for Reorder {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(repo, self.staircase.clone(), self.order.clone())?;
        Ok(Box::new(result))
    }
}

pub fn run_internal(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    order: Option<Vec<usize>>,
) -> Result<ReorderResult> {
    let rs = staircase.resolve(repo)?;
    let rs = &rs;
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

pub fn run(
    repo: &GitRepo,
    staircase: StaircaseSelectorArgs,
    order: Option<Vec<usize>>,
) -> Result<ReorderResult> {
    run_internal(repo, staircase, order)
}
