open System.Net.Http

[<RequestRoute("/report")>]
[<Public>]
type ReportController() =
    [<GetRoute>]
    member this.GetReport(request: HttpRequestMessage) : HttpResponseMessage =
        Yon.Relay([| "ruby"; "server/helpers/ReportDelegate.rb" |], request)
