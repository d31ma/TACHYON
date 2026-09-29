open Models

type AuditService() =
    let mutable records = 0
    let mutable audited = 0L
    member val Settled = 0L with get, set

    [<OnEvent({"id":"audit-log","replay":true})>]
    [<Order(-10)>]
    member this.Record(order: PlacedOrder) : unit =
        records <- records + 1

    [<OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})>]
    [<Order(10)>]
    member this.Settle(order: PlacedOrder) : SettledOrder =
        { SettledOrder.Total = order.Total }

    [<OnEvent>]
    member this.SaveSettled(order: SettledOrder) : unit =
        this.Settled <- order.Total

    [<OnEvent>]
    [<InBackground>]
    member this.Audit(order: SettledOrder) : unit =
        audited <- order.Total
