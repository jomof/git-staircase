use super::{Presentation, PresentationOutput, Summary, ToPresentation};
use crate::GitRepo;
use crate::core;
use crate::core::{ListFilter, ResolvedStaircase};
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;

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
    #[arg(long)]
    pub include_archived_materializations: bool,
    #[arg(long)]
    pub diagnostics: bool,
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
            include_archived_materializations: self.include_archived_materializations,
            diagnostics: self.diagnostics,
            onto: self.onto.clone(),
        };

        let resolved_staircases = core::list(repo, filter)?;
        let mut all_results = Vec::new();

        let cached_draft = core::draft::get_worktree_draft(repo).ok();

        // 1. First pass: Collect all summaries and count names
        let mut name_counts = HashMap::new();
        let mut entries = Vec::new();

        for rs in resolved_staircases {
            let entry = match rs {
                ResolvedStaircase::ImplicitFamily(f) => {
                    let name = f.name.clone();
                    *name_counts.entry(name).or_insert(0) += 1;
                    ListEntry::Family(Summary::new(f))
                }
                _ => {
                    let m = rs.metadata();
                    let name = m.name.clone();
                    *name_counts.entry(name).or_insert(0) += 1;
                    let status = core::status::get_status_metadata_ext(
                        repo,
                        m.clone(),
                        !rs.is_managed(),
                        None,
                        Some(cached_draft.clone()),
                    )?;
                    ListEntry::Staircase(Summary::new(status))
                }
            };
            entries.push(entry);
        }

        // 2. Second pass: Qualify ambiguous names
        for mut entry in entries {
            match &mut entry {
                ListEntry::Staircase(s) => {
                    if *name_counts.get(&s.value.metadata.name).unwrap_or(&0) > 1 {
                        let id = &s.value.metadata.id;
                        let qualification = if s.value.is_implicit {
                            format!(
                                "implicit@{}",
                                &id[id.find('@').map(|i| i + 1).unwrap_or(0)
                                    ..id.find('@').map(|i| i + 1).unwrap_or(0) + 7]
                            )
                        } else {
                            format!("lineage@{}", &id[..7])
                        };
                        s.qualification = Some(qualification);
                    }
                }
                ListEntry::Family(f) => {
                    if *name_counts.get(&f.value.name).unwrap_or(&0) > 1 {
                        f.qualification = Some(format!("family@{}", &f.value.id[..7]));
                    }
                }
            }
            all_results.push(entry);
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
