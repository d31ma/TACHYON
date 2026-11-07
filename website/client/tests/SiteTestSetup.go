// @TestSetup
type SiteTestSetup struct{}

func (SiteTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return &BehaviorConfiguration{}, nil
}
