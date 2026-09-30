open Models

[<Service>]
type CatalogService() =
    member this.Products : Product list =
        [ { Id = "anvil"; Name = "Anvil"; Price = "£42.00" }
          { Id = "rope"; Name = "Rope"; Price = "£8.00" } ]
