open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductsController(productService: ProductService) =
    [<PostRoute>]
    member this.PostProduct([<FromBody>] product: Product) : Product =
        let input = dict [ "name", BehaviorJSON.From(product.Name); "level", BehaviorJSON.From(product.Level) ]
        productService.accept(BehaviorJSON.Object(input)) |> ignore
        product
