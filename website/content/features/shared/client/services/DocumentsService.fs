[<Service>]
[<NeedsAll>]
type DocumentsService(order: string list) =
    member this.Guides() : int =
        List.length order
