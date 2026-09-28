[RequestRoute("/report")]
[Public]
class ReportController {
    [GetRoute]
    public HttpResponseMessage GetReport(HttpRequestMessage request) {
        return Yon.Relay(new[] { "ruby", "server/helpers/ReportDelegate.rb" }, request);
    }
}
