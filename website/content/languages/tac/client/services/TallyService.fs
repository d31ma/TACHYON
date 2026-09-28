[<Service>]
type TallyService() =
    member val Count = 6 with get, set

    member this.Runtime : string = "F#, compiled by .NET"

    member this.Doubled() : int =
        this.Count * 2
