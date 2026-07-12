use crate::ResolvedStaircase;
use crate::model::{
    Discovery, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step, VerificationResult,
};
use serde::Serialize;

pub trait ToHuman {
    fn to_human(&self) -> String;
}

pub trait ToPorcelain {
    fn to_porcelain(&self) -> String;
}

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

fn render_human(p: &Presentation, indent: usize) -> String {
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
            for row in rows {
                out.push_str(&format!("{}{}\n", space, row.join(" ")));
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

fn render_porcelain(p: &Presentation) -> String {
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

macro_rules! impl_formatting {
    ($t:ty) => {
        impl ToHuman for $t {
            fn to_human(&self) -> String {
                render_human(&self.to_presentation(), 0)
                    .trim_end()
                    .to_string()
            }
        }
        impl ToPorcelain for $t {
            fn to_porcelain(&self) -> String {
                render_porcelain(&self.to_presentation())
            }
        }
    };
}

impl ToPresentation for Step {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} ({})",
                self.name,
                &self.cut[..7]
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.id.clone(),
                self.name.clone(),
                self.cut.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for StaircaseMetadata {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field {
                label: "Name".to_string(),
                value: self.name.clone(),
            },
            Presentation::Field {
                label: "ID".to_string(),
                value: self.id.clone(),
            },
            Presentation::Field {
                label: "Target".to_string(),
                value: self.target.clone(),
            },
        ];

        if let Some(ref policy) = self.verification_policy {
            let mut policy_children = vec![];
            if let Some(ref cmd) = policy.build_command {
                policy_children.push(Presentation::Field {
                    label: "Build".to_string(),
                    value: cmd.clone(),
                });
            }
            if let Some(ref cmd) = policy.test_command {
                policy_children.push(Presentation::Field {
                    label: "Test".to_string(),
                    value: cmd.clone(),
                });
            }
            policy_children.push(Presentation::Field {
                label: "Verify each prefix".to_string(),
                value: policy.verify_each_prefix.to_string(),
            });
            h_children.push(Presentation::Section {
                title: "Verification Policy:".to_string(),
                children: policy_children,
            });
        }

        let mut steps_children = vec![];
        for (i, step) in self.steps.iter().enumerate() {
            steps_children.push(Presentation::Section {
                title: format!("Step {}:", i + 1),
                children: vec![
                    Presentation::Field {
                        label: "ID".to_string(),
                        value: step.id.clone(),
                    },
                    Presentation::Field {
                        label: "Name".to_string(),
                        value: step.name.clone(),
                    },
                    Presentation::Field {
                        label: "Cut".to_string(),
                        value: step.cut.clone(),
                    },
                    if let Some(ref b) = step.branch {
                        Presentation::Field {
                            label: "Branch".to_string(),
                            value: b.clone(),
                        }
                    } else {
                        Presentation::Empty
                    },
                ],
            });
        }
        h_children.push(Presentation::Section {
            title: "Steps:".to_string(),
            children: steps_children,
        });

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: String::new(),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.name.clone(),
                self.id.clone(),
            ]))),
        ])
    }
}
impl_formatting!(StaircaseMetadata);

impl ToPresentation for StaircaseStatus {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![
            Presentation::Field {
                label: "target".to_string(),
                value: self.metadata.target.clone(),
            },
            Presentation::Field {
                label: "state".to_string(),
                value: self.state().to_string(),
            },
            Presentation::Field {
                label: "steps".to_string(),
                value: self.steps.len().to_string(),
            },
            Presentation::Field {
                label: "lineage".to_string(),
                value: if self.is_implicit {
                    "none".to_string()
                } else {
                    self.metadata.id.clone()
                },
            },
        ];

        if let Some(ref results) = self.verification_results {
            let mut v_children = vec![];
            for result in results {
                v_children.push(Presentation::List(vec![
                    Presentation::Human(Box::new(Presentation::Field {
                        label: result.step_name.clone(),
                        value: if result.success {
                            "PASS".to_string()
                        } else {
                            "FAIL".to_string()
                        },
                    })),
                    Presentation::Porcelain(Box::new(Presentation::Record(vec![
                        "verify".to_string(),
                        result.step_name.clone(),
                        if result.success {
                            "pass".to_string()
                        } else {
                            "fail".to_string()
                        },
                        result.cut.clone(),
                    ]))),
                ]));
            }
            children.push(Presentation::Section {
                title: "verification:".to_string(),
                children: v_children,
            });
        }

        let mut steps_rows = vec![];
        for step in &self.steps {
            steps_rows.push(vec![
                step.name.clone(),
                step.actual_oid.as_deref().unwrap_or("none").to_string(),
                if step.is_modified {
                    "modified".to_string()
                } else {
                    "clean".to_string()
                },
                if step.is_stale {
                    "stale".to_string()
                } else {
                    "up-to-date".to_string()
                },
            ]);
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!(
                    "{}{}",
                    self.metadata.name,
                    if self.is_implicit { " (implicit)" } else { "" }
                ),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(vec![
                Presentation::Record(vec![
                    self.metadata.name.clone(),
                    self.metadata.id.clone(),
                    self.state().to_string(),
                ]),
                Presentation::Table {
                    name: Some("step".to_string()),
                    rows: steps_rows,
                },
            ]))),
        ])
    }
}
impl_formatting!(StaircaseStatus);

