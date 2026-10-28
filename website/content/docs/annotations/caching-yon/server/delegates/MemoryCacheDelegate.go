// @Delegate
type MemoryCacheDelegate struct {
	entries map[string]BehaviorCacheEntry
}

func (cache *MemoryCacheDelegate) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	entry, present := cache.entries[key]
	if !present {
		return nil, nil
	}
	return &entry, nil
}

func (cache *MemoryCacheDelegate) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	cache.entries[key] = BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (cache *MemoryCacheDelegate) Delete(key string, execution *BehaviorExecution) error {
	delete(cache.entries, key)
	return nil
}
