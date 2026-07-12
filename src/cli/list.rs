use super::formatting::{ToHuman, ToPorcelain};
use super::{PresentationOutput, Summary};
use crate::GitRepo;
use crate::core;
use crate::core::persistence;
use crate::{Discovery, ResolvedStaircase};
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
    pub onto: Option<String>,
}

impl super::Command for List {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let result = run_internal(
            repo,
            self.managed,
            self.implicit,
            self.discovered,
            self.families,
            self.onto.clone(),
        )?;
        Ok(Box::new(ListResult(result)))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ListResult(pub Vec<ListEntry>);

impl ToHuman for ListResult {
    fn to_human(&self) -> String {
        self.0.to_human()
    }
}

impl ToPorcelain for ListResult {
    fn to_porcelain(&self) -> String {
        self.0.to_porcelain()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ListEntry {
    Staircase(Summary<crate::model::StaircaseStatus>),
    Family(Summary<crate::model::StaircaseFamily>),
}

impl ToHuman for ListEntry {
    fn to_human(&self) -> String {
        match self {
            ListEntry::Staircase(s) => s.to_human(),
            ListEntry::Family(f) => f.to_human(),
        }
    }
}

impl ToPorcelain for ListEntry {
    fn to_porcelain(&self) -> String {
        match self {
            ListEntry::Staircase(s) => s.to_porcelain(),
            ListEntry::Family(f) => f.to_porcelain(),
        }
    }
}

pub fn run_internal(
    repo: &GitRepo,
    managed: bool,
    implicit: bool,
    discovered: bool,
    families: bool,
    onto: Option<String>,
) -> Result<Vec<ListEntry>> {
    let show_implicit = implicit || discovered;
    let show_all = !managed && !show_implicit && !families;
    let mut all_results = Vec::new();

    let mut resolved_staircases = Vec::new();

    if managed || show_all {
        let list = persistence::list_staircases(repo)?;
        for s in list {
            resolved_staircases.push(ResolvedStaircase::Managed(s));
        }
    }

    if show_implicit || families || show_all {
        let list = core::discover(repo, onto.as_deref(), None, families)?;
        for d in list {
            match d {
                Discovery::Linear(s) => {
                    if show_implicit || show_all {
                        resolved_staircases.push(ResolvedStaircase::Implicit(s));
                    }
                }
                Discovery::Ambiguous(f) => {
                    if families || show_all {
                        resolved_staircases.push(ResolvedStaircase::ImplicitFamily(f));
                    }
                }
            }
        }
    }

    for rs in resolved_staircases {
        match rs {
            ResolvedStaircase::ImplicitFamily(f) => {
                all_results.push(ListEntry::Family(Summary(f)));
            }
            _ => {
                let m = rs.metadata();
                let status = core::get_status_metadata(repo, m.clone(), !rs.is_managed())?;
                all_results.push(ListEntry::Staircase(Summary(status)));
            }
        }
    }

    Ok(all_results)
}

pub fn run(
    repo: &GitRepo,
    managed: bool,
    implicit: bool,
    discovered: bool,
    families: bool,
    onto: Option<String>,
) -> Result<Vec<ListEntry>> {
    run_internal(repo, managed, implicit, discovered, families, onto)
}
