use crate::error::Result;
use crate::git::GitRepo;
use crate::model::BranchInfo;
use std::collections::HashMap;

pub fn build_branch_graph(
    repo: &GitRepo,
    active_branches: &[BranchInfo],
) -> Result<(HashMap<String, String>, HashMap<String, Vec<String>>)> {
    let mut parents: HashMap<String, String> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    for child in active_branches {
        let mut best_parent: Option<&BranchInfo> = None;
        for parent in active_branches {
            if child.refname == parent.refname {
                continue;
            }
            if parent.oid != child.oid && repo.is_ancestor(&parent.oid, &child.oid)? {
                if let Some(current_best) = best_parent {
                    if repo.is_ancestor(&current_best.oid, &parent.oid)? {
                        best_parent = Some(parent);
                    }
                } else {
                    best_parent = Some(parent);
                }
            }
        }
        if let Some(p) = best_parent {
            parents.insert(child.refname.clone(), p.refname.clone());
            children_map
                .entry(p.refname.clone())
                .or_default()
                .push(child.refname.clone());
        }
    }
    Ok((parents, children_map))
}

pub fn find_roots(
    active_branches: &[BranchInfo],
    parents: &HashMap<String, String>,
) -> Vec<String> {
    let mut roots = Vec::new();
    for b in active_branches {
        if !parents.contains_key(&b.refname) {
            roots.push(b.refname.clone());
        }
    }
    roots
}

pub fn collect_family(root: &str, children_map: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut family_branches = Vec::new();
    let mut stack = vec![root.to_string()];
    while let Some(current) = stack.pop() {
        if !family_branches.contains(&current) {
            family_branches.push(current.clone());
            if let Some(children) = children_map.get(&current) {
                stack.extend(children.iter().cloned());
            }
        }
    }
    family_branches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitRepo;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn commit(dir: &Path, file: &str, content: &str, msg: &str) -> String {
        fs::write(dir.join(file), content).unwrap();
        run_git(dir, &["add", "."]);
        run_git(dir, &["commit", "-m", msg]);
        run_git(dir, &["rev-parse", "HEAD"])
    }

    #[test]
    fn test_build_branch_graph_linear() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        run_git(dir, &["init", "-b", "main"]);
        let _root = commit(dir, "root.txt", "root", "root");

        let c1 = commit(dir, "1.txt", "1", "c1");
        run_git(dir, &["branch", "b1", &c1]);

        let c2 = commit(dir, "2.txt", "2", "c2");
        run_git(dir, &["branch", "b2", &c2]);

        let repo = GitRepo::new(dir.to_path_buf());
        let active_branches = vec![
            BranchInfo {
                refname: "refs/heads/b1".to_string(),
                oid: c1.clone(),
                upstream: None,
            },
            BranchInfo {
                refname: "refs/heads/b2".to_string(),
                oid: c2.clone(),
                upstream: None,
            },
        ];

        let (parents, children) = build_branch_graph(&repo, &active_branches).unwrap();

        assert_eq!(parents.len(), 1);
        assert_eq!(parents["refs/heads/b2"], "refs/heads/b1");
        assert_eq!(children["refs/heads/b1"], vec!["refs/heads/b2"]);
    }

    #[test]
    fn test_find_roots() {
        let active_branches = vec![
            BranchInfo {
                refname: "b1".to_string(),
                oid: "1".to_string(),
                upstream: None,
            },
            BranchInfo {
                refname: "b2".to_string(),
                oid: "2".to_string(),
                upstream: None,
            },
        ];
        let mut parents = HashMap::new();
        parents.insert("b2".to_string(), "b1".to_string());

        let roots = find_roots(&active_branches, &parents);
        assert_eq!(roots, vec!["b1".to_string()]);
    }

    #[test]
    fn test_collect_family() {
        let mut children = HashMap::new();
        children.insert("b1".to_string(), vec!["b2".to_string(), "b3".to_string()]);
        children.insert("b2".to_string(), vec!["b4".to_string()]);

        let family = collect_family("b1", &children);
        assert!(family.contains(&"b1".to_string()));
        assert!(family.contains(&"b2".to_string()));
        assert!(family.contains(&"b3".to_string()));
        assert!(family.contains(&"b4".to_string()));
        assert_eq!(family.len(), 4);
    }
}
