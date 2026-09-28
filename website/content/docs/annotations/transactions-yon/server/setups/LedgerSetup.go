// @Setup
type LedgerSetup struct{}

func (LedgerSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration := NewBehaviorConfiguration()
	configuration.Entities["ledger"] = &LedgerAdapterHelper{}
	configuration.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		return true, nil
	}
	return configuration, nil
}
