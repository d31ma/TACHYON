package main

type behaviorHostCache struct {
	name string
	host func(string) string
}

type behaviorHostEntities struct {
	host func(string) string
}

func behaviorBuiltinCache(name string) (BehaviorCacheAdapter, bool) {
	host := behaviorStorageHost()
	if host == nil {
		return nil, false
	}
	return &behaviorHostCache{name: name, host: host}, true
}

func behaviorBuiltinEntities(name string) (BehaviorEntityAdapter, bool) {
	host := behaviorStorageHost()
	if host == nil || name != "indexeddb" {
		return nil, false
	}
	return &behaviorHostEntities{host: host}, true
}

func behaviorAskHost(host func(string) string, request map[string]any) (BehaviorValue, error) {
	text, err := behaviorEncode(request, 1048576)
	if err != nil {
		return nil, err
	}
	answer, err := behaviorParseJSON(host(text))
	object, isObject := answer.(map[string]any)
	if err != nil || !isObject {
		return nil, behaviorError("storage", "Tachyon storage did not answer.")
	}
	if message, failed := object["error"].(string); failed {
		return nil, behaviorError("storage", message)
	}
	return object["value"], nil
}

func (cache *behaviorHostCache) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	answer, err := behaviorAskHost(cache.host, map[string]any{"op": "cache.get", "cache": cache.name, "key": key})
	if err != nil {
		return nil, err
	}
	entry, isObject := answer.(map[string]any)
	if !isObject || entry["hit"] != true {
		return nil, nil
	}
	expiresAt, _ := entry["expiresAt"].(float64)
	return &BehaviorCacheEntry{Value: entry["value"], ExpiresAt: int64(expiresAt)}, nil
}

func (cache *behaviorHostCache) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	_, err := behaviorAskHost(cache.host, map[string]any{
		"op":         "cache.set",
		"cache":      cache.name,
		"key":        key,
		"value":      value,
		"expiresAt":  write.ExpiresAt,
		"maxEntries": write.Policy.MaxEntries,
	})
	return err
}

func (cache *behaviorHostCache) Delete(key string, execution *BehaviorExecution) error {
	_, err := behaviorAskHost(cache.host, map[string]any{"op": "cache.delete", "cache": cache.name, "key": key})
	return err
}

func (entities *behaviorHostEntities) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	sent := map[string]any{
		"entity":    command.Entity,
		"table":     command.Table,
		"idField":   command.IdField,
		"operation": command.Operation,
		"value":     command.Value,
		"page":      command.Page,
		"id":        command.Id,
		"query":     command.Query,
	}
	if command.Transaction != "" {
		sent["transaction"] = command.Transaction
	}
	return behaviorAskHost(entities.host, map[string]any{"op": "entity", "command": sent})
}

func (entities *behaviorHostEntities) Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	sent := map[string]any{"operation": "transaction", "action": command.Action}
	if command.Transaction != "" {
		sent["transaction"] = command.Transaction
	}
	answer, err := behaviorAskHost(entities.host, map[string]any{"op": "entity", "command": sent})
	if err != nil || command.Action != "begin" {
		return answer, err
	}
	begun, _ := answer.(map[string]any)
	return begun["transaction"], nil
}
