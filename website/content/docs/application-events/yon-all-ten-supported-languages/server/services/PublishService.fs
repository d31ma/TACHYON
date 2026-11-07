open Models

[<Service>]
type PublishService(applicationEventPublisher: ApplicationEventPublisher) =
    [<OnStart>]
    member this.Initialize() : unit =
        applicationEventPublisher.PublishEvent({ Value = 7L }) |> ignore
