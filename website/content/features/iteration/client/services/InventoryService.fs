open Models

[<Service>]
type InventoryService() =
    member val Products : Product list = [ { Name = "Anvil" }; { Name = "Rope" } ] with get, set
