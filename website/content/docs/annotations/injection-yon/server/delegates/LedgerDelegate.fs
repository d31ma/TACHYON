[<Delegate>]
[<Lifetime("transient")>]
type LedgerDelegate(ledgerAmountsDelegate: LedgerAmountsDelegate) =
    member this.Settle(subtotal: int) : int =
        subtotal + ledgerAmountsDelegate.Surcharge()
