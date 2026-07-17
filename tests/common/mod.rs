use git_staircase::GitRepo;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub mod builder;
pub use builder::RepoBuilder;

use git_staircase::workspace::storage::{StorageDirGuard, set_thread_storage_dir};

pub struct TestContext {
    pub tmp: TempDir,
    #[allow(dead_code)]
    pub _storage_guard: StorageDirGuard,
    #[allow(dead_code)]
    pub repo: GitRepo,
}

impl TestContext {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let (tmp, repo) = setup_repo();
        let ws_dir = tmp.path().join(".git").join("ws_storage");
        let _storage_guard = set_thread_storage_dir(&ws_dir);
        TestContext {
            tmp,
            _storage_guard,
            repo,
        }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        self.tmp.path()
    }

    #[allow(dead_code)]
    pub fn run_git(&self, args: &[&str]) -> String {
        let res = run_git(self.path(), args);
        self.repo.clear_cache();
        res
    }

    #[allow(dead_code)]
    pub fn commit(&self, file: &str, contents: &str, msg: &str) -> String {
        let res = commit(self.path(), file, contents, msg);
        self.repo.clear_cache();
        res
    }

    #[allow(dead_code)]
    pub fn run_staircase(&self, args: &[&str]) -> (bool, String, String) {
        let res = run_staircase(self.path(), args);
        self.repo.clear_cache();
        res
    }
}

#[allow(dead_code)]
pub fn run_git(dir: &Path, args: &[&str]) -> String {
    let mut actual_args: Vec<&str> = args.to_vec();
    if actual_args.first() == Some(&"init")
        && !actual_args.contains(&"-b")
        && !actual_args.iter().any(|a| a.starts_with("--initial-branch"))
    {
        actual_args.push("-b");
        actual_args.push("main");
    }

    let output = Command::new("git")
        .current_dir(dir)
        .args(&actual_args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_QUARANTINE_PATH")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
        actual_args,
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
    let canonical_path = tmp.path().canonicalize().unwrap_or_else(|_| tmp.path().to_path_buf());
    let storage_dir = canonical_path.join(".git").join("ws_storage");
    fs::create_dir_all(&storage_dir).unwrap();
    let repo = RepoBuilder::new()
        .git(&["init", "-b", "main"])
        .git(&["config", "core.hooksPath", "/dev/null"])
        .write_file("init.txt", "initial")
        .git(&["add", "."])
        .git(&["commit", "--allow-empty", "-m", "initial commit"])
        .build_in_dir_with_storage(&tmp, &storage_dir);
    (tmp, repo)
}

#[allow(dead_code)]
pub fn run_staircase(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let repo_ws_dir = canonical_dir.join(".git").join("ws_storage");
    let _ = std::fs::create_dir_all(&repo_ws_dir);
    let ws_dir = repo_ws_dir;
    let bin = env!("CARGO_BIN_EXE_git-staircase");
    let output = Command::new(bin)
        .current_dir(&canonical_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_QUARANTINE_PATH")
        .env("RUST_BACKTRACE", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test User")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .arg("--storage-dir")
        .arg(&ws_dir)
        .args(args)
        .output()
        .expect("Failed to execute git-staircase");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    )
}

use std::sync::{Mutex, MutexGuard};

#[allow(dead_code)]
pub static ENV_LOCK: Mutex<()> = Mutex::new(());
#[allow(dead_code)]
pub static WORKSPACE_DIR_LOCK: Mutex<()> = Mutex::new(());
#[allow(dead_code)]
pub static PROVIDER_DIR_LOCK: Mutex<()> = Mutex::new(());
#[allow(dead_code)]
pub static PATH_LOCK: Mutex<()> = Mutex::new(());
#[allow(dead_code)]
pub static GIT_CONFIG_LOCK: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
pub struct EnvGuard<'a> {
    _guards: Vec<MutexGuard<'a, ()>>,
    vars: Vec<(String, Option<String>)>,
    _storage_guard: Option<git_staircase::workspace::storage::StorageDirGuard>,
    _provider_guard: Option<git_staircase::workspace::provider::ProviderDirGuard>,
}

impl<'a> EnvGuard<'a> {
    #[allow(dead_code)]
    pub fn set(vars: &[(&str, &Path)]) -> Self {
        let mut guards = Vec::new();
        let mut original = Vec::new();
        let storage_guard = None;
        let provider_guard = None;
        for (k, v) in vars {
            match *k {
                "PATH" => {
                    guards.push(PATH_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
                "GIT_CONFIG_GLOBAL" => {
                    guards.push(GIT_CONFIG_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
                _ => {
                    guards.push(ENV_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
            }
        }
        EnvGuard {
            _guards: guards,
            vars: original,
            _storage_guard: storage_guard,
            _provider_guard: provider_guard,
        }
    }

    #[allow(dead_code)]
    pub fn set_str(vars: &[(&str, &str)]) -> Self {
        let mut guards = Vec::new();
        let mut original = Vec::new();
        let storage_guard = None;
        let provider_guard = None;
        for (k, v) in vars {
            match *k {
                "PATH" => {
                    guards.push(PATH_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
                "GIT_CONFIG_GLOBAL" => {
                    guards.push(GIT_CONFIG_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
                _ => {
                    guards.push(ENV_LOCK.lock().unwrap());
                    let orig = std::env::var(k).ok();
                    original.push((k.to_string(), orig));
                    unsafe { std::env::set_var(k, v); }
                }
            }
        }
        EnvGuard {
            _guards: guards,
            vars: original,
            _storage_guard: storage_guard,
            _provider_guard: provider_guard,
        }
    }
}

impl<'a> Drop for EnvGuard<'a> {
    fn drop(&mut self) {
        for (k, orig) in &self.vars {
            match orig {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}
