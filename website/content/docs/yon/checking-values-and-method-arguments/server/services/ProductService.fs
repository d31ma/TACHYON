[<Service>]
type ProductService() =
    member this.accept([<Check(typeof<ProductInput>)>] input: BehaviorJSON) : BehaviorJSON =
        input
