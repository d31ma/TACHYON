open Models

[<RequestRoute("/tax")>]
[<Public>]
type TaxController(taxDelegate: TaxDelegate) =
    [<GetRoute>]
    member this.GetTax() : TaxAnswer =
        { Rate = taxDelegate.Rate("north")
          Total = taxDelegate.Total(3)
          Extra = taxDelegate.Total(3, 4) }
