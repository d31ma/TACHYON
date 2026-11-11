[<Service>]
type MenuService() =
    member val Expanded : bool = false with get, set

    member this.Toggle() : unit =
        this.Expanded <- not this.Expanded

    member this.Close() : unit =
        this.Expanded <- false
