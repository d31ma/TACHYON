[<Service>]
type MenuService() =
    member val Expanded = false with get, set

    member this.Toggle() : unit =
        this.Expanded <- not this.Expanded
