import re

with open("src/memoization.rs", "r") as f:
    code = f.read()

# 1. Update MemoizationStore trait
code = re.sub(
    r"fn get\(&self, key: &MemoKey\) -> Option<MemoValue>;",
    r"fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue>;",
    code
)
code = re.sub(
    r"fn put\(&self, key: MemoKey, value: MemoValue\);",
    r"fn put(&self, namespace: &str, key: MemoKey, value: MemoValue);",
    code
)

# 2. Update InProcessMemoStore
code = re.sub(
    r"cache: Arc<Mutex<HashMap<MemoKey, MemoValue>>>,",
    r"cache: Arc<Mutex<HashMap<(String, MemoKey), MemoValue>>>,",
    code
)
code = re.sub(
    r"fn get\(&self, key: &MemoKey\) -> Option<MemoValue> \{\n\s*if let Ok\(guard\) = self.cache.lock\(\) \{\n\s*guard.get\(key\).cloned\(\)",
    r"fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue> {\n        if let Ok(guard) = self.cache.lock() {\n            guard.get(&(namespace.to_string(), key.clone())).cloned()",
    code
)
code = re.sub(
    r"fn put\(&self, key: MemoKey, value: MemoValue\) \{\n\s*if let Ok\(mut guard\) = self.cache.lock\(\) \{\n\s*guard.insert\(key, value\);",
    r"fn put(&self, namespace: &str, key: MemoKey, value: MemoValue) {\n        if let Ok(mut guard) = self.cache.lock() {\n            guard.insert((namespace.to_string(), key), value);",
    code
)

# 3. Update Memoizer struct
code = re.sub(
    r"pub struct Memoizer \{\n\s*store: Arc<dyn MemoizationStore>,\n\}",
    r"pub struct Memoizer {\n    store: Arc<dyn MemoizationStore>,\n    pub namespace: String,\n}",
    code
)

# 4. Update Memoizer::new and with_store
code = re.sub(
    r"pub fn new\(\) -> Self \{\n\s*Self \{\n\s*store: Arc::new\(InProcessMemoStore::new\(\)\),\n\s*\}",
    r"pub fn new() -> Self {\n        Self {\n            store: Arc::new(InProcessMemoStore::new()),\n            namespace: String::new(),\n        }",
    code
)
code = re.sub(
    r"pub fn with_store\(store: Arc<dyn MemoizationStore>\) -> Self \{\n\s*Self \{ store \}",
    r"pub fn with_store(store: Arc<dyn MemoizationStore>) -> Self {\n        Self { store, namespace: String::new() }",
    code
)

# 5. Add with_namespace
code = re.sub(
    r"pub fn with_store\(store: Arc<dyn MemoizationStore>\) -> Self \{\n\s*Self \{ store, namespace: String::new\(\) \}\n\s*\}",
    r"pub fn with_store(store: Arc<dyn MemoizationStore>) -> Self {\n        Self { store, namespace: String::new() }\n    }\n\n    pub fn with_namespace(mut self, namespace: String) -> Self {\n        self.namespace = namespace;\n        self\n    }",
    code
)

# 6. Update all self.store.get(&key) to self.store.get(&self.namespace, &key)
code = re.sub(
    r"\.get\(&key\)",
    r".get(&self.namespace, &key)",
    code
)
# Update all self.store.put(key, ...) to self.store.put(&self.namespace, key, ...)
code = re.sub(
    r"\.put\(key, (.*?)\)",
    r".put(&self.namespace, key, \1)",
    code
)

# 7. Update MockDaemonStore
code = re.sub(
    r"impl MemoizationStore for MockDaemonStore \{\n\s*fn get\(&self, key: &MemoKey\) -> Option<MemoValue> \{\n\s*self\.inner\.get\(&self\.namespace, &key\)",
    r"impl MemoizationStore for MockDaemonStore {\n        fn get(&self, namespace: &str, key: &MemoKey) -> Option<MemoValue> {\n            self.inner.get(namespace, key)",
    code
)
code = re.sub(
    r"fn put\(&self, key: MemoKey, value: MemoValue\) \{\n\s*self\.inner\.put\(&self\.namespace, key, value\)",
    r"fn put(&self, namespace: &str, key: MemoKey, value: MemoValue) {\n            self.inner.put(namespace, key, value)",
    code
)

with open("src/memoization.rs", "w") as f:
    f.write(code)
