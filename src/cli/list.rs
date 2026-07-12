use super::Summary;
use super::formatting::{ToHuman, ToPorcelain};
use crate::GitRepo;
use crate::core;
use crate::core::persistence;
use crate::{Discovery, ResolvedStaircase};
use serde::Serialize;

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

pub fn run(
    repo: &GitRepo,
    managed: bool,
    implicit: bool,
    discovered: bool,
    families: bool,
    onto: Option<String>,
) -> anyhow::Result<Vec<ListEntry>> {
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
        let list = core::discover(repo, onto.as_deref())?;
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

impl ToHuman for Vec<ListEntry> {
    fn to_human(&self) -> String {
        self.iter().map(|x| x.to_human()).collect::<Vec<_>>().join("\n")
    }
}

impl ToPorcelain for Vec<ListEntry> {
    fn to_porcelain(&self) -> String {
        self.iter().map(|x| x.to_porcelain()).collect::<Vec<_>>().join("\n")
    }
}
