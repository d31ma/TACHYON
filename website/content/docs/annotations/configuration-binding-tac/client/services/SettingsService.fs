open Models

[<Service>]
type SettingsService() =
    let published = BehaviorJSON.Parse("{\"tac\":{\"catalog\":{\"title\":\"Catalog\",\"page\":{\"size\":20},\"cache\":{\"enabled\":true}}}}")
    let catalog = Behaviors.properties(typeof<CatalogSetup>, published)
    let values = Behaviors.values(typeof<PublicValues>, published)

    do
        use signal = new BehaviorSignal()
        use context = Behaviors.create(typeof<CatalogSetup>, "public-catalog", signal, published, profiles = [| "public" |])
        ()

    member val title : string = values.Get("title").Text with get, set
    member val pageSize : int64 = catalog.Get("pageSize").Integer with get, set
