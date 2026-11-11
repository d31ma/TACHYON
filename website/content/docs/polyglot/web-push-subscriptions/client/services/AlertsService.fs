[<Service>]
type AlertsService() =
    static let serverKey = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U"
    let push = Push()
    member val Endpoint : string = "" with get, set
    member val Problem : string = "" with get, set

    member this.TurnOn() : unit = this.KeepProblem(fun () -> push.Subscribe(serverKey))

    member this.TurnOff() : unit = this.KeepProblem(fun () -> push.Unsubscribe())

    [<OnEvent>]
    member this.Subscribed(subscribed: PushSubscribed) : unit =
        this.Endpoint <- subscribed.Endpoint

    [<OnEvent>]
    member this.Unsubscribed(unsubscribed: PushUnsubscribed) : unit =
        this.Endpoint <- ""

    [<OnEvent>]
    member this.Failed(failed: BrowserCallFailed) : unit =
        this.Problem <- failed.Message

    member private this.KeepProblem(call: unit -> unit) : unit =
        try
            call ()
        with :? System.InvalidOperationException as refusal ->
            this.Problem <- refusal.Message
