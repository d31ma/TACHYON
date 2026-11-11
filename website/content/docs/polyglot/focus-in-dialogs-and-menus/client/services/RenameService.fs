[<Service>]
type RenameService() =
    let focus = Focus()
    member val Editing : bool = false with get, set
    member val Problem : string = "" with get, set

    member this.Open() : unit =
        this.Editing <- true
        this.KeepProblem(fun () -> focus.Trap("dialog"))
        this.KeepProblem(fun () -> focus.Move("name"))

    member this.Close() : unit =
        this.Editing <- false
        this.KeepProblem(fun () -> focus.Release())

    [<OnEvent>]
    member this.Failed(failed: BrowserCallFailed) : unit =
        this.Problem <- failed.Message

    member private this.KeepProblem(call: unit -> unit) : unit =
        try
            call ()
        with :? System.InvalidOperationException as refusal ->
            this.Problem <- refusal.Message
