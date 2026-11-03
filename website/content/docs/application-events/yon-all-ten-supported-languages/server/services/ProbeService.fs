open Models

[<Service>]
type ProbeService() =
    member val Last : ProbeReady option = None with get, set

    [<OnEvent>]
    member this.Observe(ready: ProbeReady) : unit =
        this.Last <- Some ready

    [<OnStop>]
    member this.Destroy() : unit =
        this.Last <- None
