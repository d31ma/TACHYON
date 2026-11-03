import "errors"

// @RequestRoute("/report")
// @Public
type ReportController struct {
	// @Needs
	reportClient *ReportClient
}

// @GetRoute
func (controller *ReportController) GetReport() (Report, error) {
	summary, summaryErr := controller.reportClient.Summarize("2026-Q3")
	forecast, forecastErr := controller.reportClient.Forecast(2)
	return Report{Summary: summary, Forecast: forecast}, errors.Join(summaryErr, forecastErr)
}
