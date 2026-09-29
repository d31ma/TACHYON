open System.Threading.Tasks

[<Store(typeof<BillingEntity>, {"fields":["id"],"defaultSize":20,"maxSize":100})>]
type BillingStore =
    abstract existsById: id: BehaviorJSON -> Task<BehaviorJSON>
