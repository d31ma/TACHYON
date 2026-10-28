[<Service>]
type LedgerService() =
    [<Needs>]
    member val Amounts : LedgerAmountsDelegate = Unchecked.defaultof<LedgerAmountsDelegate> with get, set

    member this.Subtotal() : int =
        this.Amounts.Subtotal()
