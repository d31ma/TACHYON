import "fmt"

// @Delegate
type LedgerAdapterDelegate struct {
	sequence int
	commands []string
}

func (adapter *LedgerAdapterDelegate) Commands() []string {
	return adapter.commands
}

func (adapter *LedgerAdapterDelegate) Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	entry := command.Action
	if command.Transaction != "" {
		entry = command.Action + ":" + command.Transaction
	}
	adapter.commands = append(adapter.commands, entry)
	adapter.sequence++
	return fmt.Sprintf("t%d", adapter.sequence), nil
}

func (adapter *LedgerAdapterDelegate) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	query, isObject := command.Query.(map[string]any)
	if !isObject {
		return nil, adapter.invalid("The query is not valid.")
	}
	kind, isText := query["kind"].(string)
	if !isText {
		return nil, adapter.invalid("The query kind is not valid.")
	}
	transaction := command.Transaction
	if transaction == "" {
		transaction = "none"
	}
	adapter.commands = append(adapter.commands, kind+"@"+transaction)
	if kind == "count" {
		return 1, nil
	}
	row := map[string]any{"id": 1, "status": "open", "transaction": transaction}
	return []any{row}, nil
}

func (adapter *LedgerAdapterDelegate) invalid(message string) error {
	return &BehaviorError{Code: "invalid", Message: message}
}
