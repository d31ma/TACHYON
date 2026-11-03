[<Service>]
type StatusService() =
    member val Online : bool = false with get, set
    member val Scheme : string = "" with get, set

    [<OnEvent({"replay":true})>]
    member this.HearConnectivity(changed: ConnectivityChanged) : unit =
        this.Online <- changed.Online

    [<OnEvent({"replay":true})>]
    member this.HearPreferences(changed: PreferencesChanged) : unit =
        this.Scheme <- changed.Scheme
