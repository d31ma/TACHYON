open Models

[<Service>]
type CartService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Total : int64 = 0L with get, set

    member this.Add() : unit =
        this.Total <- this.Total + 1L
        applicationEventPublisher.PublishEvent({ Total = this.Total }, true)
