pub use crate::model::{StaircaseFamily, StaircaseStatus, Step};
pub use crate::presentation::{Presentation, ToPresentation, UsePresentation};
use serde::Serialize;

pub trait ToHuman {
    fn to_human(&self) -> String;
}

pub trait ToPorcelain {
    fn to_porcelain(&self) -> String;
}

pub(crate) fn escape_machine_field(value: &str) -> String {
    value
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

pub fn render_human(p: &Presentation, indent: usize) -> String {
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
        Presentation::Error {
            code,
            message,
            details,
            ..
        } => {
            let mut out = format!("{}error [{}]: {}\n", space, code, message);
            if !details.is_null() {
                if let Ok(rendered) = serde_json::to_string_pretty(details) {
                    out.push_str(&rendered);
                    out.push('\n');
                }
            }
            out
        }
        Presentation::Porcelain(_) => String::new(),
    }
}

pub fn render_porcelain(p: &Presentation) -> String {
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
        Presentation::Error { code, message, .. } => {
            format!("error\t{}\t{}\n", code, escape_machine_field(message))
        }
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

#[derive(Serialize)]
pub struct Summary<T> {
    #[serde(flatten)]
    pub value: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualification: Option<String>,
}

impl<T> Summary<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            qualification: None,
        }
    }

    pub fn qualified(value: T, qualification: String) -> Self {
        Self {
            value,
            qualification: Some(qualification),
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct ReorderResult {
    pub status: StaircaseStatus,
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct StepsList(pub Vec<Step>);

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

#[derive(Serialize)]
#[serde(transparent)]
pub struct PlainOutput(pub String);

#[derive(Serialize)]
#[serde(transparent)]
pub struct LogOutput(pub Vec<CommitInfo>);
