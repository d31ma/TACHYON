open Models

[<RequestRoute("/probe")>]
[<Public>]
type ProbeController(probeService: ProbeService, publishService: PublishService) =
    [<GetRoute>]
    member this.GetProbe() : ProbeReady option =
        probeService.Last
