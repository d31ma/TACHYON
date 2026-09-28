// @RequestRoute("/ledger")
// @Public
type LedgerController struct{}

// @GetRoute
func (controller *LedgerController) GetLedger() (LedgerReport, error) {
	options := NewBehaviorOptions("ledger", NewBehaviorSignal())
	report, err := controller.reportFrom(BehaviorsCreate[LedgerSetup](options))
	return report, controller.asResourceError(err)
}

func (controller *LedgerController) reportFrom(context *BehaviorContext, failure error) (LedgerReport, error) {
	if failure != nil {
		return LedgerReport{}, failure
	}
	defer context.Dispose()
	entries := &EntryStore{behaviors: context}
	settled, failure := entries.Settle("open")
	adapter := context.configuration.Entities["ledger"].(*LedgerAdapterHelper)
	return LedgerReport{Settled: settled, Commands: adapter.Commands()}, failure
}

func (controller *LedgerController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
