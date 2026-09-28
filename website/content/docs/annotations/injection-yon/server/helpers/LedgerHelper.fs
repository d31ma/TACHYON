[<Helper>]
[<Lifetime("transient")>]
type LedgerHelper(ledgerStore: LedgerStore) =
    member this.Settle(subtotal: int) : int =
        subtotal + ledgerStore.Surcharge()
