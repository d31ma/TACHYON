open Models

[<Service>]
type ReleaseService(applicationEventPublisher: ApplicationEventPublisher) =
    member val Release = "" with get, set

    member this.Announce() : unit =
        applicationEventPublisher.PublishEvent({ Release = "TACHYON" }, true)

    [<OnEvent({"replay":true})>]
    member this.Receive(changed: ReleaseChanged) : unit =
        this.Release <- changed.Release
