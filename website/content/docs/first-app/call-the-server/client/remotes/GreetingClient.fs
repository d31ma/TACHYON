open System.Threading.Tasks
open Models

[<HTTPClient({"path":"/api/greeting","adapter":"greeting","setup":"GreetingSetup"})>]
type GreetingClient =
    [<GetRequest>]
    abstract getGreeting: unit -> Task<Greeting>
