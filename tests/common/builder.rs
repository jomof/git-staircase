use crate::common::run_git;
use git_staircase::GitRepo;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
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

    pub fn build(self) -> (TempDir, GitRepo) {
        let mut hasher = Sha256::new();
        for step in &self.steps {
            match step {
                Step::Git(args) => {
                    hasher.update(b"git:");
                    for arg in args {
                        hasher.update(arg.as_bytes());
                        hasher.update(b":");
                    }
                }
                Step::WriteFile(file, content) => {
                    hasher.update(b"write:");
                    hasher.update(file.as_bytes());
                    hasher.update(b":");
                    hasher.update(content.as_bytes());
                }
            }
            hasher.update(b"\n");
        }
        let hash = format!("{:x}", hasher.finalize());
        let cache_dir = std::env::temp_dir().join("git_staircase_test_cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join(format!("{}.zip", hash));

        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();

        if cache_file.exists() {
            let file = fs::File::open(&cache_file).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();
            archive.extract(&path).unwrap();
        } else {
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

            // Zip the constructed repo
            let file = fs::File::create(&cache_file).unwrap();
            let mut zip = zip::ZipWriter::new(file);

            let mut walk_dir = vec![path.clone()];
            while let Some(current_dir) = walk_dir.pop() {
                // To guarantee reproducible order, we could sort the entries, but the hash of steps is deterministic
                // We'll just read dir.
                let mut entries: Vec<_> = fs::read_dir(&current_dir)
                    .unwrap()
                    .map(|res| res.unwrap())
                    .collect();
                entries.sort_by_key(|e| e.file_name());

                for entry in entries {
                    let file_path = entry.path();
                    let name = file_path
                        .strip_prefix(&path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();

                    if entry.file_type().unwrap().is_dir() {
                        let options = zip::write::SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Stored)
                            .unix_permissions(0o755);
                        zip.add_directory(&name, options).unwrap();
                        walk_dir.push(file_path);
                    } else if entry.file_type().unwrap().is_file() {
                        let metadata = fs::metadata(&file_path).unwrap();
                        let permissions = metadata.permissions().mode();
                        let options = zip::write::SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Stored)
                            .unix_permissions(permissions);
                        zip.start_file(&name, options).unwrap();
                        let mut f = fs::File::open(file_path).unwrap();
                        std::io::copy(&mut f, &mut zip).unwrap();
                    }
                    // symlinks are skipped for now unless specifically required
                }
            }
            zip.finish().unwrap();
        }

        let repo = GitRepo::new(path);
        (tmp, repo)
    }
}
