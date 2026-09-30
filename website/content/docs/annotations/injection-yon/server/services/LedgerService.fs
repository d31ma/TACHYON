[<Service>]
type LedgerService() =
    [<Needs>]
    member val Amounts : LedgerAmountsHelper = Unchecked.defaultof<LedgerAmountsHelper> with get, set

    member this.Subtotal() : int =
        this.Amounts.Subtotal()
