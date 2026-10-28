[<Service>]
type BillingService(billingDelegate: BillingDelegate) =
    static member private Subtotal = 40

    member this.Total() : int =
        billingDelegate.Settle(BillingService.Subtotal)
