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

use crate::presentation::{Presentation, ToPresentation, UsePresentation};
use crate::workspace::review_provider::*;

impl ToPresentation for UnifiedProviderLanding {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!(
                "Landed {} items via {}",
                self.landed.len(),
                self.provider_label
            )),
            Presentation::Record(vec![
                "landed".into(), "1".into(),
                self.landed.len().to_string(),
                self.provider_label.clone(),
            ]),
        )
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
            record!["review_host".into(), "1".into(), self.host.clone()],
            record!["review_project".into(), "1".into(), self.project.clone()],
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec![
                "commit".into(), "1".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::pair(
            Presentation::Section {
                title: format!("{} Host: {}", self.provider_label, self.host),
                children: h_children,
            },
            Presentation::List(p_records),
        )
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
        Presentation::pair(
            Presentation::Section {
                title: format!("{} Review Status: {}", self.provider_label, self.status),
                children: h_children,
            },
            Presentation::Record(vec!["status".into(), "1".into(), "1".into(), self.status.clone()]),
        )
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
            record!["push_ref".into(), "1".into(), self.target.clone()],
            record!["mapping_policy".into(), "1".into(), self.policy.clone()],
        ];
        for item in &self.items {
            p_records.push(Presentation::Record(vec![
                "commit".into(), "1".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::pair(
            Presentation::Section {
                title: format!("{} Upload Plan:", self.provider_label),
                children: h_children,
            },
            Presentation::List(p_records),
        )
    }
}

impl ToPresentation for UnifiedReviewUpload {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Section {
                title: format!("{} Upload Complete:", self.provider_label),
                children: vec![Presentation::Plain(self.summary.clone())],
            },
            Presentation::Record(vec!["result".into(), "1".into(), self.summary.clone()]),
        )
    }
}

impl ToPresentation for UnifiedReviewReconcile {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!(
                "{} Reconcile Status: {}",
                self.provider_label, self.status
            )),
            Presentation::Record(vec!["status".into(), "1".into(), "1".into(), self.status.clone()]),
        )
    }
}

impl ToPresentation for UnifiedReviewOpen {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!("{} Review URL: {}", self.provider_label, self.url)),
            Presentation::Record(vec!["url".into(), "1".into(), self.url.clone()]),
        )
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
        Presentation::pair(
            Presentation::Section {
                title: format!(
                    "{} review {}: {} association(s)",
                    self.provider_label, self.action, self.changed
                ),
                children: h_children,
            },
            Presentation::Record(vec![
                self.action.clone(), "1".into(),
                self.changed.to_string(),
                self.provider_label.clone(),
            ]),
        )
    }
}

impl ToPresentation for UnifiedProviderVerification {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!("Provider verification status: {}", self.status)),
            Presentation::Record(vec!["status".into(), "1".into(), "1".into(), self.status.clone()]),
        )
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
