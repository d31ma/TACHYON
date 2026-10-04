[<Service>]
type BasketService() =
    let total : int = 1250

    member this.TotalLabel() : string =
        PriceUtils.Label(total)
