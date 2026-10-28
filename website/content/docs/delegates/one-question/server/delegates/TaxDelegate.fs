[<Delegate>]
type TaxDelegate(rates: TaxRateDelegate) =
    member this.AddTax(subtotal: int) : int =
        let tax = rates.Rate()
        subtotal + tax
