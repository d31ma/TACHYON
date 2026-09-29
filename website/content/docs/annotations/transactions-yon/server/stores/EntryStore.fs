open System.Threading.Tasks

[<Store(typeof<EntryEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})>]
type EntryStore =
    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    abstract countByStatus: status: BehaviorJSON -> Task<BehaviorJSON>
