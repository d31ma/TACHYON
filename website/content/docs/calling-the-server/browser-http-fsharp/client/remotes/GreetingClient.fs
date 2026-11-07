open System.Threading.Tasks
open Models

[<HTTPClient("/api")>]
[<CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})>]
type GreetingClient =
    [<Cached>]
    [<GetRequest("/greeting")>]
    abstract read: unit -> Task<Greeting>

    [<ClearCache({"method":"read"})>]
    [<PostRequest("/clear")>]
    abstract clear: unit -> Task<Greeting>
