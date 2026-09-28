open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController(ledgerService: LedgerService, ledgerHelper: LedgerHelper) =
    [<GetRoute>]
    member this.GetLedger() : Ledger =
        { Total = ledgerHelper.Settle(ledgerService.Subtotal()) }