impl ToPresentation for StaircaseFamily {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![
            Presentation::Field {
                label: "ID".to_string(),
                value: self.id.clone(),
            },
            Presentation::Field {
                label: "Target".to_string(),
                value: self.target.clone(),
            },
            Presentation::Field {
                label: "Roots".to_string(),
                value: self.roots.join(", "),
            },
        ];

        let mut steps_children = vec![];
        for (name, step) in &self.steps {
            let mut step_children = vec![Presentation::Field {
                label: "Cut".to_string(),
                value: step.cut.clone(),
            }];
            if let Some(ref b) = step.branch {
                step_children.push(Presentation::Field {
                    label: "Branch".to_string(),
                    value: b.clone(),
                });
            }
            if !step.children.is_empty() {
                step_children.push(Presentation::Field {
                    label: "Children".to_string(),
                    value: step.children.join(", "),
                });
            }
            steps_children.push(Presentation::Section {
                title: format!("Step {}:", name),
                children: step_children,
            });
        }
        children.push(Presentation::Section {
            title: "Steps:".to_string(),
            children: steps_children,
        });

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Name: {}", self.name),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.name.clone(),
                self.id.clone(),
                "family".to_string(),
                self.steps.len().to_string(),
            ]))),
        ])
    }
}
impl_formatting!(StaircaseFamily);

impl ToPresentation for Discovery {
    fn to_presentation(&self) -> Presentation {
        match self {
            Discovery::Linear(s) => s.to_presentation(),
            Discovery::Ambiguous(f) => f.to_presentation(),
        }
    }
}
impl_formatting!(Discovery);

impl ToPresentation for VerificationResult {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![];
        if !self.success {
            children.push(Presentation::Section {
                title: "Stdout:".to_string(),
                children: vec![Presentation::Plain(self.stdout.clone())],
            });
            children.push(Presentation::Section {
                title: "Stderr:".to_string(),
                children: vec![Presentation::Plain(self.stderr.clone())],
            });
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!(
                    "Step {}: {}",
                    self.step_name,
                    if self.success { "PASSED" } else { "FAILED" }
                ),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.step_name.clone(),
                if self.success {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                self.cut.clone(),
            ]))),
        ])
    }
}
impl_formatting!(VerificationResult);

impl ToPresentation for ResolvedStaircase {
    fn to_presentation(&self) -> Presentation {
        match self {
            ResolvedStaircase::ImplicitFamily(f) => Presentation::List(vec![
                Presentation::Human(Box::new(Presentation::Heading(format!(
                    "Implicit Staircase Family: {}",
                    f.name
                )))),
                f.to_presentation(),
            ]),
            ResolvedStaircase::Managed(m) => Presentation::List(vec![
                Presentation::Human(Box::new(Presentation::Heading(format!(
                    "Managed Staircase: {}",
                    m.name
                )))),
                m.to_presentation(),
            ]),
            ResolvedStaircase::Implicit(m) => Presentation::List(vec![
                Presentation::Human(Box::new(Presentation::Heading(format!(
                    "Implicit Staircase: {}",
                    m.name
                )))),
                m.to_presentation(),
            ]),
        }
    }
}
impl_formatting!(ResolvedStaircase);

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
impl_formatting!(Success);

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
                "{} {} {} {}{}",
                m.name,
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
impl_formatting!(Summary<StaircaseStatus>);

impl ToPresentation for Summary<StaircaseFamily> {
    fn to_presentation(&self) -> Presentation {
        let f = &self.0;
        let path_count = f.steps.values().filter(|s| s.children.is_empty()).count();
        let paths_word = if path_count == 1 { "path" } else { "paths" };
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} {} {} (implicit)",
                f.name, path_count, paths_word
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
impl_formatting!(Summary<StaircaseFamily>);

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
impl_formatting!(ReorderResult);

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
impl_formatting!(StepsList);

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
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
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
impl_formatting!(StaircaseCommits);

#[derive(Serialize)]
#[serde(transparent)]
pub struct PlainOutput(pub String);

impl ToPresentation for PlainOutput {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(self.0.clone())
    }
}
impl_formatting!(PlainOutput);

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
impl_formatting!(LogOutput);

impl ToHuman for Vec<StaircaseStatus> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for Vec<StaircaseStatus> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToHuman for Vec<Discovery> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for Vec<Discovery> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToHuman for Vec<ResolvedStaircase> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for Vec<ResolvedStaircase> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToHuman for Vec<VerificationResult> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ToPorcelain for Vec<VerificationResult> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
