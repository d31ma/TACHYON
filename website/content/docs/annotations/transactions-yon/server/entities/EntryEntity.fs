[<Entity({"adapter":"ledger"})>]
[<Table("entries")>]
type EntryEntity() =
    [<Id>]
    member val id : int64 = 0L with get, set
