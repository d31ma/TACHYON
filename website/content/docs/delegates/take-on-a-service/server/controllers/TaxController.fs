open Models

[<RequestRoute("/tax")>]
[<Public>]
type TaxController(taxDelegate: TaxDelegate) =
    static let count = 3

    [<GetRoute>]
    member this.GetTax() : TaxAnswer =
        { Rate = taxDelegate.Rate("north")
          Total = taxDelegate.Total(count)
          Extra = taxDelegate.Total(count, 4) }
