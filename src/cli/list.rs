use super::formatting::{ToHuman, ToPorcelain};
use super::{PresentationOutput, Summary};
use crate::GitRepo;
use crate::core;
use crate::core::persistence;
use crate::{Discovery, ResolvedStaircase};
use anyhow::{Result, anyhow};
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
        let show_implicit = self.implicit || self.discovered;
        let show_all = !self.managed && !show_implicit && !self.families && !self.stale && !self.archived;
        let mut all_results = Vec::new();
        let mut unresolved_errors = Vec::new();

        let mut resolved_staircases = Vec::new();

        if self.archived {
            let list = persistence::list_archived_staircases(repo)?;
            for s in list {
                resolved_staircases.push(ResolvedStaircase::Managed(s));
            }
        } else if self.all {
            let list = persistence::list_all_staircases(repo)?;
            for s in list {
                resolved_staircases.push(ResolvedStaircase::Managed(s));
            }
        } else if self.managed || self.stale || show_all {
            let list = persistence::list_staircases(repo)?;
            for s in list {
                resolved_staircases.push(ResolvedStaircase::Managed(s));
            }
        }

        let mut discovered_items = Vec::new();

        if show_implicit || self.families || show_all {
            match core::discover(repo, self.onto.as_deref(), None, self.families) {
                Ok(list) => {
                    discovered_items = list;
                    for d in &discovered_items {
                        match d {
                            Discovery::Linear(s) => {
                                if show_implicit || show_all {
                                    resolved_staircases.push(ResolvedStaircase::Implicit(s.clone()));
                                }
                            }
                            Discovery::Ambiguous(f) => {
                                if self.families || show_all {
                                    resolved_staircases.push(ResolvedStaircase::ImplicitFamily(f.clone()));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    unresolved_errors.push(format!("Unresolved implicit candidates: {}", e));
                }
            }
        }

        if self.strict && !unresolved_errors.is_empty() {
            return Err(anyhow!(
                "Strict mode: unresolved candidates detected:\n{}",
                unresolved_errors.join("\n")
            ));
        }

        let cached_draft = core::draft::get_worktree_draft(repo).ok();

        for rs in resolved_staircases {
            match rs {
                ResolvedStaircase::ImplicitFamily(f) => {
                    if !self.stale {
                        all_results.push(ListEntry::Family(Summary(f)));
                    }
                }
                _ => {
                    let m = rs.metadata();
                    let status = core::status::get_status_metadata_ext(
                        repo,
                        m.clone(),
                        !rs.is_managed(),
                        Some(&discovered_items),
                        Some(cached_draft.clone()),
                    )?;
                    if self.stale {
                        if matches!(status.state(), crate::model::StaircaseState::Stale) {
                            all_results.push(ListEntry::Staircase(Summary(status)));
                        }
                    } else {
                        all_results.push(ListEntry::Staircase(Summary(status)));
                    }
                }
            }
        }

        Ok(Box::new(ListResult(all_results)))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ListResult(pub Vec<ListEntry>);

impl ToHuman for ListResult {
    fn to_human(&self) -> String {
        if self.0.is_empty() {
            "No staircases.".to_string()
        } else {
            self.0.to_human()
        }
    }
}

impl ToPorcelain for ListResult {
    fn to_porcelain(&self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            self.0.to_porcelain()
        }
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
