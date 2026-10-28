open Models

[<RequestRoute("/checkout")>]
[<Public>]
type CheckoutController(service: CheckoutService) =
    [<GetRoute>]
    member this.GetCheckout() : Receipt =
        let total = service.Total()
        { Total = total }
