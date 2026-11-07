open System.Threading.Tasks

[<Store(typeof<EntryEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})>]
type EntryStore =
    inherit Repository<EntryEntity, int64>

    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    abstract countByStatus: status: BehaviorJSON -> Task<BehaviorJSON>
