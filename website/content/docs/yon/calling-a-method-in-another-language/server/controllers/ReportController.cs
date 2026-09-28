using System.Text.Json.Nodes;
using Models;

[RequestRoute("/report")]
[Public]
class ReportController {
    [Needs]
    readonly ReportClient _reportClient;

    [GetRoute]
    public Report GetReport() {
        var summary = _reportClient.Summarize(BehaviorJSON.From("2026-Q3"));
        var forecast = _reportClient.Forecast(BehaviorJSON.From(2));
        return new Report(JsonNode.Parse(summary.ToString()), JsonNode.Parse(forecast.ToString()));
    }
}
