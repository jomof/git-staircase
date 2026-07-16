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
    pub fn into_bool(self) -> Option<bool> {
        match self {
            MemoValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub fn into_text(self) -> Option<String> {
        match self {
            MemoValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_opt_text(self) -> Option<Option<String>> {
        match self {
            MemoValue::Text(s) => Some(Some(s)),
            MemoValue::NoneText => Some(None),
            _ => None,
        }
    }

    pub fn from_bool(b: bool) -> Self {
        MemoValue::Bool(b)
    }

    pub fn from_text(s: String) -> Self {
        MemoValue::Text(s)
    }

    pub fn from_opt_text(s: Option<String>) -> Self {
        match s {
            Some(s) => MemoValue::Text(s),
            None => MemoValue::NoneText,
        }
    }
}

/// Abstract store for immutable operation memoization.
pub trait MemoizationStore: Send + Sync + Debug {
    fn get(&self, key: &MemoKey) -> Option<MemoValue>;
    fn put(&self, key: MemoKey, value: MemoValue);
    fn clear(&self);
}

/// Thread-safe in-process memoization store.
#[derive(Debug, Default, Clone)]
pub struct InProcessMemoStore {
    cache: Arc<Mutex<HashMap<MemoKey, MemoValue>>>,
}

impl InProcessMemoStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl MemoizationStore for InProcessMemoStore {
    fn get(&self, key: &MemoKey) -> Option<MemoValue> {
        if let Ok(guard) = self.cache.lock() {
            guard.get(key).cloned()
        } else {
            None
        }
    }

    fn put(&self, key: MemoKey, value: MemoValue) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.insert(key, value);
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
        }
    }

    pub fn with_store(store: Arc<dyn MemoizationStore>) -> Self {
        Self { store }
    }
}

/// Helper for a pending memoization operation.
pub struct MemoOp<'a, V> {
    memoizer: &'a Memoizer,
    key: MemoKey,
    from_value: fn(MemoValue) -> Option<V>,
    to_value: fn(V) -> MemoValue,
}

impl<'a, V: Clone> MemoOp<'a, V> {
    pub fn get(&self) -> Option<V> {
        self.memoizer.store.get(&self.key).and_then(self.from_value)
    }

    pub fn put(&self, value: V) {
        self.memoizer
            .store
            .put(self.key.clone(), (self.to_value)(value));
    }

    pub fn get_or_compute<F, E>(&self, f: F) -> Result<V, E>
    where
        F: FnOnce() -> Result<V, E>,
    {
        if let Some(val) = self.get() {
            return Ok(val);
        }
        let val = f()?;
        self.put(val.clone());
        Ok(val)
    }
}

macro_rules! define_memo_ops {
    ($($name:ident { $($field:ident : $ftype:ty),* } -> $vtype:ty : $from:path, $to:path);* $(;)?) => {
        impl Memoizer {
            $(
                #[allow(non_snake_case)]
                pub fn $name(&self, $($field: $ftype),*) -> MemoOp<'_, $vtype> {
                    MemoOp {
                        memoizer: self,
                        key: MemoKey::$name { $($field),* },
                        from_value: $from,
                        to_value: $to,
                    }
                }
            )*
        }
    };
}

define_memo_ops! {
    Ancestry { ancestor: String, descendant: String } -> bool : MemoValue::into_bool, MemoValue::from_bool;
    MergeBase { a: String, b: String } -> String : MemoValue::into_text, MemoValue::from_text;
    PatchId { base: String, tip: String } -> String : MemoValue::into_text, MemoValue::from_text;
    TreeId { commit: String } -> String : MemoValue::into_text, MemoValue::from_text;
    ObjectFormat {} -> String : MemoValue::into_text, MemoValue::from_text;
    ResolveCommit { rev: String } -> String : MemoValue::into_text, MemoValue::from_text;
    ResolveRef { rev: String } -> Option<String> : MemoValue::into_opt_text, MemoValue::from_opt_text;
    ResolveSymbolic { name: String } -> String : MemoValue::into_text, MemoValue::from_text;
}

impl Memoizer {
    pub fn hash_data(&self, data: &str) -> MemoOp<'_, String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let content_sha = format!("{:x}", hasher.finalize());
        MemoOp {
            memoizer: self,
            key: MemoKey::HashData { content_sha },
            from_value: MemoValue::into_text,
            to_value: MemoValue::from_text,
        }
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

        assert_eq!(
            memoizer.Ancestry("commitA".into(), "commitB".into()).get(),
            None
        );
        memoizer
            .Ancestry("commitA".into(), "commitB".into())
            .put(true);
        assert_eq!(
            memoizer.Ancestry("commitA".into(), "commitB".into()).get(),
            Some(true)
        );

        assert_eq!(memoizer.ObjectFormat().get(), None);
        memoizer.ObjectFormat().put("sha1".into());
        assert_eq!(memoizer.ObjectFormat().get(), Some("sha1".to_string()));

        assert_eq!(memoizer.PatchId("c1".into(), "c2".into()).get(), None);
        memoizer
            .PatchId("c1".into(), "c2".into())
            .put("patch123".into());
        assert_eq!(
            memoizer.PatchId("c1".into(), "c2".into()).get(),
            Some("patch123".to_string())
        );

        memoizer.clear();
        assert_eq!(
            memoizer.Ancestry("commitA".into(), "commitB".into()).get(),
            None
        );
        assert_eq!(memoizer.ObjectFormat().get(), None);
    }

    #[derive(Debug)]
    struct MockDaemonStore {
        calls: Arc<Mutex<Vec<MemoKey>>>,
        inner: InProcessMemoStore,
    }

    impl MemoizationStore for MockDaemonStore {
        fn get(&self, key: &MemoKey) -> Option<MemoValue> {
            if let Ok(mut c) = self.calls.lock() {
                c.push(key.clone());
            }
            self.inner.get(key)
        }

        fn put(&self, key: MemoKey, value: MemoValue) {
            self.inner.put(key, value);
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
        memoizer.TreeId("commit1".into()).put("tree1".into());
        assert_eq!(
            memoizer.TreeId("commit1".into()).get(),
            Some("tree1".to_string())
        );

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
