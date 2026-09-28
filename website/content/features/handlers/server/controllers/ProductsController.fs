open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductsController() =
    [<GetRoute>]
    member this.GetProducts() : Product list =
        [ { Name = "anvil" } ]

    [<PostRoute>]
    [<ResponseStatus(201)>]
    member this.PostProduct([<FromBody>] product: Product) : Product =
        product
