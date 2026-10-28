[<Service>]
[<Lifetime({"value":"prototype"})>]
type CartService(prices: CartPricesDelegate, cart: CartDelegate) =
    member this.Total() : int =
        let subtotal = prices.Subtotal()
        cart.Settle(subtotal)
