[<Service>]
type BillingService(billingHelper: BillingHelper) =
    static member private Subtotal = 40

    member this.Total() : int =
        billingHelper.Settle(BillingService.Subtotal)
