[<Service>]
type ActivityService() =
    member val State : string = "foreground" with get, set

    [<OnEvent({"replay":true})>]
    member this.HearApplicationState(changed: ApplicationStateChanged) : unit =
        this.State <- changed.State
