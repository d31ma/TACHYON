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

    [<AllOrNothing({"adapter":"ledger"})>]
    member this.refuse(status: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            let! _ = entries.save(BehaviorJSON.Object(dict [ "id", BehaviorJSON.From(2); "status", status ]))
            return raise (System.InvalidOperationException("the ledger refused the entry"))
        }
