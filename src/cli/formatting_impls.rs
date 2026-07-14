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

use crate::core::draft::MaterializeResult;
use crate::core::local::{DiscoveryOverride, LayoutState, LocalMutationResult};
use crate::core::operation::{OperationJournal, OperationResult};
use crate::core::resolved::ResolvedStaircase;
use crate::core::verification::DraftVerificationEvidence;
use crate::model::*;
use crate::presentation::{Presentation, ToPresentation, UsePresentation};
use crate::workspace::review_provider::*;

// --- model.rs implementations ---

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
                children: get_worktree_draft_presentation_fields(draft),
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
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: "current worktree draft:".to_string(),
                children: get_worktree_draft_presentation_fields(self),
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

impl ToPresentation for DraftSnapshot {
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

// --- core/draft.rs implementations ---

impl ToPresentation for MaterializeResult {
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

impl UsePresentation for MaterializeResult {}

// --- core/local.rs implementations ---

impl ToPresentation for LocalMutationResult {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![
            Presentation::Field {
                label: "kind".to_string(),
                value: self.kind.clone(),
            },
            Presentation::Field {
                label: "staircase".to_string(),
                value: self.staircase_name.clone(),
            },
        ];
        if let Some(ref oid) = self.record_oid {
            children.push(Presentation::Field {
                label: "record oid".to_string(),
                value: oid[..7.min(oid.len())].to_string(),
            });
        }
        if self.dry_run {
            children.push(Presentation::Plain("(dry run)".to_string()));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Operation '{}' completed successfully:", self.kind),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.kind.clone(),
                self.staircase_name.clone(),
                self.record_oid.clone().unwrap_or_default(),
            ]))),
        ])
    }
}

impl ToPresentation for LayoutState {
    fn to_presentation(&self) -> Presentation {
        let mut branches = vec![];
        for b in &self.branches {
            branches.push(vec![
                b.step_name.clone(),
                b.expected_oid[..7.min(b.expected_oid.len())].to_string(),
                b.actual_oid.as_deref().unwrap_or("none")
                    [..7.min(b.actual_oid.as_deref().unwrap_or("none").len())]
                    .to_string(),
            ]);
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Layout state for staircase {}:", self.staircase_id),
                children: vec![
                    Presentation::Field {
                        label: "profile".to_string(),
                        value: self.profile.clone().unwrap_or("none".into()),
                    },
                    Presentation::Field {
                        label: "base".to_string(),
                        value: self.base.clone().unwrap_or("none".into()),
                    },
                    Presentation::Field {
                        label: "state".to_string(),
                        value: self.state.clone(),
                    },
                    Presentation::Table {
                        name: Some("branches".into()),
                        rows: branches,
                    },
                ],
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "layout".to_string(),
                self.staircase_id.clone(),
                self.state.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for DiscoveryOverride {
    fn to_presentation(&self) -> Presentation {
        Presentation::Record(vec![self.id.clone(), self.kind.clone(), self.value.clone()])
    }
}

impl UsePresentation for LocalMutationResult {}
impl UsePresentation for LayoutState {}
impl UsePresentation for DiscoveryOverride {}

// --- core/operation.rs implementations ---

impl ToPresentation for OperationResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(format!(
            "Operation {} completed (ID: {})",
            self.transition, self.operation_id
        ))
    }
}

impl ToPresentation for OperationJournal {
    fn to_presentation(&self) -> Presentation {
        Presentation::Section {
            title: format!("Operation '{}' (ID: {})", self.kind, self.operation_id),
            children: vec![
                Presentation::Field {
                    label: "Phase".into(),
                    value: format!("{:?}", self.phase),
                },
                Presentation::Field {
                    label: "Disposition".into(),
                    value: self.disposition.clone(),
                },
            ],
        }
    }
}

impl UsePresentation for OperationResult {}
impl UsePresentation for OperationJournal {}

// --- core/resolved.rs implementations ---

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

// --- core/verification.rs implementations ---

impl ToPresentation for DraftVerificationEvidence {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(format!(
            "Verification evidence for basis {}",
            self.basis_oid
        ))
    }
}

impl UsePresentation for DraftVerificationEvidence {}

