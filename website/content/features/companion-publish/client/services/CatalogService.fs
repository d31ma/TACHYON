open Models

[<Service>]
type CatalogService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Count = 0 with get, set

    [<OnStart>]
    member this.Initialize() : unit =
        this.Announce()

    member private this.Announce() : unit =
        applicationEventPublisher.PublishEvent({ Count = 5 })

    [<OnEvent>]
    member this.Changed(value: CatalogCount) : unit =
        this.Count <- value.Count

    [<OnStop>]
    member this.Cleanup() : unit =
        this.Count <- 0
