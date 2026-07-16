/*
 * Copyright (C) 2024 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::model::*;
use crate::presentation::{Presentation, ToPresentation, UsePresentation};

impl ToPresentation for Step {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!("{} ({})", self.name, &self.cut[..7])),
            Presentation::Record(vec!["step".into(), "1".into(), self.id.clone(), self.name.clone(), self.cut.clone()]),
        )
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

        Presentation::pair(
            Presentation::Section {
                title: String::new(),
                children: h_children,
            },
            Presentation::Record(vec!["staircase".into(), "1".into(), self.name.clone(), self.id.clone()]),
        )
    }
}

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
                v_children.push(Presentation::pair(
                    Presentation::Field {
                        label: result.step_name.clone(),
                        value: if result.success {
                            "PASS".to_string()
                        } else {
                            "FAIL".to_string()
                        },
                    },
                    Presentation::Record(vec![
                        "verify".to_string(), "1".into(),
                        result.step_name.clone(),
                        if result.success {
                            "pass".to_string()
                        } else {
                            "fail".to_string()
                        },
                        result.cut.clone(),
                    ]),
                ));
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
                children: get_worktree_draft_presentation_fields(draft),
            });
        }

        Presentation::pair(
            Presentation::Section {
                title: format!(
                    "{}{}",
                    self.metadata.name,
                    if self.is_implicit { " (implicit)" } else { "" }
                ),
                children,
            },
            Presentation::List(vec![
                Presentation::Record(vec![
                    "staircase".into(), "1".into(),
                    self.metadata.name.clone(),
                    self.metadata.id.clone(),
                    self.state().to_string(),
                ]),
                Presentation::Table {
                    name: Some("step".to_string()),
                    rows: steps_rows,
                },
            ]),
        )
    }
}

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
        let mut step_names: Vec<_> = self.steps.keys().cloned().collect();
        step_names.sort();
        for name in step_names {
            let step = &self.steps[&name];
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

        Presentation::pair(
            Presentation::Section {
                title: format!("Name: {}", self.name),
                children,
            },
            Presentation::Record(vec![
                "staircase".into(), "1".into(),
                self.name.clone(),
                self.id.clone(),
                "family".to_string(),
                self.steps.len().to_string(),
            ]),
        )
    }
}

impl ToPresentation for Discovery {
    fn to_presentation(&self) -> Presentation {
        match self {
            Discovery::Linear(s) => s.to_presentation(),
            Discovery::Ambiguous(f) => f.to_presentation(),
        }
    }
}

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
        Presentation::pair(
            Presentation::Section {
                title: format!(
                    "Step {}: {}",
                    self.step_name,
                    if self.success { "PASSED" } else { "FAILED" }
                ),
                children,
            },
            Presentation::Record(vec![
                "verify".into(), "1".into(),
                self.step_name.clone(),
                if self.success {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                self.cut.clone(),
            ]),
        )
    }
}

fn get_worktree_draft_presentation_fields(draft: &WorktreeDraft) -> Vec<Presentation> {
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

impl ToPresentation for WorktreeDraft {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Section {
                title: "current worktree draft:".to_string(),
                children: get_worktree_draft_presentation_fields(self),
            },
            Presentation::Record(vec![
                "draft".to_string(), "1".into(),
                self.basis.clone(),
                self.classification.to_string(),
                self.staged_paths.len().to_string(),
                self.unstaged_paths.len().to_string(),
                self.untracked_paths.len().to_string(),
            ]),
        )
    }
}

impl ToPresentation for DraftAttachment {
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
        Presentation::pair(
            Presentation::Plain(format!(
                "Attached to {} with intent '{}' (expected basis: {})",
                attached_to.trim(),
                intent_str,
                if self.expected_basis.len() >= 7 {
                    &self.expected_basis[..7]
                } else {
                    &self.expected_basis
                }
            )),
            Presentation::Record(vec![
                "attached".to_string(), "1".into(),
                self.staircase_name.as_deref().unwrap_or("").to_string(),
                self.step_name.as_deref().unwrap_or("").to_string(),
                intent_str.to_string(),
                self.expected_basis.clone(),
            ]),
        )
    }
}

impl ToPresentation for DraftSnapshot {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!(
                "Created snapshot {} (basis: {})",
                self.id,
                if self.basis.len() >= 7 {
                    &self.basis[..7]
                } else {
                    &self.basis
                }
            )),
            Presentation::Record(vec![
                "snapshot".to_string(), "1".into(),
                self.id.clone(),
                self.basis.clone(),
            ]),
        )
    }
}

impl ToPresentation for ActiveOperationStatus {
    fn to_presentation(&self) -> Presentation {
        Presentation::Section {
            title: "Active Operation:".to_string(),
            children: vec![
                Presentation::Field {
                    label: "ID".to_string(),
                    value: self.operation_id.clone(),
                },
                Presentation::Field {
                    label: "Kind".to_string(),
                    value: self.kind.clone(),
                },
                Presentation::Field {
                    label: "Phase".to_string(),
                    value: self.phase.clone(),
                },
            ],
        }
    }
}

impl UsePresentation for Step {}
impl UsePresentation for StaircaseMetadata {}
impl UsePresentation for StaircaseStatus {}
impl UsePresentation for StaircaseFamily {}
impl UsePresentation for Discovery {}
impl UsePresentation for VerificationResult {}
impl UsePresentation for WorktreeDraft {}
impl UsePresentation for DraftAttachment {}
impl UsePresentation for DraftSnapshot {}
impl UsePresentation for ActiveOperationStatus {}
