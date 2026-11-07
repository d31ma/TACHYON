open System.Threading.Tasks

[<Service>]
type LedgerService(entries: EntryStore) =
    member val behaviors : BehaviorContext = Behaviors.create(typeof<LedgerSetup>, "ledger", new BehaviorSignal())

    [<AllOrNothing({"adapter":"ledger"})>]
    member this.settle(status: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            let! found = entries.findByStatus(status)
            let! total = entries.countByStatus(status)
            return BehaviorJSON.Object(dict [ "open", found; "total", total ])
        }
