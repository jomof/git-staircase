use crate::cli::{
    Command, Presentation, PresentationOutput, RequiredStaircaseSelector, StaircaseSelectorArgs,
    ToPresentation, UsePresentation,
};
use crate::core::{self, ResolvedSelector};
use crate::git::GitRepo;
use crate::model::{StaircaseLink, StaircaseUserMetadata, StepMetadata};
use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::env;
use std::fs;
use std::process;

#[derive(Args, Clone, Debug)]
pub struct MetadataCmd {
    #[command(subcommand)]
    pub command: MetadataSubcommands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum MetadataSubcommands {
    /// Show user-facing metadata for a staircase
    Show(MetadataShowArgs),
    /// Edit user-facing metadata in an editor
    Edit(MetadataEditArgs),
    /// Set title of a staircase
    SetTitle(SetTitleArgs),
    /// Add a label to a staircase
    AddLabel(AddLabelArgs),
    /// Remove a label from a staircase
    RemoveLabel(RemoveLabelArgs),
    /// Add a link to a staircase
    AddLink(AddLinkArgs),
    /// Show step metadata
    ShowStep(ShowStepArgs),
    /// Edit step metadata
    EditStep(EditStepArgs),
}

#[derive(Args, Clone, Debug)]
pub struct MetadataShowArgs {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct MetadataEditArgs {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
}

#[derive(Args, Clone, Debug)]
pub struct SetTitleArgs {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    /// New title string
    pub title: String,
}

#[derive(Args, Clone, Debug)]
pub struct AddLabelArgs {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    /// Label name
    pub label: String,
}

#[derive(Args, Clone, Debug)]
pub struct RemoveLabelArgs {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    /// Label name to remove
    pub label: String,
}

#[derive(Args, Clone, Debug)]
pub struct AddLinkArgs {
    #[command(flatten)]
    pub selector: RequiredStaircaseSelector,
    /// Relationship kind (issue, design, documentation, incident, review, dependency)
    #[arg(long)]
    pub relation: String,
    /// Link URL / URI
    #[arg(long)]
    pub url: String,
    /// Display label for link
    #[arg(long)]
    pub label: Option<String>,
    /// Link description
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ShowStepArgs {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    /// Step identifier or ordinal (if not included in selector name:step)
    pub step: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct EditStepArgs {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    /// Step identifier or ordinal (if not included in selector name:step)
    pub step: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct UserMetadataOutput {
    pub name: String,
    pub metadata: StaircaseUserMetadata,
}

impl ToPresentation for UserMetadataOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];
        if let Some(ref title) = self.metadata.title {
            h_children.push(Presentation::Field {
                label: "Title".to_string(),
                value: title.clone(),
            });
        }
        if let Some(ref desc) = self.metadata.description {
            h_children.push(Presentation::Section {
                title: "Description:".to_string(),
                children: vec![Presentation::Plain(desc.clone())],
            });
        }
        if !self.metadata.labels.is_empty() {
            h_children.push(Presentation::Field {
                label: "Labels".to_string(),
                value: self.metadata.labels.join(", "),
            });
        }
        if !self.metadata.links.is_empty() {
            let mut links_children = vec![];
            for link in &self.metadata.links {
                links_children.push(Presentation::Plain(format!(
                    "[{}] {} ({})",
                    link.relationship,
                    link.url,
                    link.label.as_deref().unwrap_or("")
                )));
            }
            h_children.push(Presentation::Section {
                title: "Links:".to_string(),
                children: links_children,
            });
        }

        let mut p_children = vec![Presentation::Record(vec![
            "name".to_string(),
            self.name.clone(),
        ])];
        if let Some(ref title) = self.metadata.title {
            p_children.push(Presentation::Record(vec![
                "title".to_string(),
                title.clone(),
            ]));
        }
        if let Some(ref desc) = self.metadata.description {
            p_children.push(Presentation::Record(vec![
                "description".to_string(),
                desc.clone(),
            ]));
        }
        for label in &self.metadata.labels {
            p_children.push(Presentation::Record(vec![
                "label".to_string(),
                label.clone(),
            ]));
        }
        for link in &self.metadata.links {
            p_children.push(Presentation::Record(vec![
                "link".to_string(),
                link.relationship.clone(),
                link.url.clone(),
                link.label.clone().unwrap_or_default(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Staircase: {}", self.name),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_children))),
        ])
    }
}

impl UsePresentation for UserMetadataOutput {}

#[derive(Serialize, Debug, Clone)]
pub struct StepMetadataOutput {
    pub name: String,
    pub step_key: String,
    pub metadata: StepMetadata,
}

impl ToPresentation for StepMetadataOutput {
    fn to_presentation(&self) -> Presentation {
        let mut h_children = vec![];
        if let Some(ref title) = self.metadata.title {
            h_children.push(Presentation::Field {
                label: "Title".to_string(),
                value: title.clone(),
            });
        }
        if let Some(ref desc) = self.metadata.description {
            h_children.push(Presentation::Field {
                label: "Description".to_string(),
                value: desc.clone(),
            });
        }
        if !self.metadata.labels.is_empty() {
            h_children.push(Presentation::Field {
                label: "Labels".to_string(),
                value: self.metadata.labels.join(", "),
            });
        }

        let mut p_children = vec![
            Presentation::Record(vec!["name".to_string(), self.name.clone()]),
            Presentation::Record(vec!["step".to_string(), self.step_key.clone()]),
        ];
        if let Some(ref title) = self.metadata.title {
            p_children.push(Presentation::Record(vec![
                "title".to_string(),
                title.clone(),
            ]));
        }
        if let Some(ref desc) = self.metadata.description {
            p_children.push(Presentation::Record(vec![
                "description".to_string(),
                desc.clone(),
            ]));
        }
        for label in &self.metadata.labels {
            p_children.push(Presentation::Record(vec![
                "label".to_string(),
                label.clone(),
            ]));
        }

        Presentation::List(vec![
            Presentation::Human(Box::new(Presentation::Section {
                title: format!("Staircase: {}, Step: {}", self.name, self.step_key),
                children: h_children,
            })),
            Presentation::Porcelain(Box::new(Presentation::List(p_children))),
        ])
    }
}

impl UsePresentation for StepMetadataOutput {}

impl Command for MetadataCmd {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        match &self.command {
            MetadataSubcommands::Show(args) => {
                let sel = args.selector.resolve(repo)?;
                let meta = core::get_user_metadata(repo, &sel)?;
                Ok(Box::new(UserMetadataOutput {
                    name: sel.staircase.metadata().name.clone(),
                    metadata: meta,
                }))
            }
            MetadataSubcommands::Edit(args) => {
                let sel = args.selector.resolve(repo)?;
                let (current_meta, expected_record_oid) =
                    core::get_user_metadata_snapshot(repo, &sel)?;
                let json_str = serde_json::to_string_pretty(&current_meta)?;

                let temp_dir = env::temp_dir();
                let temp_file = temp_dir.join(format!(
                    "STAIRCASE_META_{}.json",
                    uuid::Uuid::new_v4().simple()
                ));
                fs::write(&temp_file, &json_str)?;

                let editor = env::var("GIT_EDITOR")
                    .or_else(|_| env::var("VISUAL"))
                    .or_else(|_| env::var("EDITOR"))
                    .unwrap_or_else(|_| "vi".to_string());

                let status = process::Command::new(&editor)
                    .arg(&temp_file)
                    .status()
                    .map_err(|e| anyhow!("Failed to launch editor: {}", e))?;

                if !status.success() {
                    let _ = fs::remove_file(&temp_file);
                    return Err(anyhow!("Editor exited with non-zero status"));
                }

                let edited = fs::read_to_string(&temp_file)?;
                let _ = fs::remove_file(&temp_file);

                let parsed: StaircaseUserMetadata = serde_json::from_str(&edited)
                    .map_err(|e| anyhow!("Invalid JSON metadata: {}", e))?;

                core::update_user_metadata_expected(repo, &sel, parsed, &expected_record_oid)?;
                let updated = core::get_user_metadata(repo, &sel)?;
                Ok(Box::new(UserMetadataOutput {
                    name: sel.staircase.metadata().name.clone(),
                    metadata: updated,
                }))
            }
            MetadataSubcommands::SetTitle(args) => {
                let sel = args.selector.resolve(repo)?;
                let record = core::set_title(repo, &sel, &args.title)?;
                Ok(Box::new(UserMetadataOutput {
                    name: record.metadata.name.clone(),
                    metadata: record.user_metadata,
                }))
            }

            MetadataSubcommands::AddLabel(args) => {
                let sel = args.selector.resolve(repo)?;
                let record = core::add_label(repo, &sel, &args.label)?;
                Ok(Box::new(UserMetadataOutput {
                    name: record.metadata.name.clone(),
                    metadata: record.user_metadata,
                }))
            }

            MetadataSubcommands::RemoveLabel(args) => {
                let sel = args.selector.resolve(repo)?;
                let record = core::remove_label(repo, &sel, &args.label)?;
                Ok(Box::new(UserMetadataOutput {
                    name: record.metadata.name.clone(),
                    metadata: record.user_metadata,
                }))
            }

            MetadataSubcommands::AddLink(args) => {
                let sel = args.selector.resolve(repo)?;
                let link = StaircaseLink {
                    id: format!("link-{}", uuid::Uuid::new_v4().simple()),
                    relationship: args.relation.clone(),
                    url: args.url.clone(),
                    label: args.label.clone(),
                    description: args.description.clone(),
                };
                let record = core::add_link(repo, &sel, link)?;
                Ok(Box::new(UserMetadataOutput {
                    name: record.metadata.name.clone(),
                    metadata: record.user_metadata,
                }))
            }

            MetadataSubcommands::ShowStep(args) => {
                let sel = args.selector.resolve(repo)?;
                let step_key = resolve_step_arg(&sel, args.step.as_deref())?;
                let meta = core::get_step_metadata(repo, &sel, &step_key)?;
                Ok(Box::new(StepMetadataOutput {
                    name: sel.staircase.metadata().name.clone(),
                    step_key,
                    metadata: meta,
                }))
            }
            MetadataSubcommands::EditStep(args) => {
                let sel = args.selector.resolve(repo)?;
                let step_key = resolve_step_arg(&sel, args.step.as_deref())?;
                let (current_step_meta, expected_record_oid) =
                    core::get_step_metadata_snapshot(repo, &sel, &step_key)?;
                let json_str = serde_json::to_string_pretty(&current_step_meta)?;

                let temp_dir = env::temp_dir();
                let temp_file =
                    temp_dir.join(format!("STEP_META_{}.json", uuid::Uuid::new_v4().simple()));
                fs::write(&temp_file, &json_str)?;

                let editor = env::var("GIT_EDITOR")
                    .or_else(|_| env::var("VISUAL"))
                    .or_else(|_| env::var("EDITOR"))
                    .unwrap_or_else(|_| "vi".to_string());

                let status = process::Command::new(&editor)
                    .arg(&temp_file)
                    .status()
                    .map_err(|e| anyhow!("Failed to launch editor: {}", e))?;

                if !status.success() {
                    let _ = fs::remove_file(&temp_file);
                    return Err(anyhow!("Editor exited with non-zero status"));
                }

                let edited = fs::read_to_string(&temp_file)?;
                let _ = fs::remove_file(&temp_file);

                let parsed: StepMetadata = serde_json::from_str(&edited)
                    .map_err(|e| anyhow!("Invalid JSON step metadata: {}", e))?;

                core::update_step_metadata_expected(
                    repo,
                    &sel,
                    &step_key,
                    parsed,
                    &expected_record_oid,
                )?;
                let updated = core::get_step_metadata(repo, &sel, &step_key)?;
                Ok(Box::new(StepMetadataOutput {
                    name: sel.staircase.metadata().name.clone(),
                    step_key,
                    metadata: updated,
                }))
            }
        }
    }
}

fn resolve_step_arg(sel: &ResolvedSelector, step_arg: Option<&str>) -> Result<String> {
    if let Some(idx) = sel.step_index {
        let meta = sel.staircase.metadata();
        if idx < meta.steps.len() {
            let step = &meta.steps[idx];
            return Ok(if !step.id.is_empty() {
                step.id.clone()
            } else {
                step.name.clone()
            });
        }
    }
    if let Some(arg) = step_arg {
        return Ok(arg.to_string());
    }
    Err(anyhow!("No step specified"))
}
