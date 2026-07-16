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
            Presentation::record(vec![
                "landed".into(),
                self.landed.len().to_string(),
                self.provider_label.clone(),
            ]),
        )
    }
}

impl ToPresentation for UnifiedReviewShow {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = Presentation::fields(vec![
            ("Project", self.project.clone()),
            ("Destination Branch", self.destination_branch.clone()),
        ]);
        for (k, v) in &self.details {
            h_children.push(Presentation::field(k.clone(), v.clone()));
        }
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!(
                "  {} {} [{}]",
                Presentation::truncate_hash(&item.oid),
                item.title,
                item.detail
            )));
        }
        h_children.push(Presentation::section("Commits:", items));

        let mut p_records = vec![
            record!["host".into(), self.host.clone()],
            record!["project".into(), self.project.clone()],
        ];
        for item in &self.items {
            p_records.push(Presentation::record(vec![
                "commit".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::pair(
            Presentation::section(
                format!("{} Host: {}", self.provider_label, self.host),
                h_children,
            ),
            Presentation::List(p_records),
        )
    }
}

impl ToPresentation for UnifiedReviewStatus {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = Presentation::fields(vec![
            ("Host", self.host.clone()),
            ("Project", self.project.clone()),
        ]);
        for (k, v) in &self.details {
            h_children.push(Presentation::field(k.clone(), v.clone()));
        }
        Presentation::pair(
            Presentation::section(
                format!("{} Review Status: {}", self.provider_label, self.status),
                h_children,
            ),
            Presentation::record(vec!["status".into(), self.status.clone()]),
        )
    }
}

impl ToPresentation for UnifiedReviewPlan {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = Presentation::fields(vec![
            ("Target Ref", self.target.clone()),
            ("Mapping Policy", self.policy.clone()),
        ]);
        let mut items = vec![];
        for item in &self.items {
            items.push(Presentation::Plain(format!(
                "    - {} {} ({})",
                Presentation::truncate_hash(&item.oid),
                item.title,
                item.detail
            )));
        }
        h_children.push(Presentation::section("Commits to push:", items));
        if !self.warnings.is_empty() {
            h_children.push(Presentation::section(
                "Warnings:",
                self.warnings
                    .iter()
                    .map(|w| Presentation::Plain(format!("    - {}", w)))
                    .collect(),
            ));
        }

        let mut p_records = vec![
            record!["push_ref".into(), self.target.clone()],
            record!["mapping_policy".into(), self.policy.clone()],
        ];
        for item in &self.items {
            p_records.push(Presentation::record(vec![
                "commit".into(),
                item.oid.clone(),
                item.detail.clone(),
            ]));
        }

        Presentation::pair(
            Presentation::section(format!("{} Upload Plan:", self.provider_label), h_children),
            Presentation::List(p_records),
        )
    }
}

impl ToPresentation for UnifiedReviewUpload {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::section(
                format!("{} Upload Complete:", self.provider_label),
                vec![Presentation::Plain(self.summary.clone())],
            ),
            Presentation::record(vec!["result".into(), self.summary.clone()]),
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
            Presentation::record(vec!["status".into(), self.status.clone()]),
        )
    }
}

impl ToPresentation for UnifiedReviewOpen {
    fn to_presentation(&self) -> Presentation {
        Presentation::pair(
            Presentation::Plain(format!("{} Review URL: {}", self.provider_label, self.url)),
            Presentation::record(vec!["url".into(), self.url.clone()]),
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
            Presentation::section(
                format!(
                    "{} review {}: {} association(s)",
                    self.provider_label, self.action, self.changed
                ),
                h_children,
            ),
            Presentation::record(vec![
                self.action.clone(),
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
            Presentation::record(vec!["status".into(), self.status.clone()]),
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
