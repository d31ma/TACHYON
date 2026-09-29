// @RequestRoute("/ledger")
// @Public
type LedgerController struct {
	// @Needs
	ledger *LedgerService
}

// @GetRoute
func (controller *LedgerController) GetLedger() (LedgerReport, error) {
	settled, err := controller.ledger.Settle("open")
	return LedgerReport{Settled: settled, Commands: controller.ledger.Commands()}, controller.asResourceError(err)
}

func (controller *LedgerController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
