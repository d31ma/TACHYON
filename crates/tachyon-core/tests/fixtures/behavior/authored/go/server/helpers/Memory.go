import "sync"

// @Description("Behavior conformance Memory")
type Memory struct {
	lock    sync.Mutex
	entries map[string]BehaviorCacheEntry
	log     []string
}

func (memory *Memory) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	memory.lock.Lock()
	defer memory.lock.Unlock()
	entry, present := memory.entries[key]
	if !present {
		return nil, nil
	}
	return &entry, nil
}

func (memory *Memory) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	if err := write.Execution.Check(); err != nil {
		return err
	}
	memory.lock.Lock()
	defer memory.lock.Unlock()
	memory.entries[key] = BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (memory *Memory) Delete(key string, execution *BehaviorExecution) error {
	if err := execution.Check(); err != nil {
		return err
	}
	memory.lock.Lock()
	defer memory.lock.Unlock()
	delete(memory.entries, key)
	return nil
}

func (memory *Memory) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	if request.Credentials != "omit" || request.Redirect != "error" {
		return nil, &BehaviorError{Code: "adapter", Message: "unsafe request defaults"}
	}
	return request.Path, nil
}

func (memory *Memory) Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	entry := command.Action
	if command.Transaction != "" {
		entry += ":" + command.Transaction
	}
	memory.record(entry)
	return "tx-1", nil
}

func (memory *Memory) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	transaction := command.Transaction
	if transaction == "" {
		transaction = "none"
	}
	memory.record(command.Operation + "@" + transaction)
	if command.Query == nil {
		return map[string]any{"operation": command.Operation, "table": command.Table}, nil
	}
	return memory.answerQuery(command.Query)
}

func (memory *Memory) TakeLog() []string {
	memory.lock.Lock()
	defer memory.lock.Unlock()
	entries := memory.log
	memory.log = nil
	return entries
}

func (memory *Memory) record(entry string) {
	memory.lock.Lock()
	defer memory.lock.Unlock()
	memory.log = append(memory.log, entry)
}

func (memory *Memory) answerQuery(query BehaviorValue) (BehaviorValue, error) {
	kind := query.(map[string]any)["kind"]
	switch kind {
	case "find":
		return []any{map[string]any{"id": "one", "status": "active"}}, nil
	case "count", "delete":
		return 1, nil
	case "exists":
		return true, nil
	}
	return nil, &BehaviorError{Code: "invalid", Message: "unknown query kind"}
}
