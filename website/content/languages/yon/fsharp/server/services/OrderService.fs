[<Service>]
type OrderService(orderStore: OrderStore) =
    member this.List() : string[] =
        orderStore.All()
