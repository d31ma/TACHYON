open System
open Models

[<RequestRoute("/settings")>]
[<Public>]
type SettingsController() =
    [<GetRoute>]
    member this.GetSettings() : Settings =
        let application = BehaviorJSON.Parse(Environment.GetEnvironmentVariable("YON_APPLICATION_JSON"))
        let catalog = Behaviors.properties(typeof<CatalogSetup>, application)
        let endpoint = Behaviors.values(typeof<Endpoint>, application)
        let profile = endpoint.Get("profile").Text
        use signal = new BehaviorSignal()
        use context = Behaviors.create(typeof<CatalogSetup>, "public-catalog", signal, application, profiles = [| profile |])
        { Profile = profile; PageSize = int (catalog.Get("pageSize").Integer) }
