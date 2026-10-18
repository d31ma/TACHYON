[<Service>]
type AuditService() =
    [<Needs>]
    member val Ledger : CartHelper = Unchecked.defaultof<CartHelper> with get, set

    member this.Lines() : int = this.Ledger.Count()
