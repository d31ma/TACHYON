open Models

[<Service>]
type CounterService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Count = 0 with get, set

    member this.Add() : unit =
        this.Count <- this.Count + 1
        applicationEventPublisher.PublishEvent({ Count = this.Count })
