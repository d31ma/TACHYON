[<Helper>]
type CartHelper() =
    let prices = new CartPricesHelper()

    member this.Settle(subtotal: int) : int =
        let surcharge = prices.Surcharge()
        subtotal + surcharge

    member this.Count() : int = prices.Count()
