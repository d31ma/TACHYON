[<Entity({"adapter":"billing"})>]
[<Table("invoices")>]
type BillingEntity() =
    [<Id>]
    member val id : int64 = 0L with get, set
