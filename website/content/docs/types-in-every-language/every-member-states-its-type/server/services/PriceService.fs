open Models

[<Service>]
type PriceService() =
    static member private Currency : string = "EUR"

    member this.PriceFor(amount: int, count: int) : Price =
        { Amount = amount * count; Currency = PriceService.Currency }
