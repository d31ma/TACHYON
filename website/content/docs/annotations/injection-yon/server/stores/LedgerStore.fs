[<Store>]
[<Lifetime("singleton")>]
type LedgerStore() =
    member this.Subtotal() : int =
        40

    member this.Surcharge() : int =
        2
