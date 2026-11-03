open Models

[<Service>]
type InventoryService() =
    member val Records : InventoryRecord list = [] with get, set

    [<OnEvent>]
    member this.AddRecord(record: InventoryRecord) : unit =
        let kept = this.Records @ [ record ]
        this.Records <- List.skip (max 0 (kept.Length - 64)) kept
