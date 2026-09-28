[<Service>]
type LedgerService(ledgerStore: LedgerStore) =
    member this.Subtotal() : int =
        ledgerStore.Subtotal()
