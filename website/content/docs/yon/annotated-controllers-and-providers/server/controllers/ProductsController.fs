open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductsController(catalog: ProductsService) =
    [<GetRoute>]
    member this.GetProducts() : Product list =
        catalog.List()

    [<GetRoute("/{id}")>]
    member this.GetProduct([<FromPath>] id: int64) : Product =
        catalog.Find(id)
