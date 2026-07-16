use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// Keys representing immutable operations that can be memoized.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoKey {
    /// Ancestry reachability check between two commit OIDs.
    Ancestry {
        ancestor: String,
        descendant: String,
    },
    /// Merge base between two commit OIDs.
    MergeBase { a: String, b: String },
    /// Patch ID for a diff between two commit OIDs.
    PatchId { base: String, tip: String },
    /// Tree OID for a given commit OID.
    TreeId { commit: String },
    /// Repository object format (e.g., "sha1" or "sha256").
    ObjectFormat,
    /// Git object hash for raw string content.
    HashData { content_sha: String },
    /// Resolved commit OID for a given revision string.
    ResolveCommit { rev: String },
    /// Resolved ref OID (or None if missing) for a revision string.
    ResolveRef { rev: String },
    /// Full refname for a symbolic branch or ref name.
    ResolveSymbolic { name: String },
}

/// Serialized/typed memoized value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoValue {
    Bool(bool),
    Text(String),
    NoneText,
}

impl MemoValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MemoValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            MemoValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Abstract store for immutable operation memoization.
///
/// Implementations range from per-command in-process thread-safe hash maps (default)
/// to a shared out-of-process daemon memoizer communicating over IPC/sockets.
pub trait MemoizationStore: Send + Sync + Debug {
    fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue>;
    fn put(&self, namespace: &str, key: MemoKey, value: MemoValue);
    fn clear(&self);
}

/// Thread-safe in-process memoization store.
#[derive(Debug, Default, Clone)]
pub struct InProcessMemoStore {
    cache: Arc<Mutex<HashMap<(String, MemoKey), MemoValue>>>,
}

impl InProcessMemoStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl MemoizationStore for InProcessMemoStore {
    fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue> {
        if let Ok(guard) = self.cache.lock() {
            guard.get(&(namespace.to_string(), key.clone())).cloned()
        } else {
            None
        }
    }

    fn put(&self, namespace: &str, key: MemoKey, value: MemoValue) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.insert((namespace.to_string(), key), value);
        }
    }

    fn clear(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.clear();
        }
    }
}

/// Primary interface for git-staircase operations to memoize immutable values.
#[derive(Debug, Clone)]
pub struct Memoizer {
    store: Arc<dyn MemoizationStore>,
    pub namespace: String,
}

impl Default for Memoizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Memoizer {
    pub fn new() -> Self {
        Self {
            store: Arc::new(InProcessMemoStore::new()),
            namespace: String::new(),
        }
    }

    pub fn with_store(store: Arc<dyn MemoizationStore>) -> Self {
        Self { store, namespace: String::new() }
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
        self
    }

    pub fn get_ancestry(&self, ancestor: &str, descendant: &str) -> Option<bool> {
        let key = MemoKey::Ancestry {
            ancestor: ancestor.to_string(),
            descendant: descendant.to_string(),
        };
        self.store.get(&self.namespace, &key).and_then(|v| v.as_bool())
    }

    pub fn set_ancestry(&self, ancestor: &str, descendant: &str, is_ancestor: bool) {
        let key = MemoKey::Ancestry {
            ancestor: ancestor.to_string(),
            descendant: descendant.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Bool(is_ancestor));
    }

