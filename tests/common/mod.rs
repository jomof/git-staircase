use git_staircase::GitRepo;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub struct TestContext {
    pub tmp: TempDir,
    #[allow(dead_code)]
    pub repo: GitRepo,
}

impl TestContext {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let (tmp, repo) = setup_repo();
        TestContext { tmp, repo }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        self.tmp.path()
    }

    #[allow(dead_code)]
    pub fn run_git(&self, args: &[&str]) -> String {
        let res = run_git(self.path(), args);
        self.repo.memoizer.clear();
        res
    }

    #[allow(dead_code)]
    pub fn commit(&self, file: &str, contents: &str, msg: &str) -> String {
        let res = commit(self.path(), file, contents, msg);
        self.repo.memoizer.clear();
        res
    }

    #[allow(dead_code)]
    pub fn run_staircase(&self, args: &[&str]) -> (bool, String, String) {
        let res = run_staircase(self.path(), args);
        self.repo.memoizer.clear();
        res
    }
}

#[allow(dead_code)]
pub fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed. Stderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[allow(dead_code)]
pub fn get_head_oid(dir: &Path) -> String {
    let head_file = dir.join(".git").join("HEAD");
    if let Ok(content) = fs::read_to_string(&head_file) {
        let content = content.trim();
        if let Some(ref_path) = content.strip_prefix("ref: ") {
            let ref_file = dir.join(".git").join(ref_path);
            if let Ok(oid) = fs::read_to_string(&ref_file) {
                return oid.trim().to_string();
            }
        } else if content.len() == 40 {
            return content.to_string();
        }
    }
    run_git(dir, &["rev-parse", "HEAD"])
}

#[allow(dead_code)]
pub fn commit(dir: &Path, file: &str, contents: &str, msg: &str) -> String {
    let path = dir.join(file);
    fs::write(path, contents).unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "--allow-empty", "-m", msg]);
    get_head_oid(dir)
}

#[allow(dead_code)]
pub fn setup_repo() -> (TempDir, GitRepo) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    run_git(&path, &["init", "-b", "main"]);
    commit(&path, "init.txt", "initial", "initial commit");
    let repo = GitRepo::new(path);
    (tmp, repo)
}

#[allow(dead_code)]
pub fn get_test_binary_path() -> std::path::PathBuf {
    let bin_str = env!("CARGO_BIN_EXE_git-staircase");
    let mut bin = std::path::PathBuf::from(bin_str);
    if bin_str.contains("/shadow-") || !bin.exists() {
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("git-staircase");
        if fallback.exists() {
            bin = fallback;
        } else if let Ok(entries) = std::fs::read_dir(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("debug")
                .join("deps"),
        ) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map_or(false, |n| n.starts_with("git-staircase-") && !n.contains("."))
                {
                    bin = p;
                    break;
                }
            }
        }
    }
    bin
}

#[allow(dead_code)]
pub fn run_staircase(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let ws_dir = std::env::temp_dir().join(format!(".ws_storage_{:p}", dir));
    let bin = get_test_binary_path();
    let output = match Command::new(&bin)
        .current_dir(dir)
        .env("GIT_STAIRCASE_WORKSPACE_DIR", &ws_dir)
        .args(args)
        .output()
    {
        Ok(out) => out,
        Err(e) => panic!("Failed to run binary '{:?}' in dir '{:?}': {}", bin, dir, e),
    };
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    )
}
