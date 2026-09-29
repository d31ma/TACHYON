[<Helper>]
[<Lifetime("transient")>]
type LedgerHelper(ledgerAmountsHelper: LedgerAmountsHelper) =
    member this.Settle(subtotal: int) : int =
        subtotal + ledgerAmountsHelper.Surcharge()
