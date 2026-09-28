// @HTTPClient
type ReportClient struct{}

// @Gateway({"class":"server/services/ReportService.py","method":"summarize"})
func (client *ReportClient) Summarize(period BehaviorValue) (BehaviorValue, error) {}

// @Gateway({"command":["python3","server/helpers/forecast.py"]})
func (client *ReportClient) Forecast(months BehaviorValue) (BehaviorValue, error) {}
