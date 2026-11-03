[<Delegate>]
[<Lifetime("singleton")>]
type LedgerAmountsDelegate() =
    member this.Subtotal() : int =
        40

    member this.Surcharge() : int =
        2