// --- workspace/review_provider.rs implementations ---

impl ToPresentation for UnifiedProviderLanding {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Landed {} items via {}",
                self.landed.len(),
                self.provider_label
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "landed".into(),
                self.landed.len().to_string(),
                self.provider_label.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewShow {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field {
                label: "Project".into(),
                value: self.project.clone(),
            },
            Presentation::Field {
                label: "Destination Branch".into(),
                value: self.destination_branch.clone(),
            },
        ];
        for (k, v) in &self.details {
            h_children.push(Presentation::Field {
                label: k.clone(),
                value: v.clone(),
            });
        }
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!(
                "  {} {} [{}]",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            )));
        }
        h_children.push(Presentation::Section {
            title: "Commits:".into(),
            children: items,
        });

        let mut p_records = vec![
            Presentation::Record(vec!["host".into(), self.host.clone()]),
            Presentation::Record(vec!["project".into(), self.project.clone()]),
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec![
                "commit".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Host: {}", self.provider_label, self.host),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

impl ToPresentation for UnifiedReviewStatus {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field {
                label: "Host".into(),
                value: self.host.clone(),
            },
            Presentation::Field {
                label: "Project".into(),
                value: self.project.clone(),
            },
        ];
        for (k, v) in &self.details {
            h_children.push(Presentation::Field {
                label: k.clone(),
                value: v.clone(),
            });
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Review Status: {}", self.provider_label, self.status),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "status".into(),
                self.status.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewPlan {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![
            Presentation::Field {
                label: "Target Ref".into(),
                value: self.target.clone(),
            },
            Presentation::Field {
                label: "Mapping Policy".into(),
                value: self.policy.clone(),
            },
        ];
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!(
                "    - {} {} ({})",
                &item.oid[..7.min(item.oid.len())],
                item.title,
                item.detail
            )));
        }
        h_children.push(Presentation::Section {
            title: "Commits to push:".into(),
            children: items,
        });
        if !self.warnings.is_empty() {
            h_children.push(Presentation::Section {
                title: "Warnings:".into(),
                children: self
                    .warnings
                    .iter()
                    .map(|w| Presentation::Plain(format!("    - {}", w)))
                    .collect(),
            });
        }

        let mut p_records = vec![
            Presentation::Record(vec!["push_ref".into(), self.target.clone()]),
            Presentation::Record(vec!["mapping_policy".into(), self.policy.clone()]),
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec![
                "commit".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Upload Plan:", self.provider_label),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_records))),
        ])
    }
}

impl ToPresentation for UnifiedReviewUpload {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("{} Upload Complete:", self.provider_label),
                children: vec![Presentation::Plain(self.summary.clone())],
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "result".into(),
                self.summary.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewReconcile {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} Reconcile Status: {}",
                self.provider_label, self.status
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "status".into(),
                self.status.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewOpen {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "{} Review URL: {}",
                self.provider_label, self.url
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "url".into(),
                self.url.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedReviewMutation {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];
        for detail in &self.details {
            h_children.push(Presentation::Plain(format!("  {}", detail)));
        }
        if let (Some(before), Some(after)) = (&self.record_before, &self.record_after) {
            h_children.push(Presentation::Plain(format!(
                "record revision: {} -> {}",
                before, after
            )));
        }
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!(
                    "{} review {}: {} association(s)",
                    self.provider_label, self.action, self.changed
                ),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.action.clone(),
                self.changed.to_string(),
                self.provider_label.clone(),
            ]))),
        ])
    }
}

impl ToPresentation for UnifiedProviderVerification {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Provider verification status: {}",
                self.status
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "status".into(),
                self.status.clone(),
            ]))),
        ])
    }
}

impl UsePresentation for UnifiedProviderLanding {}
impl UsePresentation for UnifiedReviewShow {}
impl UsePresentation for UnifiedReviewStatus {}
impl UsePresentation for UnifiedReviewPlan {}
impl UsePresentation for UnifiedReviewUpload {}
impl UsePresentation for UnifiedReviewReconcile {}
impl UsePresentation for UnifiedReviewOpen {}
impl UsePresentation for UnifiedReviewMutation {}
impl UsePresentation for UnifiedProviderVerification {}
