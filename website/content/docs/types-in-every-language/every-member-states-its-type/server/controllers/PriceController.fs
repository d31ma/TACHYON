open Models

[<RequestRoute("/price")>]
[<Public>]
type PriceController(priceService: PriceService) =
    [<GetRoute>]
    member this.GetPrice() : Price =
        priceService.PriceFor(20, 3)
