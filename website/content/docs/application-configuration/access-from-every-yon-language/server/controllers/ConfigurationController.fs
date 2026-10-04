open System
open Models

[<RequestRoute("/configuration")>]
[<Public>]
type ConfigurationController() =
    [<GetRoute>]
    member this.GetConfiguration() : Configuration =
        let application = BehaviorJSON.Parse(Environment.GetEnvironmentVariable("YON_APPLICATION_JSON"))
        let settings = Behaviors.properties(typeof<CatalogSettings>, application)
        let endpoint = Behaviors.values(typeof<Endpoint>, application)
        let pageSize = settings.Get("pageSize").Integer
        let port = endpoint.Get("port").Integer
        { PageSize = int pageSize; Port = int port }
