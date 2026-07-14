use crate::GitRepo;
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs, ToPresentation, Presentation};
use crate::core;
use crate::core::refs::StaircaseRefs;
use crate::error::StaircaseError;
use anyhow::Result;
use clap::Args;
use serde::Serialize;

#[derive(Args, Clone, Debug)]
#[group(id = "projection", multiple = false)]
pub struct RevParse {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long, group = "projection")]
    pub r#ref: bool,
    #[arg(long, group = "projection")]
    pub lineage: bool,
    #[arg(long, group = "projection")]
    pub record: bool,
    #[arg(long, group = "projection")]
    pub structure: bool,
    #[arg(long, group = "projection")]
    pub top: bool,
    #[arg(long, group = "projection")]
    pub step: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RevParseResult {
    pub schema: String,
    pub version: u32,
    pub kind: String,
    pub value: String,
    pub lineage_id: Option<String>,
    pub record_oid: Option<String>,
    pub structure_oid: Option<String>,
    pub top_oid: String,
    pub step_id: Option<String>,
}

impl ToPresentation for RevParseResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(self.value.clone()))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "identity".into(),
                "1".into(),
                self.kind.clone(),
                self.value.clone(),
            ]))),
        ])
    }
}

impl Command for RevParse {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let selector = self.selector.resolve(repo)?;
        let metadata = selector.metadata();
        let top_oid = metadata
            .steps
            .last()
            .ok_or_else(|| StaircaseError::InvalidStructure("empty staircase".into()))?
            .cut
            .clone();
        let mut record_oid = None;
        let mut structure_oid = None;
        if selector.is_managed() {
            let reference =
                if metadata.lifecycle.as_ref().is_some_and(|lifecycle| {
                    lifecycle.state == crate::model::LifecycleState::Archived
                }) {
                    StaircaseRefs::archive_record(&metadata.id)
                } else {
                    StaircaseRefs::state_record(&metadata.id)
                };
            let record = core::read_record(repo, &reference)?;
            record_oid = Some(record.record_oid);
            structure_oid = Some(record.structure_oid);
        }
        let (kind, value, step_id) = if self.r#ref {
            let reference = StaircaseRefs::public(&metadata.name);
            if repo.resolve_ref_opt(&reference)?.is_some() {
                ("ref", reference, None)
            } else {
                let branch = metadata
                    .steps
                    .last()
                    .and_then(|step| step.branch.as_ref())
                    .ok_or_else(|| {
                        StaircaseError::Other("selected staircase has no canonical ref".into())
                    })?;
                (
                    "ref",
                    if branch.starts_with("refs/") {
                        branch.clone()
                    } else {
                        format!("refs/heads/{}", branch)
                    },
                    None,
                )
            }
        } else if self.lineage {
            if !selector.is_managed() {
                return Err(StaircaseError::Other(
                    "implicit staircases do not have lineage identity".into(),
                )
                .into());
            }
            ("lineage", metadata.id.clone(), None)
        } else if self.record {
            (
                "record",
                record_oid.clone().ok_or_else(|| {
                    StaircaseError::Other("implicit staircases do not have record revisions".into())
                })?,
                None,
            )
        } else if self.structure {
            (
                "structure",
                structure_oid.clone().unwrap_or_else(|| metadata.id.clone()),
                None,
            )
        } else if self.top {
            ("top", top_oid.clone(), None)
        } else if self.step {
            let index = selector
                .step_index
                .ok_or_else(|| StaircaseError::Other("--step requires a step selector".into()))?;
            let selected = metadata.steps.get(index).ok_or_else(|| {
                StaircaseError::InvalidStructure("selected step is out of range".into())
            })?;
            ("step", selected.id.clone(), Some(selected.id.clone()))
        } else if selector.is_managed() {
            let reference = StaircaseRefs::public(&metadata.name);
            if repo.resolve_ref_opt(&reference)?.is_some() {
                ("ref", reference, None)
            } else {
                ("lineage", metadata.id.clone(), None)
            }
        } else {
            ("structure", metadata.id.clone(), None)
        };
        Ok(Box::new(RevParseResult {
            schema: "git-staircase/rev-parse".into(),
            version: 1,
            kind: kind.into(),
            value,
            lineage_id: selector.is_managed().then(|| metadata.id.clone()),
            record_oid,
            structure_oid,
            top_oid,
            step_id,
        }))
    }
}
