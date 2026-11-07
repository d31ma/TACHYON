open Models

[<Service>]
type CheckoutService(applicationEventPublisher: ApplicationEventPublisher) =
    [<OnStart>]
    member this.Start() : unit =
        applicationEventPublisher.PublishEvent({ Status = "paid"; Total = 25L }) |> ignore

    [<OnStop>]
    member this.Stop() : unit = ()