    pub fn get_merge_base(&self, a: &str, b: &str) -> Option<String> {
        let key = MemoKey::MergeBase {
            a: a.to_string(),
            b: b.to_string(),
        };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_merge_base(&self, a: &str, b: &str, result: &str) {
        let key = MemoKey::MergeBase {
            a: a.to_string(),
            b: b.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Text(result.to_string()));
    }

    pub fn get_patch_id(&self, base: &str, tip: &str) -> Option<String> {
        let key = MemoKey::PatchId {
            base: base.to_string(),
            tip: tip.to_string(),
        };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_patch_id(&self, base: &str, tip: &str, patch_id: &str) {
        let key = MemoKey::PatchId {
            base: base.to_string(),
            tip: tip.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Text(patch_id.to_string()));
    }

    pub fn get_tree_id(&self, commit: &str) -> Option<String> {
        let key = MemoKey::TreeId {
            commit: commit.to_string(),
        };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_tree_id(&self, commit: &str, tree_id: &str) {
        let key = MemoKey::TreeId {
            commit: commit.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Text(tree_id.to_string()));
    }

    pub fn get_object_format(&self) -> Option<String> {
        let key = MemoKey::ObjectFormat;
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_object_format(&self, format: &str) {
        let key = MemoKey::ObjectFormat;
        self.store.put(&self.namespace, key, MemoValue::Text(format.to_string()));
    }

    pub fn get_hash_data(&self, data: &str) -> Option<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let content_sha = format!("{:x}", hasher.finalize());
        let key = MemoKey::HashData { content_sha };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_hash_data(&self, data: &str, hash: &str) {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let content_sha = format!("{:x}", hasher.finalize());
        let key = MemoKey::HashData { content_sha };
        self.store.put(&self.namespace, key, MemoValue::Text(hash.to_string()));
    }

    pub fn get_resolve_commit(&self, rev: &str) -> Option<String> {
        let key = MemoKey::ResolveCommit {
            rev: rev.to_string(),
        };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_resolve_commit(&self, rev: &str, oid: &str) {
        let key = MemoKey::ResolveCommit {
            rev: rev.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Text(oid.to_string()));
    }

    pub fn get_resolve_ref(&self, rev: &str) -> Option<Option<String>> {
        let key = MemoKey::ResolveRef {
            rev: rev.to_string(),
        };
        match self.store.get(&self.namespace, &key) {
            Some(MemoValue::Text(s)) => Some(Some(s)),
            Some(MemoValue::NoneText) => Some(None),
            _ => None,
        }
    }

    pub fn set_resolve_ref(&self, rev: &str, oid: Option<&str>) {
        let key = MemoKey::ResolveRef {
            rev: rev.to_string(),
        };
        let val = match oid {
            Some(s) => MemoValue::Text(s.to_string()),
            None => MemoValue::NoneText,
        };
        self.store.put(&self.namespace, key, val);
    }

    pub fn get_symbolic_name(&self, name: &str) -> Option<String> {
        let key = MemoKey::ResolveSymbolic {
            name: name.to_string(),
        };
        self.store
            .get(&self.namespace, &key)
            .and_then(|v| v.as_text().map(String::from))
    }

    pub fn set_symbolic_name(&self, name: &str, full_name: &str) {
        let key = MemoKey::ResolveSymbolic {
            name: name.to_string(),
        };
        self.store.put(&self.namespace, key, MemoValue::Text(full_name.to_string()));
    }

    pub fn clear(&self) {
        self.store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_process_memo_store() {
        let memoizer = Memoizer::new();

        assert_eq!(memoizer.get_ancestry("commitA", "commitB"), None);
        memoizer.set_ancestry("commitA", "commitB", true);
        assert_eq!(memoizer.get_ancestry("commitA", "commitB"), Some(true));

        assert_eq!(memoizer.get_object_format(), None);
        memoizer.set_object_format("sha1");
        assert_eq!(memoizer.get_object_format(), Some("sha1".to_string()));

        assert_eq!(memoizer.get_patch_id("c1", "c2"), None);
        memoizer.set_patch_id("c1", "c2", "patch123");
        assert_eq!(
            memoizer.get_patch_id("c1", "c2"),
            Some("patch123".to_string())
        );

        memoizer.clear();
        assert_eq!(memoizer.get_ancestry("commitA", "commitB"), None);
        assert_eq!(memoizer.get_object_format(), None);
    }

    #[derive(Debug)]
    struct MockDaemonStore {
        calls: Arc<Mutex<Vec<MemoKey>>>,
        inner: InProcessMemoStore,
    }

    impl MemoizationStore for MockDaemonStore {
        fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue> {
            if let Ok(mut c) = self.calls.lock() {
                c.push(key.clone());
            }
            self.inner.get(namespace, key)
        }

        fn put(&self, namespace: &str, key: MemoKey, value: MemoValue) {
            self.inner.put(namespace, key, value)
        }

        fn clear(&self) {
            self.inner.clear();
        }
    }

    #[test]
    fn test_custom_store_pluggability() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mock_daemon = Arc::new(MockDaemonStore {
            calls: calls.clone(),
            inner: InProcessMemoStore::new(),
        });

        let memoizer = Memoizer::with_store(mock_daemon);
        memoizer.set_tree_id("commit1", "tree1");
        assert_eq!(memoizer.get_tree_id("commit1"), Some("tree1".to_string()));

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0],
            MemoKey::TreeId {
                commit: "commit1".to_string()
            }
        );
    }
}
