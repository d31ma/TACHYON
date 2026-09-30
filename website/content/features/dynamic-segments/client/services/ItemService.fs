[<Service>]
type ItemService() =
    member this.IdFrom(path: string) : string =
        path.Substring(path.LastIndexOf('/') + 1)
