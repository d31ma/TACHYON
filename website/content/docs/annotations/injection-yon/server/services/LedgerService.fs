[<Service>]
type LedgerService(ledgerAmountsHelper: LedgerAmountsHelper) =
    member this.Subtotal() : int =
        ledgerAmountsHelper.Subtotal()
