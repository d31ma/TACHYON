[<Service>]
type AuditService() =
    [<Needs>]
    member val Ledger : CartDelegate = Unchecked.defaultof<CartDelegate> with get, set

    member this.Lines() : int = this.Ledger.Count()
