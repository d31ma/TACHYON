[<Delegate>]
type BillingDelegate() =
    member this.Settle(subtotal: int) : int =
        subtotal + 2
