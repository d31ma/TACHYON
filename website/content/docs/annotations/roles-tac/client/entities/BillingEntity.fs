[<Entity({"adapter":"publicBilling"})>]
[<Table("invoices")>]
type BillingEntity() =
    [<Id>]
    member val id : int64 = 0L with get, set
