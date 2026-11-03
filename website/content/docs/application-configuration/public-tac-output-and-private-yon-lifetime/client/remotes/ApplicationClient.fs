open System.Threading.Tasks

[<HTTPClient>]
type ApplicationClient =
    [<GetRequest("/manifest.webmanifest")>]
    abstract Load: unit -> Task<BehaviorJSON>
