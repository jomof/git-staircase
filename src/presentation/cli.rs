use crate::cli::formatting::*;
use crate::model::*;
use crate::presentation::{Presentation, ToPresentation};
use serde_json::Value;
use std::collections::BTreeMap;

impl ToPresentation for Success {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.message.clone())
    }
}

impl ToPresentation for Summary<StaircaseStatus> {
    fn to_presentation(&self) -> Presentation {
        let s = &self.0;
        let m = &s.metadata;
        let steps_count = m.steps.len();
        let steps_word = if steps_count == 1 { "step" } else { "steps" };
        let implicit_marker = if s.is_implicit { " (implicit)" } else { "" };
        Presentation::pair(
            Presentation::Plain(format!(
                "{} [{}] {} {} {}{}",
                m.name,
                m.id,
                steps_count,
                steps_word,
                s.state(),
                implicit_marker
            )),
            Presentation::Record(vec![m.name.clone(), m.id.clone(), s.state().to_string()]),
        )
    }
}

impl ToPresentation for Summary<StaircaseFamily> {
    fn to_presentation(&self) -> Presentation {
        let f = &self.0;
        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
        let paths_word = if path_count == 1 { "path" } else { "paths" };
        Presentation::pair(
            Presentation::Plain(format!(
                "{} [{}] {} {} (implicit)",
                f.name, f.id, path_count, paths_word
            )),
            Presentation::Record(vec![
                f.name.clone(),
                f.id.clone(),
                "family".to_string(),
                f.steps.len().to_string(),
            ]),
        )
    }
}

impl ToPresentation for ReorderResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain("Reordered staircase.".to_string())
    }
}

impl ToPresentation for StepsList {
    fn to_presentation(&self) -> Presentation {
        let mut h_rows = vec![];
        let mut p_rows = vec![];
        for (i, step) in self.0.iter().enumerate() {
            h_rows.push(vec![
                format!("Step {}:", i + 1),
                step.name.clone(),
                format!("({})", &step.cut[..7]),
            ]);
            p_rows.push(vec![
                (i + 1).to_string(),
                step.id.clone(),
                step.name.clone(),
                step.cut.clone(),
            ]);
        }
        Presentation::pair(
            Presentation::Table {
                name: None,
                rows: h_rows,
            },
            Presentation::Table {
                name: None,
                rows: p_rows,
            },
        )
    }
}

impl ToPresentation for StaircaseCommits {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![];
        for step in &self.steps {
            let mut h_commits = vec![];
            let mut p_commits = vec![];
            for commit in &step.commits {
                h_commits.push(Presentation::Plain(format!(
                    "{} {}",
                    commit.hash, commit.subject
                )));
                p_commits.push(Presentation::Record(vec![
                    "commit".to_string(),
                    commit.hash.clone(),
                    commit.subject.clone(),
                ]));
            }
            children.push(Presentation::List(vec![
                Presentation::human(Presentation::Section {
                    title: format!("Step {}: {}", step.index, step.name),
                    children: h_commits,
                }),
                Presentation::porcelain(Presentation::Record(vec![
                    "step".to_string(),
                    step.index.to_string(),
                    step.name.clone(),
                ])),
                Presentation::porcelain(Presentation::List(p_commits)),
            ]));
        }
        Presentation::List(children)
    }
}

impl ToPresentation for PlainOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.0.clone())
    }
}

impl ToPresentation for LogOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_items = vec![];
        let mut p_items = vec![];
        for c in &self.0 {
            h_items.push(Presentation::Plain(format!("{} {}", c.hash, c.subject)));
            p_items.push(Presentation::Record(vec![
                c.hash.clone(),
                c.subject.clone(),
            ]));
        }
        Presentation::pair(Presentation::List(h_items), Presentation::List(p_items))
    }
}

impl ToPresentation for BTreeMap<String, Value> {
    fn to_presentation(&self) -> Presentation {
        let mut fields = vec![];
        for (k, v) in self {
            fields.push(Presentation::Field {
                label: k.clone(),
                value: v.to_string(),
            });
        }
        Presentation::Section {
            title: "Values:".into(),
            children: fields,
        }
    }
}

