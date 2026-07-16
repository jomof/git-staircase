use crate::GitRepo;
use crate::core::{self, ResolvedStaircase, persistence};
use crate::error::Result;
use crate::model::{Discovery, StaircaseFamily, StaircaseStatus};
use std::collections::BTreeMap;

#[derive(Default, Clone, Debug)]
pub struct ListFilter {
    pub managed: bool,
    pub discovered: bool,
    pub families: bool,
    pub implicit: bool,
    pub stale: bool,
    pub archived: bool,
    pub all: bool,
    pub onto: Option<String>,
}

pub fn list(repo: &GitRepo, filter: ListFilter) -> Result<Vec<ResolvedStaircase>> {
    list_internal(repo, filter, false).map(|(rs, _)| rs)
}

pub enum ListEntry {
    Staircase(StaircaseStatus),
    Family(StaircaseFamily),
}

pub fn list_with_status(repo: &GitRepo, filter: ListFilter) -> Result<Vec<ListEntry>> {
    let (resolved_staircases, discovered_items) = list_internal(repo, filter.clone(), true)?;
    let mut results = Vec::new();

    let cached_draft = core::draft::get_worktree_draft(repo).ok();

    for rs in resolved_staircases {
        match rs {
            ResolvedStaircase::ImplicitFamily(f) => {
                results.push(ListEntry::Family(f));
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
                results.push(ListEntry::Staircase(status));
            }
        }
    }

    Ok(results)
}

fn list_internal(
    repo: &GitRepo,
    filter: ListFilter,
    include_discovered: bool,
) -> Result<(Vec<ResolvedStaircase>, Vec<Discovery>)> {
    let show_implicit = filter.implicit || filter.discovered;
    let show_all = !filter.all
        && !filter.managed
        && !show_implicit
        && !filter.families
        && !filter.stale
        && !filter.archived;
    let all = filter.all;

    let mut resolved_staircases = Vec::new();

    if filter.archived {
        let list = persistence::list_archived_staircases(repo)?;
        for s in list {
            resolved_staircases.push(ResolvedStaircase::Managed(s));
        }
    } else if all {
        let list = persistence::list_all_staircases(repo)?;
        for s in list {
            resolved_staircases.push(ResolvedStaircase::Managed(s));
        }
    } else if filter.managed || filter.stale || show_all {
        let list = persistence::list_staircases(repo)?;
        for s in list {
            resolved_staircases.push(ResolvedStaircase::Managed(s));
        }
    }

    let mut discovered_items = Vec::new();

    if show_implicit || filter.families || show_all || include_discovered {
        match core::discover(repo, filter.onto.as_deref(), None, filter.families) {
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
                            if filter.families || show_all {
                                resolved_staircases
                                    .push(ResolvedStaircase::ImplicitFamily(f.clone()));
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Ignore discovery errors for listing, consistent with previous behavior
            }
        }
    }

    let mut canonical = BTreeMap::<String, ResolvedStaircase>::new();
    for staircase in resolved_staircases {
        let key = match &staircase {
            ResolvedStaircase::Managed(metadata) => {
                let integration = repo.resolve_commit(&metadata.target)?;
                core::discovery::compute_implicit_id(repo, &integration, &metadata.steps)?
            }
            ResolvedStaircase::Implicit(metadata) => metadata.id.clone(),
            ResolvedStaircase::ImplicitFamily(family) => format!("family:{}", family.id),
        };
        match canonical.get(&key) {
            Some(ResolvedStaircase::Managed(_)) => {}
            Some(_) if staircase.is_managed() => {
                canonical.insert(key, staircase);
            }
            None => {
                canonical.insert(key, staircase);
            }
            Some(_) => {}
        }
    }

    let mut final_results = Vec::new();
    let cached_draft = if filter.stale {
        core::draft::get_worktree_draft(repo).ok()
    } else {
        None
    };

    for rs in canonical.into_values() {
        if filter.stale {
            match rs {
                ResolvedStaircase::ImplicitFamily(_) => {
                    // Families are not "stale" in the same way, usually skipped in CLI too
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
                    if matches!(status.state(), crate::model::StaircaseState::Stale) {
                        final_results.push(rs);
                    }
                }
            }
        } else {
            final_results.push(rs);
        }
    }

    Ok((final_results, discovered_items))
}
