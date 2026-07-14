use std::collections::BTreeMap;
use serde_json::Value;
pub use crate::model::{StaircaseFamily, StaircaseStatus, Step};
pub use crate::presentation::{Presentation, ToPresentation, UsePresentation};
use serde::Serialize;

pub trait ToHuman {
    fn to_human(&self) -> String;
}

pub trait ToPorcelain {
    fn to_porcelain(&self) -> String;
}

pub(crate) fn render_human(p: &Presentation, indent: usize) -> String {
    let space = "  ".repeat(indent);
    match p {
        Presentation::Empty => String::new(),
        Presentation::Plain(s) => format!("{}{}\n", space, s),
        Presentation::Heading(s) => format!("{}{}\n", space, s),
        Presentation::Field { label, value } => format!("{}{}: {}\n", space, label, value),
        Presentation::Section { title, children } => {
            let mut out = format!("{}{}\n", space, title);
            for child in children {
                out.push_str(&render_human(child, indent + 1));
            }
            out
        }
        Presentation::Table { name, rows } => {
            let mut out = String::new();
            if let Some(n) = name {
                out.push_str(&format!("{}{}:\n", space, n));
            }
            if rows.is_empty() {
                return out;
            }
            let col_count = rows[0].len();
            let mut col_widths = vec![0; col_count];
            for row in rows {
                for (i, col) in row.iter().enumerate() {
                    if i < col_widths.len() {
                        col_widths[i] = col_widths[i].max(col.len());
                    }
                }
            }
            for row in rows {
                out.push_str(&space);
                for (i, col) in row.iter().enumerate() {
                    let width = col_widths[i];
                    if i == col_count - 1 {
                        out.push_str(col);
                    } else {
                        out.push_str(&format!("{:<width$}  ", col, width = width));
                    }
                }
                out.push('\n');
            }
            out
        }
        Presentation::Record(fields) => format!("{}{}\n", space, fields.join(" ")),
        Presentation::List(items) => {
            let mut out = String::new();
            for item in items {
                out.push_str(&render_human(item, indent));
            }
            out
        }
        Presentation::Human(inner) => render_human(inner, indent),
        Presentation::Porcelain(_) => String::new(),
    }
}

pub(crate) fn render_porcelain(p: &Presentation) -> String {
    match p {
        Presentation::Empty => String::new(),
        Presentation::Plain(s) => format!("{}\n", s),
        Presentation::Heading(_) => String::new(),
        Presentation::Field { label, value } => format!("{}\t{}\n", label, value),
        Presentation::Section { children, .. } => children
            .iter()
            .map(render_porcelain)
            .collect::<Vec<_>>()
            .join(""),
        Presentation::Table { name, rows } => {
            let mut out = String::new();
            for row in rows {
                if let Some(n) = name {
                    out.push_str(n);
                    out.push('\t');
                }
                out.push_str(&row.join("\t"));
                out.push('\n');
            }
            out
        }
        Presentation::Record(fields) => {
            let mut out = fields.join("\t");
            out.push('\n');
            out
        }
        Presentation::List(items) => items
            .iter()
            .map(render_porcelain)
            .collect::<Vec<_>>()
            .join(""),
        Presentation::Human(_) => String::new(),
        Presentation::Porcelain(inner) => render_porcelain(inner),
    }
}

impl<T: UsePresentation> ToHuman for T {
    fn to_human(&self) -> String {
        render_human(&self.to_presentation(), 0)
            .trim_end()
            .to_string()
    }
}

impl<T: UsePresentation> ToPorcelain for T {
    fn to_porcelain(&self) -> String {
        render_porcelain(&self.to_presentation())
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
}

#[derive(Serialize)]
pub struct Success {
    pub message: String,
}

impl Success {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ToPresentation for Success {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.message.clone())
    }
}
impl UsePresentation for Success {}

#[derive(Serialize)]
#[serde(transparent)]
pub struct Summary<T>(pub T);

impl ToPresentation for Summary<StaircaseStatus> {
    fn to_presentation(&self) -> Presentation {
        let s = &self.0;
        let m = &s.metadata;
        let steps_count = m.steps.len();
        let steps_word = if steps_count == 1 { "step" } else { "steps" };
        let implicit_marker = if s.is_implicit { " (implicit)" } else { "" };
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} [{}] {} {} {}{}",
                m.name,
                m.id,
                steps_count,
                steps_word,
                s.state(),
                implicit_marker
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                m.name.clone(),
                m.id.clone(),
                s.state().to_string(),
            ]))),
        ])
    }
}
impl UsePresentation for Summary<StaircaseStatus> {}

impl ToPresentation for Summary<StaircaseFamily> {
    fn to_presentation(&self) -> Presentation {
        let f = &self.0;
        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
        let paths_word = if path_count == 1 { "path" } else { "paths" };
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} [{}] {} {} (implicit)",
                f.name, f.id, path_count, paths_word
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                f.name.clone(),
                f.id.clone(),
                "family".to_string(),
                f.steps.len().to_string(),
            ]))),
        ])
    }
}
impl UsePresentation for Summary<StaircaseFamily> {}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ReorderResult {
    pub status: StaircaseStatus,
}

impl ToPresentation for ReorderResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain("Reordered staircase.".to_string())
    }
}
impl UsePresentation for ReorderResult {}

#[derive(Serialize)]
#[serde(transparent)]
pub struct StepsList(pub Vec<Step>);

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
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Table {
                name: None,
                rows: h_rows,
            })),
            Presentation::Porcelain(Box::new(Presentation::Table {
                name: None,
                rows: p_rows,
            })),
        ])
    }
}
impl UsePresentation for StepsList {}

#[derive(Serialize)]
pub struct StaircaseCommits {
    pub steps: Vec<StepCommits>,
}

#[derive(Serialize)]
pub struct StepCommits {
    pub index: usize,
    pub name: String,
    pub commits: Vec<CommitInfo>,
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
                Presentation::Human(Box::new(Presentation::Section {
                    title: format!("Step {}: {}", step.index, step.name),
                    children: h_commits,
                })),
                Presentation::Porcelain(Box::new(Presentation::Record(vec![
                    "step".to_string(),
                    step.index.to_string(),
                    step.name.clone(),
                ]))),
                Presentation::Porcelain(Box::new(Presentation::List(p_commits))),
            ]));
        }
        Presentation::List(children)
    }
}
impl UsePresentation for StaircaseCommits {}

#[derive(Serialize)]
#[serde(transparent)]
pub struct PlainOutput(pub String);

impl ToPresentation for PlainOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.0.clone())
    }
}
impl UsePresentation for PlainOutput {}

#[derive(Serialize)]
#[serde(transparent)]
pub struct LogOutput(pub Vec<CommitInfo>);

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
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::List(h_items))),
            Presentation::Porcelain(Box::new(Presentation::List(p_items))),
        ])
    }
}
impl UsePresentation for LogOutput {}

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
impl UsePresentation for BTreeMap<String, Value> {}
