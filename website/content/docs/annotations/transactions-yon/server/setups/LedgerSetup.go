// @Setup
type LedgerSetup struct{}

func (LedgerSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	configuration := NewBehaviorConfiguration()
	configuration.Entities["ledger"] = &LedgerAdapterDelegate{}
	configuration.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		return true, nil
	}
	return configuration, nil
}
