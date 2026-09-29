open System.Threading.Tasks

[<Store(typeof<EntryEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100})>]
type EntryStore(context: BehaviorContext) =
    member val behaviors : BehaviorContext = context

    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    abstract countByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    [<AllOrNothing({"adapter":"ledger"})>]
    member this.settle(status: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            let! found = this.findByStatus(status)
            let! total = this.countByStatus(status)
            return BehaviorJSON.Object(dict [ "open", found; "total", total ])
        }
