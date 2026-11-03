open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController(ledgerService: LedgerService) =
    [<Needs>]
    [<Named("LedgerDelegate")>]
    member val Settlement : LedgerDelegate = Unchecked.defaultof<LedgerDelegate> with get, set

    [<GetRoute>]
    member this.GetLedger() : Ledger =
        { Total = this.Settlement.Settle(ledgerService.Subtotal()) }
