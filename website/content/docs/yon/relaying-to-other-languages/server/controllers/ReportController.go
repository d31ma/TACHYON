import "net/http"

// @RequestRoute("/report")
// @Public
type ReportController struct {
}

// @GetRoute
func (controller *ReportController) GetReport(request *http.Request) *http.Response {
	return Relay([]string{"ruby", "server/helpers/ReportDelegate.rb"}, request)
}
