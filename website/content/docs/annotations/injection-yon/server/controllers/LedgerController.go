// @RequestRoute("/ledger")
// @Public
type LedgerController struct {
	// @Needs
	// @Named("LedgerService")
	service *LedgerService
	// @Needs
	// @Named("LedgerDelegate")
	settlement *LedgerDelegate
}

// @GetRoute
func (controller *LedgerController) GetLedger() Ledger {
	subtotal := controller.service.Subtotal()
	total := controller.settlement.Settle(subtotal)
	return Ledger{Total: total}
}
