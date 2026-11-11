[<Schedule({"fixedDelaySeconds":300})>]
type Handler(ledger: LedgerService) =
    member this.POST() : int = ledger.Total()
