open System.Collections.Generic

[<Service>]
type LinkService() =
    member val Topic : obj = Dictionary<string, string>(dict [ "slug", "routing"; "title", "Routing" ]) with get, set

    member val State : string = "ready" with get, set

    member val Message : string = "Everything is up to date." with get, set
