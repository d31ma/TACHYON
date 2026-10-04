[<Service>]
[<Lifetime({"value":"prototype"})>]
type CartService() =
    let prices = new CartPricesHelper()
    let cart = new CartHelper()

    member this.Total() : int =
        let subtotal = prices.Subtotal()
        cart.Settle(subtotal)
