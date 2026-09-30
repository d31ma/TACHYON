open Models

[<Service>]
type MapService() =
    member val Latitude = 0.0 with get, set

    member val Longitude = 0.0 with get, set

    [<OnEvent>]
    member this.Show(value: PlaceFound) : unit =
        this.Latitude <- value.Latitude
        this.Longitude <- value.Longitude
