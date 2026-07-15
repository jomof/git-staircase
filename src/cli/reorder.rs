use super::{NonStepsStaircaseSelectorArgs, PresentationOutput, ReorderResult};
use crate::GitRepo;
use crate::core;
use anyhow::{Result, anyhow};

#[derive(clap::Args, Clone, Debug)]
pub struct Reorder {
    #[command(flatten)]
    pub staircase: NonStepsStaircaseSelectorArgs,
    /// Complete new order by 1-based ordinal or stable step ID.
    #[arg(long, value_delimiter = ',', required = true)]
    pub steps: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
}

impl super::Command for Reorder {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let mut zero_based_steps = Vec::new();
        for value in &self.steps {
            if let Ok(ordinal) = value.parse::<usize>() {
                if ordinal == 0 {
                    return Err(anyhow!("Step indices must be 1-based (got 0)"));
                }
                zero_based_steps.push(ordinal - 1);
            } else {
                let index = rs
                    .metadata()
                    .steps
                    .iter()
                    .position(|step| step.id == *value)
                    .ok_or_else(|| anyhow!("unknown step ID '{}'", value))?;
                zero_based_steps.push(index);
            }
        }
        if self.dry_run {
            core::reorder_dry_run(
                repo,
                &rs,
                &zero_based_steps,
                core::ReorderOptions { no_restack: false },
            )?;
            return Ok(Box::new(super::Success::new(format!(
                "Planned reorder of staircase '{}'",
                rs.metadata().name
            ))));
        } else {
            core::reorder(
                repo,
                &rs,
                &zero_based_steps,
                core::ReorderOptions { no_restack: false },
            )?;
        }

        let updated_rs = core::resolve_staircase(
            repo,
            &rs.metadata().name,
            self.staircase.base.onto.as_deref(),
        )?
        .ok_or_else(|| anyhow!("Staircase '{}' not found after reorder", rs.metadata().name))?;
        let status = core::get_status_metadata(
            repo,
            updated_rs.metadata().clone(),
            !updated_rs.is_managed(),
        )?;

        Ok(Box::new(ReorderResult { status }))
    }
}
