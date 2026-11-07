[<Service>]
type PlaceService() =
    let geolocation = Geolocation()
    let clipboard = Clipboard()
    member val Place : string = "" with get, set
    member val Problem : string = "" with get, set

    member this.Locate() : unit = this.KeepProblem(fun () -> geolocation.CurrentPosition())

    member this.Copy() : unit = this.KeepProblem(fun () -> clipboard.WriteText(this.Place))

    [<OnEvent>]
    member this.Found(found: PositionFound) : unit =
        this.Place <- string found.Latitude + ", " + string found.Longitude

    [<OnEvent>]
    member this.Failed(failed: BrowserCallFailed) : unit =
        this.Problem <- failed.Message

    member private this.KeepProblem(call: unit -> unit) : unit =
        try
            call ()
        with :? System.InvalidOperationException as refusal ->
            this.Problem <- refusal.Message
