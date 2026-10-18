open Models

[<HTTPClient("/ticks")>]
type TicksClient =
    [<StreamRequest>]
    abstract Receive: Tick -> unit
