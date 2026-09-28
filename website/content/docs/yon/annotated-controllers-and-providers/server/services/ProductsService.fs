open Models

[<Service>]
[<Lifetime("transient")>]
type ProductsService() =
    member this.List() : Product list =
        [ this.Find(1L) ]

    member this.Find(id: int64) : Product =
        { Id = id; Name = "Anvil" }
