open Models

[<RequestRoute("/money")>]
[<Public>]
type MoneyController() =
    [<Needs>]
    member val Money : MoneyService = Unchecked.defaultof<MoneyService> with get, set

    [<GetRoute>]
    member this.GetMoney() : Money =
        { Amount = this.Money.Amount() }
