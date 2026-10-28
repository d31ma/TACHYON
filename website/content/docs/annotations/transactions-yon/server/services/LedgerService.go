import "errors"

// @Service
type LedgerService struct {
	behaviors *BehaviorContext
	// @Needs
	entries *EntryStore
}

// @OnStart
func (service *LedgerService) Open() error {
	options := NewBehaviorOptions("settlement", NewBehaviorSignal())
	context, err := BehaviorsCreate[LedgerSetup](options)
	service.behaviors = context
	return err
}

// @AllOrNothing({"adapter":"ledger"})
func (service *LedgerService) Settle(status BehaviorValue) (BehaviorValue, error) {
	open, openErr := service.entries.FindByStatus(status)
	total, totalErr := service.entries.CountByStatus(status)
	return map[string]any{"open": open, "total": total}, errors.Join(openErr, totalErr)
}

func (service *LedgerService) Commands() []string {
	return service.behaviors.configuration.Entities["ledger"].(*LedgerAdapterDelegate).Commands()
}
