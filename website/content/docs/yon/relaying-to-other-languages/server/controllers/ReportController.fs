open System.Net.Http

[<RequestRoute("/report")>]
[<Public>]
type ReportController() =
    [<GetRoute>]
    member this.GetReport(request: HttpRequestMessage) : HttpResponseMessage =
        Yon.Relay([| "perl"; "server/relays/report.pl" |], request)
