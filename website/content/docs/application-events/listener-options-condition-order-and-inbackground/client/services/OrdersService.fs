open Models

[<Service>]
type OrdersService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Log : string list = [] with get, set
    member val Settled = 0 with get, set
    member val Audited = 0L with get, set

    member this.Place(total: int64) : unit =
        applicationEventPublisher.PublishEvent({ Status = "paid"; Total = total })

    [<OnEvent({"id":"orders-log"})>]
    [<Order(-10)>]
    member this.Record(order: PlacedOrder) : unit =
        this.Log <- this.Log @ [ order.Status ]

    [<OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})>]
    member this.SettleLargeOrder(order: PlacedOrder) : SettledOrder =
        this.Settled <- this.Settled + 1
        { SettledOrder.Total = order.Total }

    [<OnEvent>]
    [<InBackground>]
    member this.Audit(order: SettledOrder) : unit =
        this.Audited <- order.Total
