[<Service>]
type TallyService() =
    member val Count = 0 with get, set

    member this.Add(amount: int) : unit =
        this.Count <- this.Count + amount

    member this.Summary(label: string) : string =
        string this.Count + " " + label
