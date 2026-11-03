open System.Threading.Tasks
open Models

[<Service>]
type CounterService() =
    [<Needs>]
    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set

    member val Count : int = 0 with get, set
    member val Message : string = "" with get, set

    member this.Increment() : unit =
        this.Count <- this.Count + 1

    member this.AskServer() : Task<Greeting> =
        task {
            let! greeting = this.GreetingClient.getGreeting()
            this.Message <- greeting.Message
            return greeting
        }
