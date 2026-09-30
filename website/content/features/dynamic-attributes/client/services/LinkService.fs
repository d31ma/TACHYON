open Models

[<Service>]
type LinkService() =
    member val Topic : Topic = { Slug = "routing"; Title = "Routing" } with get, set

    member val State : string = "ready" with get, set

    member val Message : string = "Everything is up to date." with get, set
