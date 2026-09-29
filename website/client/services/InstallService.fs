[<Service>]
type InstallService() =
    let mutable system = "unix"

    member this.Pick(chosen: string) : unit =
        system <- chosen

    member this.Shows(candidate: string) : bool =
        candidate = system
