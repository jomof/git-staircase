#[derive(Debug, PartialEq, Clone)]
pub enum Presentation {
    Empty,
    Plain(String),
    Heading(String),
    Field {
        label: String,
        value: String,
    },
    Section {
        title: String,
        children: Vec<Presentation>,
    },
    Table {
        name: Option<String>,
        rows: Vec<Vec<String>>,
    },
    Record(Vec<String>),
    List(Vec<Presentation>),
    Human(Box<Presentation>),
    Porcelain(Box<Presentation>),
    Error {
        code: String,
        message: String,
        exit_status: i32,
        details: serde_json::Value,
    },
}

impl Presentation {
    pub fn human(inner: Presentation) -> Self {
        Self::Human(Box::new(inner))
    }

    pub fn porcelain(inner: Presentation) -> Self {
        Self::Porcelain(Box::new(inner))
    }

    pub fn pair(h: Presentation, p: Presentation) -> Self {
        Self::List(vec![Self::human(h), Self::porcelain(p)])
    }

    pub fn field(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Field {
            label: label.into(),
            value: value.into(),
        }
    }

    pub fn section(title: impl Into<String>, children: Vec<Presentation>) -> Self {
        Self::Section {
            title: title.into(),
            children,
        }
    }

    pub fn table(name: impl Into<Option<String>>, rows: Vec<Vec<String>>) -> Self {
        Self::Table {
            name: name.into(),
            rows,
        }
    }

    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        exit_status: i32,
        details: serde_json::Value,
    ) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
            exit_status,
            details,
        }
    }

    pub fn record<S: Into<String>>(fields: Vec<S>) -> Self {
        Self::Record(fields.into_iter().map(|f| f.into()).collect())
    }
}

#[macro_export]
macro_rules! record {
    ($($x:expr),* $(,)?) => {
        $crate::presentation::Presentation::record(vec![$($x),*])
    };
}

pub trait ToPresentation {
    fn to_presentation(&self) -> Presentation;
}

impl<T: ToPresentation> ToPresentation for Vec<T> {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(self.iter().map(|x| x.to_presentation()).collect())
    }
}

pub trait UsePresentation: ToPresentation {}

pub mod cli;
pub mod core;
pub mod model;
pub mod workspace;

#[macro_export]
macro_rules! present_pair {
    ($h:expr, $p:expr) => {
        $crate::presentation::Presentation::pair($h, $p)
    };
}
