open Models

[<RequestRoute("/billing")>]
[<Public>]
type BillingController(billingService: BillingService) =
    [<GetRoute>]
    member this.GetBilling() : Bill =
        { Total = billingService.Total() }
