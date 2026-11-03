[RequestRoute("/report")]
[Public]
class ReportController {
    [GetRoute]
    public HttpResponseMessage GetReport(HttpRequestMessage request) {
        return Yon.Relay(new[] { "perl", "server/relays/report.pl" }, request);
    }
}
