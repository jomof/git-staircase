use anyhow::{Result, anyhow};
use crate::cli::{Command, PresentationOutput, StaircaseSelectorArgs, ToHuman, ToPorcelain};
use crate::core;
use crate::git::GitRepo;
use clap::Args;
use serde::Serialize;
use std::env;
use std::fs;
use std::process;

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

impl ToHuman for DescribeOutput {
    fn to_human(&self) -> String {
        let mut out = String::new();
        if let Some(ref t) = self.title {
            out.push_str(&format!("Title: {}\n", t));
        }
        if let Some(ref d) = self.description {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(d);
        }
        if out.is_empty() {
            out = format!("No description for staircase '{}'", self.name);
        }
        out
    }
}

impl ToPorcelain for DescribeOutput {
    fn to_porcelain(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("name\t{}\n", self.name));
        if let Some(ref t) = self.title {
            out.push_str(&format!("title\t{}\n", t));
        }
        if let Some(ref d) = self.description {
            out.push_str(&format!("description\t{}\n", d.replace('\n', "\\n")));
        }
        out
    }
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

            let temp_dir = env::temp_dir();
            let temp_file = temp_dir.join(format!("STAIRCASE_DESC_{}.txt", uuid::Uuid::new_v4().simple()));
            fs::write(&temp_file, &init_content)?;

            let editor = env::var("GIT_EDITOR")
                .or_else(|_| env::var("VISUAL"))
                .or_else(|_| env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".to_string());

            let status = process::Command::new(&editor)
                .arg(&temp_file)
                .status()
                .map_err(|e| anyhow!("Failed to launch editor '{}': {}", editor, e))?;

            if !status.success() {
                let _ = fs::remove_file(&temp_file);
                return Err(anyhow!("Editor exited with non-zero status"));
            }

            let edited_content = fs::read_to_string(&temp_file)?;
            let _ = fs::remove_file(&temp_file);

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
