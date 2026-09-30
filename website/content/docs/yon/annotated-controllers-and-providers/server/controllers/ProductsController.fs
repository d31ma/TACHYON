open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductsController() =
    [<Needs>]
    [<Named("ProductsService")>]
    member val Catalog : ProductsService = Unchecked.defaultof<ProductsService> with get, set

    [<GetRoute>]
    member this.GetProducts() : Product list =
        this.Catalog.List()

    [<GetRoute("/{id}")>]
    member this.GetProduct([<FromPath>] id: int64) : Product =
        this.Catalog.Find(id)
