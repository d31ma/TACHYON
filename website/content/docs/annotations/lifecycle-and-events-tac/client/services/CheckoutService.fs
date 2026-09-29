open Models

type CheckoutService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Placed = 0 with get, set

    [<OnStart>]
    member this.Start() : unit =
        this.Place()

    member this.Place() : unit =
        this.Placed <- this.Placed + 1
        applicationEventPublisher.PublishEvent({ Status = "paid"; Total = 25L }, true)

    [<OnStop>]
    member this.Stop() : unit =
        this.Placed <- 0
