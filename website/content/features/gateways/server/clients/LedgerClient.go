// @HTTPClient
type LedgerClient interface {
	// @Gateway({"class":"server/services/LedgerService.py","method":"balance"})
	Balance(account BehaviorValue) (BehaviorValue, error)

	// @Gateway({"command":["python3","server/helpers/forecast.py"]})
	Forecast(months BehaviorValue) (BehaviorValue, error)
}
