[<Service>]
type ReportService() =
    let windowControls = WindowControls()
    member val Problem : string = "" with get, set

    member this.Minimize() : unit = this.KeepProblem(fun () -> windowControls.Minimize())

    member this.Resize() : unit = this.KeepProblem(fun () -> windowControls.Resize(1280, 800))

    member this.Rename() : unit = this.KeepProblem(fun () -> windowControls.SetTitle("Quarterly report"))

    member private this.KeepProblem(call: unit -> unit) : unit =
        try
            call ()
        with :? System.InvalidOperationException as refusal ->
            this.Problem <- refusal.Message
