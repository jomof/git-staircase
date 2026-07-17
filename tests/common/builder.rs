use crate::common::run_git;
use git_staircase::GitRepo;
use std::fs;
use tempfile::TempDir;

#[derive(Clone, Debug)]
pub enum Step {
    Git(Vec<String>),
    WriteFile(String, String),
}

pub struct RepoBuilder {
    steps: Vec<Step>,
}

impl RepoBuilder {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn git(mut self, args: &[&str]) -> Self {
        self.steps
            .push(Step::Git(args.iter().map(|s| s.to_string()).collect()));
        self
    }

    pub fn write_file(mut self, file: &str, content: &str) -> Self {
        self.steps
            .push(Step::WriteFile(file.to_string(), content.to_string()));
        self
    }

    pub fn build_in_dir(self, tmp: &TempDir) -> GitRepo {
        let path = tmp.path().to_path_buf();

        for step in &self.steps {
            match step {
                Step::Git(args) => {
                    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    run_git(&path, &args_ref);
                }
                Step::WriteFile(file, content) => {
                    let file_path = path.join(file);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    fs::write(file_path, content).unwrap();
                }
            }
        }

        GitRepo::new(path)
    }

    pub fn build_in_dir_with_storage(self, tmp: &TempDir, storage_dir: &std::path::Path) -> GitRepo {
        let path = tmp.path().to_path_buf();

        for step in &self.steps {
            match step {
                Step::Git(args) => {
                    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    run_git(&path, &args_ref);
                }
                Step::WriteFile(file, content) => {
                    let file_path = path.join(file);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    fs::write(file_path, content).unwrap();
                }
            }
        }

        GitRepo::with_storage_dir(path, storage_dir.to_path_buf())
    }

    pub fn build(self) -> (TempDir, GitRepo) {
        let tmp = TempDir::new().unwrap();
        let repo = self.build_in_dir(&tmp);
        (tmp, repo)
    }
}
