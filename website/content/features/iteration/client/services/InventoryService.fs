open System.Collections.Generic

[<Service>]
type InventoryService() =
    let product (name: string) : Dictionary<string, string> = Dictionary<string, string>(dict [ "name", name ])

    member val Products : obj = [ product "Anvil"; product "Rope" ] with get, set
