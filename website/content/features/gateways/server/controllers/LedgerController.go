import "errors"

// @RequestRoute("/ledger")
// @Public
type LedgerController struct {
	// @Needs
	ledgerClient *LedgerClient
}

// @GetRoute
func (controller *LedgerController) GetLedger() (Balance, error) {
	balance, balanceErr := controller.ledgerClient.Balance("ACME-7")
	forecast, forecastErr := controller.ledgerClient.Forecast(1)
	return Balance{Balance: balance, Forecast: forecast}, errors.Join(balanceErr, forecastErr)
}
