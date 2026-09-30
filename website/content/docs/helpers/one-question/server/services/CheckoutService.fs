[<Service>]
type CheckoutService(helper: TaxHelper) =
    member this.Total() : int =
        let subtotal = this.Subtotal()
        helper.AddTax(subtotal)

    member private this.Subtotal() : int =
        40
