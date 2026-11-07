open Models

[<Service>]
type BasketService() =
    let mutable selected : string = ""

    member val Count : int = 0 with get, set

    member val Label : string = "" with get, set

    member val Tabs : Tab list = [ { Id = "a"; Name = "A" } ] with get, set

    member this.Increment() : unit =
        this.Count <- this.Count + 1

    member this.Select(id: string) : unit =
        selected <- id
