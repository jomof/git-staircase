use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs};
use crate::core;
use crate::git::GitRepo;
use anyhow::{Result, anyhow};
use clap::Args;
use serde::Serialize;

#[derive(Args, Clone, Debug)]
pub struct Describe {
    #[command(flatten)]
    pub selector: StaircaseSelectorArgs,
    /// Edit title and description using $EDITOR.
    #[arg(long)]
    pub edit: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct DescribeOutput {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl Command for Describe {
    fn run(&self, repo: &GitRepo) -> Result<Box<dyn PresentationOutput>> {
        let sel = self.selector.resolve(repo)?;
        if self.edit {
            let user_meta = core::get_user_metadata(repo, &sel)?;
            let init_content = format!(
                "# Title: {}\n# Enter title above, description below.\n\n{}",
                user_meta.title.as_deref().unwrap_or(""),
                user_meta.description.as_deref().unwrap_or("")
            );

            let edited_content =
                crate::presentation::cli::edit_in_editor(&init_content, "STAIRCASE_DESC", "txt")?;

            let mut title = None;
            let mut desc_lines = Vec::new();
            for line in edited_content.lines() {
                if line.starts_with("# Title:") {
                    let t = line.strip_prefix("# Title:").unwrap().trim();
                    if !t.is_empty() {
                        title = Some(t.to_string());
                    }
                } else if line.starts_with("# ") {
                    continue;
                } else {
                    desc_lines.push(line);
                }
            }

            let desc = desc_lines.join("\n").trim().to_string();
            let description = if desc.is_empty() { None } else { Some(desc) };

            if let Some(ref t) = title {
                if t.len() > 4096 {
                    return Err(anyhow!("Title exceeds limit of 4 KiB"));
                }
                core::set_title(repo, &sel, t)?;
            }
            if let Some(ref d) = description {
                if d.len() > 1048576 {
                    return Err(anyhow!("Description exceeds limit of 1 MiB"));
                }
                core::set_description(repo, &sel, d)?;
            }

            let updated_user_meta = core::get_user_metadata(repo, &sel)?;
            Ok(Box::new(DescribeOutput {
                name: sel.staircase.metadata().name.clone(),
                title: updated_user_meta.title,
                description: updated_user_meta.description,
            }))
        } else {
            let user_meta = core::get_user_metadata(repo, &sel)?;
            Ok(Box::new(DescribeOutput {
                name: sel.staircase.metadata().name.clone(),
                title: user_meta.title,
                description: user_meta.description,
            }))
        }
    }
}
