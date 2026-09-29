open Models

[<Service>]
type AuditService() =
    member val Log : ResizeArray<string> = ResizeArray<string>() with get
    member val Settled = 0L with get, set

    [<OnEvent({"id":"audit-log"})>]
    [<Order(-10)>]
    member this.Record(order: PlacedOrder) : unit =
        this.Log.Add(order.Status)

    [<OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})>]
    [<Order(10)>]
    member this.Settle(order: PlacedOrder) : SettledOrder =
        { SettledOrder.Total = order.Total }

    [<OnEvent>]
    member this.SaveSettled(order: SettledOrder) : unit =
        this.Settled <- order.Total
