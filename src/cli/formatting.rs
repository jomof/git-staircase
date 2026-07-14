use crate::ResolvedStaircase;
use crate::model::{
    Discovery, DraftIntent, StaircaseFamily, StaircaseMetadata, StaircaseStatus, Step,
    VerificationResult, WorktreeDraft,
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

fn get_draft_fields(draft: &WorktreeDraft) -> Vec<Presentation> {
    let mut children = vec![];
    if draft.is_attachment_stale {
        children.push(Presentation::Field {
            label: "attachment".to_string(),
            value: "stale".to_string(),
        });
    } else if let Some(ref att) = draft.attachment {
        let attached_to = format!(
            "{} {}",
            att.staircase_name.as_deref().unwrap_or("unnamed"),
            att.step_name.as_deref().unwrap_or("")
        );
        children.push(Presentation::Field {
            label: "attached to".to_string(),
            value: attached_to.trim().to_string(),
        });
        let intent_str = match &att.intent {
            DraftIntent::ExtendStep => "extend-step",
            DraftIntent::NewStep => "new-step",
            DraftIntent::Unassigned => "unassigned",
            DraftIntent::RewriteStep(_) => "rewrite-step",
        };
        children.push(Presentation::Field {
            label: "intent".to_string(),
            value: intent_str.to_string(),
        });
    }
    let basis_short = if draft.basis.len() >= 7 {
        &draft.basis[..7]
    } else {
        &draft.basis
    };
    children.push(Presentation::Field {
        label: "basis".to_string(),
        value: basis_short.to_string(),
    });
    children.push(Presentation::Field {
        label: "staged".to_string(),
        value: format!("{} paths", draft.staged_paths.len()),
    });
    children.push(Presentation::Field {
        label: "unstaged".to_string(),
        value: format!("{} paths", draft.unstaged_paths.len()),
    });
    children.push(Presentation::Field {
        label: "untracked".to_string(),
        value: format!("{} paths", draft.untracked_paths.len()),
    });
    children.push(Presentation::Field {
        label: "conflicts".to_string(),
        value: if draft.conflicted_paths.is_empty() {
            "none".to_string()
        } else {
            format!("{} paths", draft.conflicted_paths.len())
        },
    });
    children
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

pub trait UsePresentation: ToPresentation {}

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
impl UsePresentation for StaircaseMetadata {}

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
                if step.is_incomplete {
                    "incomplete".to_string()
                } else if step.is_modified {
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

        if let Some(ref draft) = self.worktree_draft {
            children.push(Presentation::Section {
                title: "current worktree draft:".to_string(),
                children: get_draft_fields(draft),
            });
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
impl UsePresentation for StaircaseStatus {}

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
impl UsePresentation for StaircaseFamily {}

impl ToPresentation for Discovery {
    fn to_presentation(&self) -> Presentation {
        match self {
            Discovery::Linear(s) => s.to_presentation(),
            Discovery::Ambiguous(f) => f.to_presentation(),
        }
    }
}
impl UsePresentation for Discovery {}

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
impl UsePresentation for VerificationResult {}

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
impl UsePresentation for ResolvedStaircase {}

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

impl ToPresentation for WorktreeDraft {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: "current worktree draft:".to_string(),
                children: get_draft_fields(self),
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "draft".to_string(),
                self.basis.clone(),
                self.classification.to_string(),
                self.staged_paths.len().to_string(),
                self.unstaged_paths.len().to_string(),
                self.untracked_paths.len().to_string(),
            ]))),
        ])
    }
}
impl UsePresentation for WorktreeDraft {}

impl ToPresentation for crate::model::DraftAttachment {
    fn to_presentation(&self) -> Presentation {
        let attached_to = format!(
            "{} {}",
            self.staircase_name.as_deref().unwrap_or("unnamed"),
            self.step_name.as_deref().unwrap_or("")
        );
        let intent_str = match &self.intent {
            DraftIntent::ExtendStep => "extend-step",
            DraftIntent::NewStep => "new-step",
            DraftIntent::Unassigned => "unassigned",
            DraftIntent::RewriteStep(_) => "rewrite-step",
        };
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Attached to {} with intent '{}' (expected basis: {})",
                attached_to.trim(),
                intent_str,
                if self.expected_basis.len() >= 7 {
                    &self.expected_basis[..7]
                } else {
                    &self.expected_basis
                }
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "attached".to_string(),
                self.staircase_name.as_deref().unwrap_or("").to_string(),
                self.step_name.as_deref().unwrap_or("").to_string(),
                intent_str.to_string(),
                self.expected_basis.clone(),
            ]))),
        ])
    }
}
impl UsePresentation for crate::model::DraftAttachment {}

impl ToPresentation for crate::model::DraftSnapshot {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Created snapshot {} (basis: {})",
                self.id,
                if self.basis.len() >= 7 {
                    &self.basis[..7]
                } else {
                    &self.basis
                }
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "snapshot".to_string(),
                self.id.clone(),
                self.basis.clone(),
            ]))),
        ])
    }
}
impl UsePresentation for crate::model::DraftSnapshot {}

impl ToPresentation for crate::core::draft::MaterializeResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Materialized draft as commit {} on step '{}' of staircase '{}'",
                if self.commit_oid.len() >= 7 {
                    &self.commit_oid[..7]
                } else {
                    &self.commit_oid
                },
                self.step_name,
                self.staircase_name
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "materialized".to_string(),
                self.staircase_name.clone(),
                self.step_name.clone(),
                self.commit_oid.clone(),
            ]))),
        ])
    }
}
impl UsePresentation for crate::core::draft::MaterializeResult {}

impl<T: ToHuman> ToHuman for Vec<T> {
    fn to_human(&self) -> String {
        self.iter()
            .map(|x| x.to_human())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl<T: ToPorcelain> ToPorcelain for Vec<T> {
    fn to_porcelain(&self) -> String {
        self.iter()
            .map(|x| x.to_porcelain())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
