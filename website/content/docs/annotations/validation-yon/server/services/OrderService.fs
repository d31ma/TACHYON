[<Service>]
type OrderService() =
    member this.accept([<Check(typeof<OrderInput>)>] order: BehaviorJSON) : BehaviorJSON = order
