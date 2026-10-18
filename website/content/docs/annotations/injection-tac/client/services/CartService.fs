[<Service>]
[<Lifetime({"value":"prototype"})>]
type CartService(prices: CartPricesHelper, cart: CartHelper) =
    member this.Total() : int =
        let subtotal = prices.Subtotal()
        cart.Settle(subtotal)
