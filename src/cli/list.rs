use super::{Presentation, PresentationOutput, Summary, ToPresentation};
use crate::GitRepo;
use crate::core;
use crate::core::{ListFilter, ResolvedStaircase};
use anyhow::Result;
use serde::Serialize;

#[derive(clap::Args, Clone, Debug)]
pub struct List {
    #[arg(long)]
    pub managed: bool,
    #[arg(long)]
    pub discovered: bool,
    #[arg(long, short)]
    pub families: bool,
    #[arg(long)]
    pub implicit: bool,
    #[arg(long)]
    pub stale: bool,
    #[arg(long)]
    pub archived: bool,
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub onto: Option<String>,
    #[arg(long)]
    pub strict: bool,
}

impl super::Command for List {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let filter = ListFilter {
            managed: self.managed,
            discovered: self.discovered,
            families: self.families,
            implicit: self.implicit,
            stale: self.stale,
            archived: self.archived,
            all: self.all,
            onto: self.onto.clone(),
        };

        let resolved_staircases = core::list(repo, filter)?;
        let mut all_results = Vec::new();

        // Note: strict mode detection of unresolved implicit candidates is currently simplified in core::list
        // and doesn't explicitly return errors unless they are fatal.

        let cached_draft = core::draft::get_worktree_draft(repo).ok();
        // We re-run status if needed for presentation, but core::list already filtered if self.stale was true.
        // Actually, we need status for the Summary presentation anyway.

        for rs in resolved_staircases {
            match rs {
                ResolvedStaircase::ImplicitFamily(f) => {
                    all_results.push(ListEntry::Family(Summary(f)));
                }
                _ => {
                    let m = rs.metadata();
                    let status = core::status::get_status_metadata_ext(
                        repo,
                        m.clone(),
                        !rs.is_managed(),
                        None, // core::list already used discoveries if needed
                        Some(cached_draft.clone()),
                    )?;
                    all_results.push(ListEntry::Staircase(Summary(status)));
                }
            }
        }

        Ok(Box::new(ListResult(all_results)))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ListResult(pub Vec<ListEntry>);

impl ToPresentation for ListResult {
    fn to_presentation(&self) -> Presentation {
        if self.0.is_empty() {
            Presentation::List(vec![
                Presentation::Human(Box::new(Presentation::Plain("No staircases.".to_string()))),
                Presentation::Porcelain(Box::new(Presentation::Empty)),
            ])
        } else {
            self.0.to_presentation()
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ListEntry {
    Staircase(Summary<crate::model::StaircaseStatus>),
    Family(Summary<crate::model::StaircaseFamily>),
}

impl ToPresentation for ListEntry {
    fn to_presentation(&self) -> Presentation {
        match self {
            ListEntry::Staircase(s) => s.to_presentation(),
            ListEntry::Family(f) => f.to_presentation(),
        }
    }
}
