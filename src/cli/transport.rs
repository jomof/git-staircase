use crate::GitRepo;
use crate::cli::{Command, Presentation, PresentationOutput, ToPresentation};
use crate::core;
use crate::core::refs::{ARCHIVE_PREFIX, PUBLIC_PREFIX, STATE_PREFIX, StaircaseRefs};
use crate::error::StaircaseError;
use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Args, Clone, Debug)]
pub struct Push {
    pub remote: Option<String>,
    pub selectors: Vec<String>,
    #[arg(long)]
    pub include_archived: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub struct Fetch {
    pub remote: Option<String>,
    #[arg(long)]
    pub include_archived: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransportResult {
    pub schema: String,
    pub version: u32,
    pub direction: String,
    pub remote: String,
    pub refspecs: Vec<String>,
    pub review_publication: bool,
    pub dry_run: bool,
}

impl ToPresentation for TransportResult {
    fn to_presentation(&self) -> Presentation {
        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Plain(format!(
                "Successfully {}ed to/from remote '{}' ({} refspecs)",
                self.direction,
                self.remote,
                self.refspecs.len()
            )))),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                self.direction.clone(),
                self.remote.clone(),
                self.refspecs.len().to_string(),
            ]))),
        ])
    }
}

impl Command for Push {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let remote = self.remote.as_deref().unwrap_or("origin");
        let mut refspecs = BTreeSet::new();
        if self.selectors.is_empty() {
            refspecs.insert(format!("{}*:{}*", PUBLIC_PREFIX, PUBLIC_PREFIX));
            refspecs.insert(format!("{}*:{}*", STATE_PREFIX, STATE_PREFIX));
        } else {
            for selector in &self.selectors {
                let resolved = core::resolve_staircase(repo, selector, None)?
                    .ok_or_else(|| StaircaseError::NotFound(selector.clone()))?;
                if !resolved.is_managed() {
                    return Err(StaircaseError::Other(format!(
                        "cannot transport implicit staircase '{}'; adopt it first",
                        selector
                    ))
                    .into());
                }
                let metadata = resolved.metadata();
                let record_ref = StaircaseRefs::state_record(&metadata.id);
                refspecs.insert(format!("{}:{}", record_ref, record_ref));
                let prefix = format!("{}{}/", STATE_PREFIX, metadata.id);
                refspecs.insert(format!("{}*:{}*", prefix, prefix));
                let public = StaircaseRefs::public(&metadata.name);
                if repo.resolve_ref_opt(&public)?.is_some() {
                    refspecs.insert(format!("{}:{}", public, public));
                }
            }
        }
        if self.include_archived {
            refspecs.insert(format!("{}*:{}*", ARCHIVE_PREFIX, ARCHIVE_PREFIX));
        }
        let refspecs = refspecs.into_iter().collect::<Vec<_>>();
        let mut args = vec!["push", "--atomic"];
        if self.dry_run {
            args.push("--dry-run");
        }
        args.push(remote);
        args.extend(refspecs.iter().map(String::as_str));
        repo.run(&args)?;
        Ok(Box::new(TransportResult {
            schema: "git-staircase/transport-result".into(),
            version: 1,
            direction: "push".into(),
            remote: remote.into(),
            refspecs,
            review_publication: false,
            dry_run: self.dry_run,
        }))
    }
}

impl Command for Fetch {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let remote = self.remote.as_deref().unwrap_or("origin");
        repo.run(&["check-ref-format", "--branch", remote])?;
        let mut refspecs = vec![
            format!("+{}*:refs/remotes/{}/staircases/*", PUBLIC_PREFIX, remote),
            format!(
                "+{}*:refs/remotes/{}/staircase-state/*",
                STATE_PREFIX, remote
            ),
        ];
        if self.include_archived {
            refspecs.push(format!(
                "+{}*:refs/remotes/{}/staircase-archive/*",
                ARCHIVE_PREFIX, remote
            ));
        }
        let mut args = vec!["fetch"];
        if self.dry_run {
            args.push("--dry-run");
        }
        args.push(remote);
        args.extend(refspecs.iter().map(String::as_str));
        repo.run(&args)?;
        Ok(Box::new(TransportResult {
            schema: "git-staircase/transport-result".into(),
            version: 1,
            direction: "fetch".into(),
            remote: remote.into(),
            refspecs,
            review_publication: false,
            dry_run: self.dry_run,
        }))
    }
}
