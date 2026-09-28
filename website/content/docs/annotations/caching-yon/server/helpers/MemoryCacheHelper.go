// @Helper
type MemoryCacheHelper struct {
	entries map[string]BehaviorCacheEntry
}

func (cache *MemoryCacheHelper) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	entry, present := cache.entries[key]
	if !present {
		return nil, nil
	}
	return &entry, nil
}

func (cache *MemoryCacheHelper) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	cache.entries[key] = BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (cache *MemoryCacheHelper) Delete(key string, execution *BehaviorExecution) error {
	delete(cache.entries, key)
	return nil
}
