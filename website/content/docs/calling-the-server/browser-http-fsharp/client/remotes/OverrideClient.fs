open System.Threading.Tasks
open Models

[<HTTPClient("/api")>]
type OverrideClient =
    [<GetRequest("/never")>]
    abstract read: unit -> Task<Greeting>