impl UsePresentation for Success {}
impl UsePresentation for Summary<StaircaseStatus> {}
impl UsePresentation for Summary<StaircaseFamily> {}
impl UsePresentation for ReorderResult {}
impl UsePresentation for StepsList {}
impl UsePresentation for StaircaseCommits {}
impl UsePresentation for PlainOutput {}
impl UsePresentation for LogOutput {}
impl UsePresentation for BTreeMap<String, Value> {}
use crate::presentation::UsePresentation;

use crate::cli::archive::{ArchiveOutput, ReleaseNameOutput};

impl ToPresentation for ArchiveOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];

        if self.result.archive_kind == "implicit-snapshot" {
            if let Some(ref aid) = self.result.archive_id {
                h_children.push(Presentation::Plain(format!("  archive ID: {}", aid)));
            }
            if let Some(ref key) = self.result.originating_structural_key {
                h_children.push(Presentation::Plain(format!("  originating structural key: {}", key)));
            }
            h_children.push(Presentation::Plain(format!("  adopted: {}", if self.result.adopted { "yes" } else { "no" })));
        } else if self.result.adopted {
            if let Some(ref lid) = self.result.lineage_id {
                h_children.push(Presentation::Plain(format!("  lineage: {}", lid)));
            }
            h_children.push(Presentation::Plain("  archive kind: managed lineage".into()));
        }

        if !self.result.moved_branches.is_empty() {
            h_children.push(Presentation::Section {
                title: "archived branches:".into(),
                children: self
                    .result
                    .moved_branches
                    .iter()
                    .map(|b| Presentation::Plain(format!("  {}", b)))
                    .collect(),
            });
        }

        for warn in &self.result.unowned_warnings {
            h_children.push(Presentation::Plain(warn.clone()));
        }

        let title = if self.result.is_dry_run {
            format!("Dry run: planned archive for staircase '{}':", self.result.canonical_name)
        } else if self.result.archive_kind == "implicit-snapshot" {
            format!("Archived implicit staircase '{}'.", self.result.canonical_name)
        } else if self.result.adopted {
            format!("Adopted & archived implicit staircase '{}'.", self.result.canonical_name)
        } else {
            format!("Archived managed staircase '{}'.", self.result.canonical_name)
        };

        let rec_id = self.result.archive_id.clone().or_else(|| self.result.lineage_id.clone()).unwrap_or_default();

        Presentation::pair(
            Presentation::Section {
                title,
                children: h_children,
            },
            Presentation::Record(vec![
                "archived".into(),
                self.result.canonical_name.clone(),
                rec_id,
            ]),
        )
    }
}

impl ToPresentation for ReleaseNameOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!(
                "Released canonical name reservation (record OID: {})",
                self.record_oid
            )),
            Presentation::Record(vec!["name_released".into(), self.record_oid.clone()]),
        )
    }
}

use crate::cli::describe::DescribeOutput;

impl ToPresentation for DescribeOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];
        if let Some(ref t) = self.title {
            h_children.push(Presentation::Field {
                label: "Title".into(),
                value: t.clone(),
            });
        }
        if let Some(ref d) = self.description {
            h_children.push(Presentation::Plain(d.clone()));
        }
        if h_children.is_empty() {
            h_children.push(Presentation::Plain(format!(
                "No description for staircase '{}'",
                self.name
            )));
        }

        let mut p_records = vec![Presentation::Record(vec!["name".into(), self.name.clone()])];
        if let Some(ref t) = self.title {
            p_records.push(Presentation::Record(vec!["title".into(), t.clone()]));
        }
        if let Some(ref d) = self.description {
            p_records.push(Presentation::Record(vec![
                "description".into(),
                d.replace('\n', "\n"),
            ]));
        }

        Presentation::pair(
            Presentation::List(h_children),
            Presentation::List(p_records),
        )
    }
}

use crate::cli::id::IdResult;

impl ToPresentation for IdResult {
    fn to_presentation(&self) -> Presentation {
        let mut h_items = vec![];
        if self.was_implicit && self.kind == IdentityKind::Lineage {
            h_items.push(Presentation::Plain(format!(
                "adopted implicit staircase '{}'",
                self.name
            )));
        }
        h_items.push(Presentation::Plain(self.id.clone()));

        Presentation::pair(
            Presentation::List(h_items),
            Presentation::Plain(self.id.clone()),
        )
    }
}
