// @TestSetup
type CatalogTestSetup struct{}

func (CatalogTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
