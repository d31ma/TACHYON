#[derive(Default)]
#[Delegate]
struct MemoryCacheDelegate {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorCacheEntry>>,
}
impl BehaviorCacheAdapter for MemoryCacheDelegate {
    fn get(&self, key: &str, _execution: &BehaviorExecution) -> BehaviorResult<Option<BehaviorCacheEntry>> {
        let entries = self.entries.lock().expect("read the cache");
        Ok(entries.get(key).cloned())
    }

    fn set(&self, key: &str, value: BehaviorValue, write: &BehaviorCacheWrite) -> BehaviorResult<()> {
        let entry = BehaviorCacheEntry { value, expires_at: write.expires_at };
        self.entries.lock().expect("write the cache").insert(key.to_owned(), entry);
        Ok(())
    }

    fn delete(&self, key: &str, _execution: &BehaviorExecution) -> BehaviorResult<()> {
        self.entries.lock().expect("clear the cache").remove(key);
        Ok(())
    }
}

impl MemoryCacheDelegate {
    fn new() -> Self { Self::default() }
}
