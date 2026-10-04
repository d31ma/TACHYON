[<Service>]
type AuditService() =
    let ledger = new CartHelper()

    member this.Lines() : int = ledger.Count()
