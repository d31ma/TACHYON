open Models

[<Service>]
type OrderService() =
    member val status : string = "ready" with get, set

    member this.accept([<Check(typeof<OrderInput>)>] order: BehaviorJSON) : BehaviorJSON = order
