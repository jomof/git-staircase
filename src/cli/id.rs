use super::formatting::{ToHuman, ToPorcelain};
use super::{PresentationOutput, StaircaseSelectorArgs};
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

impl ToHuman for IdResult {
    fn to_human(&self) -> String {
        let mut out = String::new();
        if self.was_implicit && self.kind == IdentityKind::Lineage {
            out.push_str(&format!("adopted implicit staircase '{}'\n", self.name));
        }
        out.push_str(&self.id);
        out
    }
}

impl ToPorcelain for IdResult {
    fn to_porcelain(&self) -> String {
        self.id.clone()
    }
}
