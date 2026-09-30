[<Helper>]
type TaxHelper(rates: TaxRateHelper) =
    member this.AddTax(subtotal: int) : int =
        let tax = rates.Rate()
        subtotal + tax
