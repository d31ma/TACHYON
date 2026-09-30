[<Service>]
type CardService() =
    member this.Name : string = "Anvil"
    member val Status : string = "waiting" with get, set

    [<OnStart>]
    member this.Activate() : unit =
        this.Status <- "active"
