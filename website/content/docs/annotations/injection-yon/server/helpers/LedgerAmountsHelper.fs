[<Helper>]
[<Lifetime("singleton")>]
type LedgerAmountsHelper() =
    member this.Subtotal() : int =
        40

    member this.Surcharge() : int =
        2
