import "net/http"

// @RequestRoute("/report")
// @Public
type ReportController struct {
}

// @GetRoute
func (controller *ReportController) GetReport(request *http.Request) *http.Response {
	return Relay([]string{"perl", "server/relays/report.pl"}, request)
}
