[<Service>]
type BillingService(billingStore: BillingStore, billingHelper: BillingHelper) =
    member this.Total() : int =
        billingHelper.Settle(billingStore.Subtotal())
