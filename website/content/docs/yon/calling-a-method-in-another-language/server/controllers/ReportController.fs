open System.Text.Json.Nodes
open Models

[<RequestRoute("/report")>]
[<Public>]
type ReportController(reportClient: ReportClient) =
    [<GetRoute>]
    member this.GetReport() : Report =
        let summary = reportClient.Summarize(BehaviorJSON.From("2026-Q3"))
        let forecast = reportClient.Forecast(BehaviorJSON.From(2))
        { Summary = JsonNode.Parse(summary.ToString())
          Forecast = JsonNode.Parse(forecast.ToString()) }
