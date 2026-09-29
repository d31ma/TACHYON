[<Service>]
type CatalogService(dataSource: DataSource) =
    member this.Find() : obj =
        box (dataSource.Query("SELECT 42 AS amount", [||]))
