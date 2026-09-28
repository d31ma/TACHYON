[<Helper>]
type BillingHelper() =
    member this.Settle(subtotal: int) : int =
        subtotal + 2
