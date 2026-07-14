use crate::GitRepo;
use crate::cli::{
    Command, Presentation, PresentationOutput, StaircaseSelectorArgs, ToPresentation,
};
use crate::core::refs::StaircaseRefs;
use crate::core::{self, MutationPlan};
use crate::error::StaircaseError;
use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

#[derive(Args, Clone, Debug)]
pub struct Tag {
    pub snapshot_name: String,
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    #[arg(long)]
    pub message: Option<String>,
    #[arg(long)]
    pub sign: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TagResult {
    pub schema: String,
    pub version: u32,
    pub tag_ref: String,
    pub tag_oid: String,
    pub record_oid: String,
    pub replaced_oid: Option<String>,
    pub replaced_record_oid: Option<String>,
    pub dry_run: bool,
}

impl ToPresentation for TagResult {
    fn to_presentation(&self) -> Presentation {
        let mut children = vec![
            Presentation::Field {
                label: "tag".to_string(),
                value: self.tag_ref.clone(),
            },
            Presentation::Field {
                label: "tag oid".to_string(),
                value: self.tag_oid[..7].to_string(),
            },
            Presentation::Field {
                label: "record oid".to_string(),
                value: self.record_oid[..7].to_string(),
            },
        ];
        if let Some(ref old) = self.replaced_oid {
            children.push(Presentation::Field {
                label: "replaced".to_string(),
                value: old[..7].to_string(),
            });
        }
        if self.dry_run {
            children.push(Presentation::Plain("(dry run)".to_string()));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: "Created snapshot tag:".to_string(),
                children,
            })),
            Presentation::Porcelain(Box::new(Presentation::Record(vec![
                "tag".to_string(),
                self.tag_ref.clone(),
                self.tag_oid.clone(),
                self.record_oid.clone(),
            ]))),
        ])
    }
}

impl Command for Tag {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        repo.run(&[
            "check-ref-format",
            &format!("refs/tags/staircase/{}", self.snapshot_name),
        ])?;
        let selector = self.selector.resolve(repo)?;
        if !selector.is_managed() {
            return Err(StaircaseError::Other(
                "snapshot tags require an exact managed record revision".into(),
            )
            .into());
        }
        let record_ref = if selector
            .metadata()
            .lifecycle
            .as_ref()
            .is_some_and(|lifecycle| lifecycle.state == crate::model::LifecycleState::Archived)
        {
            StaircaseRefs::archive_record(&selector.metadata().id)
        } else {
            StaircaseRefs::state_record(&selector.metadata().id)
        };
        let record = core::read_record(repo, &record_ref)?;
        let tag_ref = format!("refs/tags/staircase/{}", self.snapshot_name);
        let replaced_oid = repo.resolve_ref_opt(&tag_ref)?;
        let replaced_record_oid = replaced_oid
            .as_ref()
            .map(|_| repo.run(&["rev-parse", &format!("{}^{{}}", tag_ref)]))
            .transpose()?;
        if replaced_oid.is_some() && !self.force {
            return Err(StaircaseError::RefCollision {
                reference: tag_ref,
                expected: "<missing>".into(),
                actual: replaced_oid.expect("checked"),
            }
            .into());
        }
        let tagger = repo.run(&["var", "GIT_COMMITTER_IDENT"])?;
        let message = self.message.as_deref().unwrap_or("Git Staircase snapshot");
        let mut body = format!(
            "object {}\ntype tree\ntag staircase/{}\ntagger {}\n\n{}\n",
            record.record_oid, self.snapshot_name, tagger, message
        );
        if self.sign {
            let signature = sign_tag_payload(repo, &body)?;
            body.push_str(&signature);
        }
        let tag_oid = if self.dry_run {
            repo.command()
                .args(["hash-object", "-t", "tag", "--stdin"])
                .stdin(body)
                .run()?
        } else {
            repo.run_with_stdin(&["mktag"], &body)?
        };
        let mut plan = MutationPlan::new("tag", Some(record.metadata.id))
            .expected_record(Some(record.record_oid.clone()));
        plan.update(tag_ref.clone(), replaced_oid.clone(), Some(tag_oid.clone()));
        plan.publish(repo, self.dry_run)?;
        Ok(Box::new(TagResult {
            schema: "git-staircase/tag-result".into(),
            version: 1,
            tag_ref,
            tag_oid,
            record_oid: record.record_oid,
            replaced_oid,
            replaced_record_oid,
            dry_run: self.dry_run,
        }))
    }
}

fn sign_tag_payload(repo: &GitRepo, payload: &str) -> Result<String> {
    let format = repo
        .command()
        .args(["config", "--get", "gpg.format"])
        .run()
        .unwrap_or_else(|_| "openpgp".into());
    if format.trim() != "openpgp" {
        return Err(StaircaseError::UnsupportedTopology {
            operation: "tag-sign".into(),
            reason: format!(
                "configured signing format '{}' is not supported for snapshot tags",
                format.trim()
            ),
        }
        .into());
    }
    let program = repo
        .command()
        .args(["config", "--get", "gpg.program"])
        .run()
        .unwrap_or_else(|_| "gpg".into());
    let key = repo
        .command()
        .args(["config", "--get", "user.signingkey"])
        .run()
        .ok();
    let mut command = ProcessCommand::new(program.trim());
    command
        .current_dir(&repo.workdir)
        .args(["--armor", "--detach-sign"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(key) = key.as_deref().filter(|key| !key.trim().is_empty()) {
        command.args(["--local-user", key.trim()]);
    }
    let mut child = command.spawn().map_err(|error| {
        StaircaseError::Other(format!(
            "failed to start configured OpenPGP signer: {}",
            error
        ))
    })?;
    child
        .stdin
        .take()
        .ok_or_else(|| StaircaseError::Other("OpenPGP signer stdin is unavailable".into()))?
        .write_all(payload.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(StaircaseError::Other(format!(
            "OpenPGP signer failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
        .into());
    }
    let signature = String::from_utf8(output.stdout)
        .map_err(|_| StaircaseError::Other("OpenPGP signer returned non-UTF-8 armor".into()))?;
    if !signature.contains("-----BEGIN PGP SIGNATURE-----")
        || !signature.contains("-----END PGP SIGNATURE-----")
    {
        return Err(StaircaseError::Other(
            "OpenPGP signer did not return an armored detached signature".into(),
        )
        .into());
    }
    Ok(signature)
}
