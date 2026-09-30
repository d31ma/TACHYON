// @HTTPClient
type ReportClient interface {
	// @Gateway({"class":"server/services/ReportService.py","method":"summarize"})
	Summarize(period BehaviorValue) (BehaviorValue, error)

	// @Gateway({"command":["python3","server/helpers/forecast.py"]})
	Forecast(months BehaviorValue) (BehaviorValue, error)
}
