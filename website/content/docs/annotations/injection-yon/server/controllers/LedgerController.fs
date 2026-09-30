open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController(ledgerService: LedgerService) =
    [<Needs>]
    [<Named("LedgerHelper")>]
    member val Settlement : LedgerHelper = Unchecked.defaultof<LedgerHelper> with get, set

    [<GetRoute>]
    member this.GetLedger() : Ledger =
        { Total = this.Settlement.Settle(ledgerService.Subtotal()) }
