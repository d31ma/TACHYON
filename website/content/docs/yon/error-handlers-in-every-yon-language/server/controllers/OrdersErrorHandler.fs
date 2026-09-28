open Models

[<ErrorHandler>]
type OrdersErrorHandler() =
    [<HandlesError>]
    [<ResponseStatus(404)>]
    member this.Missing(error: OrderNotFound) : Problem =
        { Code = "order-not-found" }
