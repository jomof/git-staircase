use super::{PresentationOutput, StaircaseSelectorArgs, ToPresentation, Presentation};
use crate::GitRepo;
use crate::IdentityKind;
use crate::core;
use anyhow::Result;
use serde::Serialize;

#[derive(clap::Args, Clone, Debug)]
pub struct Id {
    #[command(flatten)]
    pub staircase: StaircaseSelectorArgs,
    #[arg(long, value_enum, default_value = "lineage")]
    pub kind: IdentityKind,
}

impl super::Command for Id {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let rs = self.staircase.resolve(repo)?;
        let was_implicit = !rs.is_managed();
        let id = core::compute_identity(repo, &rs, self.kind)?;

        Ok(Box::new(IdResult {
            id,
            was_implicit,
            kind: self.kind,
            name: rs.metadata().name.clone(),
        }))
    }
}

#[derive(Serialize)]
pub struct IdResult {
    pub id: String,
    #[serde(skip)]
    pub was_implicit: bool,
    #[serde(skip)]
    pub kind: IdentityKind,
    #[serde(skip)]
    pub name: String,
}

impl ToPresentation for IdResult {
    fn to_presentation(&self) -> Presentation {
        let mut h_items = vec![];
        if self.was_implicit && self.kind == IdentityKind::Lineage {
            h_items.push(Presentation::Plain(format!("adopted implicit staircase '{}'", self.name)));
        }
        h_items.push(Presentation::Plain(self.id.clone()));

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::List(h_items))),
            Presentation::Porcelain(Box::new(Presentation::Plain(self.id.clone()))),
        ])
    }
}
