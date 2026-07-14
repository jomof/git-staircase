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
use crate::presentation::{Presentation, ToPresentation, UsePresentation};
use crate::record;

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
        record![self.id.clone(), self.kind.clone(), self.value.clone()]
    }
}

impl UsePresentation for LocalMutationResult {}
impl UsePresentation for LayoutState {}
impl UsePresentation for DiscoveryOverride {}

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

impl ToPresentation for DraftVerificationEvidence {
    fn to_presentation(&self) -> Presentation {
        Presentation::Plain(format!(
            "Verification evidence for basis {}",
            self.basis_oid
        ))
    }
}

impl UsePresentation for DraftVerificationEvidence {}
