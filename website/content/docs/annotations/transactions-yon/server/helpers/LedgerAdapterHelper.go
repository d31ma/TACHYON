import "fmt"

// @Helper
type LedgerAdapterHelper struct {
	sequence int
	commands []string
}

func (adapter *LedgerAdapterHelper) Commands() []string {
	return adapter.commands
}

func (adapter *LedgerAdapterHelper) Transaction(command *BehaviorTransactionCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	entry := command.Action
	if command.Transaction != "" {
		entry = command.Action + ":" + command.Transaction
	}
	adapter.commands = append(adapter.commands, entry)
	adapter.sequence++
	return fmt.Sprintf("t%d", adapter.sequence), nil
}

func (adapter *LedgerAdapterHelper) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
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
	row := map[string]any{"id": 1, "status": "open"}
	return []any{row}, nil
}

func (adapter *LedgerAdapterHelper) invalid(message string) error {
	return &BehaviorError{Code: "invalid", Message: message}
}
