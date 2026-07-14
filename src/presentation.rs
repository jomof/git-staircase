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
